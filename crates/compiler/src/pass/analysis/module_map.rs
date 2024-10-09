//! This pass is responsible for generating a map of the top-level structure of
//! an LLVM IR module as described by an `.ll` file. This map encompasses both
//! function and global entries at the module level, as well as data layout
//! description for the module.
//!
//! The [`ModuleMap`] that results from this pass is intended for downstream
//! usage during the compilation step, primarily for consistency checking.

use std::collections::HashMap;

use inkwell::{
    module::{Linkage, Module},
    values::{FunctionValue, GlobalValue},
    GlobalVisibility,
};
use ltc_errors::compile::{Error, Result};

use crate::{
    llvm::{data_layout::DataLayout, typesystem::LLVMType, TopLevelEntryKind},
    pass::{
        data::{ConcretePassData, DynPassDataMap, PassDataOps},
        ConcretePass,
        DynamicPassReturnData,
        Pass,
        PassKey,
        PassOps,
    },
    source::SourceContext,
};

/// Generates a map of the top-level structure of an LLVM module.
///
/// This map includes both functions and globals, as well as the [`DataLayout`]
/// definition for the module.
#[derive(Clone, Debug, PartialEq)]
pub struct BuildModuleMap {
    /// The passes that this pass depends upon the results of for its execution.
    depends: Vec<PassKey>,

    /// The passes that this pass invalidates the results of by executing.
    invalidates: Vec<PassKey>,

    /// LLVM intrinsics that need to be handled specially.
    special_intrinsics: SpecialIntrinsics,
}

impl Default for BuildModuleMap {
    fn default() -> Self {
        Self::new()
    }
}

impl BuildModuleMap {
    /// Creates a new instance of the module mapping pass.
    #[must_use]
    pub fn new() -> Self {
        // This pass depends on the results of no other passes.
        let depends = vec![];

        // This pass's operation is purely analytical and hence it does not invalidate
        // any other passes.
        let invalidates = vec![];

        let special_intrinsics = SpecialIntrinsics::new();
        Self {
            depends,
            invalidates,
            special_intrinsics,
        }
    }

    /// Creates a new trait object of the module mapping pass.
    #[must_use]
    pub fn new_dyn() -> Box<Self> {
        Box::new(Self::new())
    }
}

impl BuildModuleMap {
    /// Generates a module map for the provided module in the source context,
    /// returning the module map if successful.
    ///
    /// # Errors
    ///
    /// - [`Error`] if the module cannot be mapped successfully.
    pub fn map_module(&mut self, module: &Module) -> Result<ModuleMap> {
        // We start by analyzing the data-layout of the module, which is important to
        // ensure that things match later on and that we are not being asked for things
        // that we do not or cannot support. This _may_ currently return errors due to
        // unsupported data layouts, but this could potentially be moved into the
        // compilation step in the future.
        let data_layout = self.process_data_layout(module.get_data_layout().as_str().to_str()?)?;

        // With our data layout obtained succesffully, we can build our module map and
        // start adding top-level entries to it.
        let mut mod_map = ModuleMap::new(data_layout);

        // We then process the global definitions in scope and gather the relevant
        // information about them.
        module
            .get_globals()
            .map(|g| self.map_global(&g, &mut mod_map))
            .collect::<Result<Vec<()>>>()?;

        // Finally we use the top-level information about functions to create a map of
        // the remaining symbols that occur in the module.
        module
            .get_functions()
            .map(|f| self.map_function(&f, &mut mod_map))
            .collect::<Result<Vec<()>>>()?;

        // Our map is complete, so we can just return it.
        Ok(mod_map)
    }

    /// Processes the data layout declaration from the module.
    ///
    /// # Future-Gazing
    ///
    /// In the future we may well want to treat our target as a proper Harvard
    /// architecture with the separate program address space and allocation
    /// address space that it actually has. For now, we are relying on a stopgap
    /// target (`aarch64-unknown-none-softfloat`) which does not give us this
    /// control, and so we are raising an error if the configuration is
    /// incorrect.
    ///
    /// # Errors
    ///
    /// - [`Error::UnsupportedAdditionalAddressSpaces`] if the data layout
    ///   declares pointers in any address space other than the default 0.
    /// - [`Error::UnsupportedNonIntegralPointerConfiguration`] if the data
    ///   layout requests non-integral pointers for any address space.
    pub fn process_data_layout(&mut self, layout_string: &str) -> Result<DataLayout> {
        // We start by analyzing the data-layout of the module, which is important to
        // ensure that things match later on and that we are not being asked for things
        // that we do not or cannot support.
        let data_layout = DataLayout::new(layout_string)?;

        // We do not support split address spaces (for now). Later we may want to use
        // this to properly state that our target architecture is a Harvard one rather
        // than a Von-Neumann one, but we are leaving it for now.
        if data_layout.pointer_layouts.iter().any(|p| p.address_space != 0)
            || data_layout.alloc_address_space != 0
            || data_layout.global_address_space != 0
            || data_layout.program_address_space != 0
        {
            Err(Error::UnsupportedAdditionalAddressSpaces)?;
        }

        // We do not support non-integral pointers in any address space.
        if !data_layout.nointptr_address_spaces.address_spaces.is_empty() {
            Err(Error::UnsupportedNonIntegralPointerConfiguration)?;
        }

        Ok(data_layout)
    }

    /// Gathers the data for a module-level global and writes it into the
    /// `mod_map`.
    ///
    /// # Errors
    ///
    /// - [`Error`] if the global information cannot be gathered successfully.
    pub fn map_global(&mut self, global: &GlobalValue, mod_map: &mut ModuleMap) -> Result<()> {
        let name = global.get_name().to_str()?.to_string();

        let kind = if global.is_declaration() {
            TopLevelEntryKind::Declaration
        } else {
            TopLevelEntryKind::Definition
        };

        let typ = global.get_value_type().try_into()?;
        let is_const = global.is_constant();
        let alignment = global.get_alignment() as usize;
        let linkage = global.get_linkage();
        let visibility = global.get_visibility();
        let is_initialized = global.get_initializer().is_some();

        let global_info = GlobalInfo {
            kind,
            typ,
            linkage,
            visibility,
            alignment,
            is_const,
            is_initialized,
        };

        mod_map.globals.insert(name, global_info);

        Ok(())
    }

    /// Gathers the data for a module-level function and writes it into the
    /// `mod_map`.
    ///
    /// # Errors
    ///
    /// - [`Error`] if the function information cannot be gathered successfully.
    pub fn map_function(&mut self, func: &FunctionValue, mod_map: &mut ModuleMap) -> Result<()> {
        let name = func.get_name().to_str()?.to_string();

        if let Some(intrinsic) = self.special_intrinsics.info_for(&name) {
            mod_map.functions.insert(name, intrinsic);
        } else {
            let kind = if func.as_global_value().is_declaration() {
                TopLevelEntryKind::Declaration
            } else {
                TopLevelEntryKind::Definition
            };
            let typ = LLVMType::try_from(func.get_type())?;
            let linkage = func.get_linkage();
            let intrinsic = func.get_intrinsic_id() != 0;
            let visibility = func.as_global_value().get_visibility();
            let f_info = FunctionInfo {
                kind,
                intrinsic,
                typ,
                linkage,
                visibility,
            };

            mod_map.functions.insert(name, f_info);
        }

        Ok(())
    }
}

/// We need to be able to run this pass using the pass manager, so we are
/// obliged to implement `PassOps` for it to make this possible.
impl PassOps for BuildModuleMap {
    fn run(
        &mut self,
        context: SourceContext,
        _pass_data: &DynPassDataMap,
    ) -> Result<DynamicPassReturnData> {
        let analysis_result = context.analyze_module(|module| self.map_module(module))?;
        Ok(DynamicPassReturnData::new(
            context,
            Box::new(analysis_result),
        ))
    }

    fn depends(&self) -> &[PassKey] {
        self.depends.as_slice()
    }

    fn invalidates(&self) -> &[PassKey] {
        self.invalidates.as_slice()
    }

    fn dupe(&self) -> Pass {
        Box::new(self.clone())
    }
}

/// We also want to be able to work with the pass when it is not type-erased to
/// `dyn PassOps`, so we are obliged to implement the concrete pass operations
/// trait here too.
impl ConcretePass for BuildModuleMap {
    type Data = ModuleMap;
}

/// The module map that results from executing this analysis pass on an LLVM IR
/// module.
///
/// It contains information on the module's:
///
/// - Data layout, as given by the embedded data layout string.
/// - Functions, as given by the function definitions and declarations.
/// - Globals, as given by the global definitions and declarations.
#[derive(Clone, Debug, PartialEq)]
pub struct ModuleMap {
    /// The data layout provided for this module.
    pub data_layout: DataLayout,

    /// The globals that are contained within the module.
    pub globals: HashMap<String, GlobalInfo>,

    /// The functions that are contained within the module.
    pub functions: HashMap<String, FunctionInfo>,
}

impl ModuleMap {
    /// Creates a new instance of the output data for the module mapping pass.
    #[must_use]
    pub fn new(data_layout: DataLayout) -> Self {
        let functions = HashMap::new();
        let globals = HashMap::new();
        Self {
            data_layout,
            globals,
            functions,
        }
    }

    /// Creates a new trait object of the output data for the module mapping
    /// pass.
    #[must_use]
    pub fn new_dyn(data_layout: DataLayout) -> Box<Self> {
        Box::new(Self::new(data_layout))
    }
}

/// We need to work with this type as a generic piece of pass data.
impl PassDataOps for ModuleMap {}

/// We also need to work with this type as a piece of _concrete_ pass data for
/// non type-erased workflows.
impl ConcretePassData for ModuleMap {
    type Pass = BuildModuleMap;
}

/// Information about a function to be stored in the module map.
#[derive(Clone, Debug, PartialEq)]
pub struct FunctionInfo {
    /// The type of function entity that was encountered here.
    pub kind: TopLevelEntryKind,

    /// Set if this function is an LLVM intrinsic, and unset otherwise.
    pub intrinsic: bool,

    /// The LLVM type of our function.
    pub typ: LLVMType,

    /// The linkage for our function.
    pub linkage: Linkage,

    /// The visibility of our function.
    pub visibility: GlobalVisibility,
}

/// Information about a global to be stored in the module map.
#[derive(Clone, Debug, PartialEq)]
pub struct GlobalInfo {
    /// The type of global entity that was encountered here.
    pub kind: TopLevelEntryKind,

    /// The LLVM type of our global.
    pub typ: LLVMType,

    /// The linkage for our global.
    pub linkage: Linkage,

    /// The visibility of our global.
    pub visibility: GlobalVisibility,

    /// The alignment of the global value.
    pub alignment: usize,

    /// `true` if this global is constant, and `false` otherwise.
    pub is_const: bool,

    /// `true` if this global is initialized, and `false` otherwise.
    pub is_initialized: bool,
}

/// A registry of LLVM intrinsic functions that need to be handled specially.
///
/// # Avoiding an Issue in Inkwell
///
/// Unfortunately [`inkwell`] does not deal well with `metadata`-typed function
/// arguments, despite them being valid argument types for function-typed values
/// in LLVM IR. For now, we handle them by delegating to known signatures for
/// these functions, rather than trying to introspect the functions themselves.
///
/// See [this issue](https://github.com/TheDan64/inkwell/issues/546) for more
/// information.
#[derive(Clone, Debug, PartialEq)]
pub struct SpecialIntrinsics {
    /// The intrinsics that need to be handled specially.
    intrinsics: HashMap<String, FunctionInfo>,
}

impl SpecialIntrinsics {
    /// Constructs the special intrinsics mapping, providing the appropriate
    /// [`FunctionInfo`] metadata for the intrinsics that we insert.
    #[must_use]
    pub fn new() -> Self {
        let mut intrinsics = HashMap::new();
        intrinsics.insert(
            "llvm.dbg.declare".to_string(),
            FunctionInfo {
                kind:       TopLevelEntryKind::Declaration,
                intrinsic:  true,
                typ:        LLVMType::make_function(
                    LLVMType::void,
                    &[LLVMType::Metadata, LLVMType::Metadata, LLVMType::Metadata],
                ),
                linkage:    Linkage::External,
                visibility: GlobalVisibility::Default,
            },
        );
        intrinsics.insert(
            "llvm.dbg.value".to_string(),
            FunctionInfo {
                kind:       TopLevelEntryKind::Declaration,
                intrinsic:  true,
                typ:        LLVMType::make_function(
                    LLVMType::void,
                    &[LLVMType::Metadata, LLVMType::Metadata, LLVMType::Metadata],
                ),
                linkage:    Linkage::External,
                visibility: GlobalVisibility::Default,
            },
        );
        intrinsics.insert(
            "llvm.dbg.assign".to_string(),
            FunctionInfo {
                kind:       TopLevelEntryKind::Declaration,
                intrinsic:  true,
                typ:        LLVMType::make_function(
                    LLVMType::void,
                    &[
                        LLVMType::Metadata,
                        LLVMType::Metadata,
                        LLVMType::Metadata,
                        LLVMType::Metadata,
                        LLVMType::Metadata,
                    ],
                ),
                linkage:    Linkage::External,
                visibility: GlobalVisibility::Default,
            },
        );

        Self { intrinsics }
    }

    /// Gets the function information for `function_name` if it exists, and
    /// returns [`None`] otherwise.
    #[must_use]
    pub fn info_for(&self, function_name: &str) -> Option<FunctionInfo> {
        self.intrinsics.get(function_name).cloned()
    }

    /// Gets the function information for `function_name` if it exists.
    ///
    /// # Panics
    ///
    /// If `function_name` does not exist in the special intrinsics container.
    #[must_use]
    pub fn info_for_unchecked(&self, function_name: &str) -> FunctionInfo {
        self.info_for(function_name)
            .unwrap_or_else(|| panic!("No information found for {function_name}"))
    }
}

impl Default for SpecialIntrinsics {
    fn default() -> Self {
        Self::new()
    }
}
