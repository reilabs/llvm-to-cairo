//! This pass is responsible for generating a map of the top-level structure of
//! an LLVM IR module as described by an `.ll` file. This map encompasses both
//! function and global entries at the module level, as well as data layout
//! description for the module.
//!
//! The [`ModuleMap`] that results from this pass is intended for downstream
//! usage during the compilation step, primarily for consistency checking.

use std::collections::HashMap;

use ethnum::U256;
use hieratika_errors::compile::{Error, Result};
use inkwell::{
    module::{Linkage, Module},
    values::{FunctionValue, GlobalValue},
    GlobalVisibility,
};

use crate::{
    context::SourceContext,
    llvm::{
        data_layout::DataLayout,
        special_intrinsics::SpecialIntrinsics,
        typesystem::LLVMType,
        TopLevelEntryKind,
    },
    pass::{
        data::{ConcretePassData, DynPassDataMap, DynPassReturnData, PassDataOps},
        ConcretePass,
        Pass,
        PassKey,
        PassOps,
    },
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

/// Constructors that provide ways to create an instance of the
/// [`BuildModuleMap`] pass.
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

/// Functionality that the [`BuildModuleMap`] pass implements.
impl BuildModuleMap {
    /// Generates a module map for the provided module in the source context,
    /// returning the module map if successful.
    ///
    /// # Errors
    ///
    /// - [`Error`] if the module cannot be mapped successfully.
    pub fn map_module(&mut self, module: &Module) -> Result<ModuleMap> {
        // First, we grab the module name.
        let module_name = module.get_name().to_str()?;

        // We start by analyzing the data-layout of the module, which is important to
        // ensure that things match later on and that we are not being asked for things
        // that we do not or cannot support. This _may_ currently return errors due to
        // unsupported data layouts, but this could potentially be moved into the
        // compilation step in the future.
        let data_layout = self.process_data_layout(module.get_data_layout().as_str().to_str()?)?;

        // With our data layout obtained successfully, we can build our module map and
        // start adding top-level entries to it.
        let mut mod_map = ModuleMap::new(module_name, data_layout);

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
/// obligated to implement `PassOps` for it to make this possible.
impl PassOps for BuildModuleMap {
    fn run(
        &mut self,
        context: SourceContext,
        _pass_data: &DynPassDataMap,
    ) -> Result<DynPassReturnData> {
        let analysis_result = context.analyze_module(|module| self.map_module(module))?;
        Ok(DynPassReturnData::new(context, Box::new(analysis_result)))
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
/// - Name, useful for identifying the module in question.
/// - Data layout, as given by the embedded data layout string.
/// - Functions, as given by the function definitions and declarations.
/// - Globals, as given by the global definitions and declarations.
#[derive(Clone, Debug, PartialEq)]
pub struct ModuleMap {
    /// The name for the module.
    pub module_name: String,

    /// The data layout provided for this module.
    pub data_layout: DataLayout,

    /// The globals that are contained within the module.
    pub globals: HashMap<String, GlobalInfo>,

    /// The functions that are contained within the module.
    pub functions: HashMap<String, FunctionInfo>,
}

impl ModuleMap {
    /// Creates a new instance of the output data for the module mapping pass.
    ///
    /// # Anonymous Modules
    ///
    /// If the module is anonymous—in other words that its name is an empty
    /// string—it will have a name generated at random. Please note that the
    /// underlying RNG **cannot be relied upon to be cryptographically secure**,
    /// and should not be treated as such.
    #[must_use]
    pub fn new(name: &str, data_layout: DataLayout) -> Self {
        let module_name = if name.is_empty() {
            // This _is_ actually cryptographically secure, but the fact that it is (see the
            // docs on `ThreadRng` for more details), is an implementation detail and need
            // not be sustained through changes.
            let rand_bytes: [u8; size_of::<U256>()] = rand::random();
            let rand_num = U256::from_be_bytes(rand_bytes);
            format!("{rand_num:#032x}")
        } else {
            name.to_string()
        };
        let functions = HashMap::new();
        let globals = HashMap::new();

        Self {
            module_name,
            data_layout,
            globals,
            functions,
        }
    }

    /// Creates a new trait object of the output data for the module mapping
    /// pass.
    #[must_use]
    pub fn new_dyn(name: &str, data_layout: DataLayout) -> Box<Self> {
        Box::new(Self::new(name, data_layout))
    }
}

/// We need to work with this type as a generic piece of pass data.
impl PassDataOps for ModuleMap {}

/// We also need to work with this type as a piece of _concrete_ pass data for
/// non type-erased workflows.
impl ConcretePassData for ModuleMap {
    type Pass = BuildModuleMap;
}

/// The information necessary to describe the conventions and operations
/// necessary to call a function.
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

/// The information necessary to describe the conventions and operations
/// necessary to access a global.
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

#[cfg(test)]
mod test {
    use std::path::Path;

    use inkwell::{module::Linkage, GlobalVisibility};

    use crate::{
        context::SourceContext,
        llvm::{data_layout::DataLayout, typesystem::LLVMType, TopLevelEntryKind},
        pass::{
            analysis::module_map::{BuildModuleMap, ModuleMap},
            data::DynPassDataMap,
            ConcretePass,
            PassOps,
        },
    };

    /// A utility function to make it easy to load the testing context in all
    /// the tests.
    fn get_text_context() -> SourceContext {
        SourceContext::create(Path::new(r"input/add.ll"))
            .expect("Unable to construct testing source context")
    }

    #[test]
    fn generates_random_names_for_anon_modules() {
        let map_1 = ModuleMap::new("", DataLayout::new("").unwrap());
        let map_2 = ModuleMap::new("", DataLayout::new("").unwrap());

        assert_ne!(map_1.module_name, map_2.module_name);
    }

    #[test]
    fn returns_correct_data_type() -> anyhow::Result<()> {
        // Setup
        let ctx = get_text_context();
        let data = DynPassDataMap::new();
        let mut pass = BuildModuleMap::new_dyn();
        let dyn_return_data = pass.run(ctx, &data)?;

        // We should be able to get the pass data as the correct associated type.
        assert!(
            dyn_return_data
                .data
                .view_as::<<BuildModuleMap as ConcretePass>::Data>()
                .is_some()
        );

        Ok(())
    }

    #[test]
    fn discovers_correct_data_layout() -> anyhow::Result<()> {
        // Setup
        let ctx = get_text_context();
        let data = DynPassDataMap::new();
        let mut pass = BuildModuleMap::new_dyn();

        let dyn_return_data = pass.run(ctx, &data)?;
        let map = dyn_return_data
            .data
            .view_as::<<BuildModuleMap as ConcretePass>::Data>()
            .unwrap();

        // The data layout should have been picked up correctly from the module, and we
        // know that parsing works, so we check equality
        let data_layout = &map.data_layout;
        let expected_data_layout =
            DataLayout::new("e-m:e-i8:8:32-i16:16:32-i64:64-i128:128-n32:64-S128")?;
        assert_eq!(data_layout, &expected_data_layout);

        Ok(())
    }

    #[test]
    fn discovers_correct_globals() -> anyhow::Result<()> {
        let ctx = get_text_context();
        let data = DynPassDataMap::new();
        let mut pass = BuildModuleMap::new_dyn();

        let dyn_return_data = pass.run(ctx, &data)?;
        let map = dyn_return_data
            .data
            .view_as::<<BuildModuleMap as ConcretePass>::Data>()
            .unwrap();
        let globals = &map.globals;

        // Functions, though technically globals, should not be seen
        assert!(
            !globals.contains_key(
                &"_ZN19hieratika_rust_test_input3add17h828e50e9267cb510E".to_string()
            )
        );
        assert!(!globals.contains_key(&"llvm.dbg.declare".to_string()));
        assert!(!globals.contains_key(&"llvm.uadd.with.overflow.i64".to_string()));
        assert!(
            !globals.contains_key(
                &"_ZN4core9panicking11panic_const24panic_const_add_overflow17he7771b1d81fa091aE"
                    .to_string()
            )
        );

        // The first global
        assert!(globals.contains_key(&"alloc_4190527422e5cc48a15bd1cb4f38f425".to_string()));
        let global_1 = globals
            .get(&"alloc_4190527422e5cc48a15bd1cb4f38f425".to_string())
            .unwrap();
        assert!(global_1.is_initialized);
        assert_eq!(global_1.visibility, GlobalVisibility::Default);
        assert_eq!(global_1.alignment, 1);
        assert!(global_1.is_const);
        assert_eq!(global_1.linkage, Linkage::Private);
        assert_eq!(global_1.kind, TopLevelEntryKind::Definition);
        assert_eq!(
            global_1.typ,
            LLVMType::make_struct(true, &[LLVMType::make_array(33, LLVMType::i8)])
        );

        // The second global
        assert!(globals.contains_key(&"alloc_5b4544c775a23c08ca70c48dd7be27fc".to_string()));
        let global_2 = globals
            .get(&"alloc_5b4544c775a23c08ca70c48dd7be27fc".to_string())
            .unwrap();
        assert!(global_2.is_initialized);
        assert_eq!(global_2.visibility, GlobalVisibility::Default);
        assert_eq!(global_2.alignment, 8);
        assert!(global_2.is_const);
        assert_eq!(global_2.linkage, Linkage::Private);
        assert_eq!(global_2.kind, TopLevelEntryKind::Definition);
        assert_eq!(
            global_2.typ,
            LLVMType::make_struct(
                true,
                &[LLVMType::ptr, LLVMType::make_array(16, LLVMType::i8)]
            )
        );

        Ok(())
    }

    #[test]
    fn discovers_correct_functions() -> anyhow::Result<()> {
        let ctx = get_text_context();
        let data = DynPassDataMap::new();
        let mut pass = BuildModuleMap::new_dyn();

        let dyn_return_data = pass.run(ctx, &data)?;
        let map = dyn_return_data
            .data
            .view_as::<<BuildModuleMap as ConcretePass>::Data>()
            .unwrap();
        let functions = &map.functions;

        // First we check that the globals have avoided somehow being recorded as
        // functions.
        assert!(!functions.contains_key(&"alloc_4190527422e5cc48a15bd1cb4f38f425".to_string()));
        assert!(!functions.contains_key(&"alloc_5b4544c775a23c08ca70c48dd7be27fc".to_string()));

        // _ZN19hieratika_rust_test_input3add17h828e50e9267cb510E
        let rust_test_input = functions
            .get(&"_ZN19hieratika_rust_test_input3add17h828e50e9267cb510E".to_string())
            .unwrap();
        assert!(!rust_test_input.intrinsic);
        assert_eq!(rust_test_input.kind, TopLevelEntryKind::Definition);
        assert_eq!(rust_test_input.linkage, Linkage::External);
        assert_eq!(rust_test_input.visibility, GlobalVisibility::Default);
        assert_eq!(
            rust_test_input.typ,
            LLVMType::make_function(LLVMType::i64, &[LLVMType::i64, LLVMType::i64])
        );

        // llvm.dbg.declare
        let rust_test_input = functions.get(&"llvm.dbg.declare".to_string()).unwrap();
        assert!(rust_test_input.intrinsic);
        assert_eq!(rust_test_input.kind, TopLevelEntryKind::Declaration);
        assert_eq!(rust_test_input.linkage, Linkage::External);
        assert_eq!(rust_test_input.visibility, GlobalVisibility::Default);
        assert_eq!(
            rust_test_input.typ,
            LLVMType::make_function(
                LLVMType::void,
                &[LLVMType::Metadata, LLVMType::Metadata, LLVMType::Metadata]
            )
        );

        // llvm.uadd.with.overflow.i64
        let rust_test_input = functions.get(&"llvm.uadd.with.overflow.i64".to_string()).unwrap();
        assert!(rust_test_input.intrinsic);
        assert_eq!(rust_test_input.kind, TopLevelEntryKind::Declaration);
        assert_eq!(rust_test_input.linkage, Linkage::External);
        assert_eq!(rust_test_input.visibility, GlobalVisibility::Default);
        assert_eq!(
            rust_test_input.typ,
            LLVMType::make_function(
                LLVMType::make_struct(false, &[LLVMType::i64, LLVMType::bool]),
                &[LLVMType::i64, LLVMType::i64]
            )
        );

        // _ZN4core9panicking11panic_const24panic_const_add_overflow17he7771b1d81fa091aE
        let rust_test_input = functions
            .get(
                &"_ZN4core9panicking11panic_const24panic_const_add_overflow17he7771b1d81fa091aE"
                    .to_string(),
            )
            .unwrap();
        assert!(!rust_test_input.intrinsic);
        assert_eq!(rust_test_input.kind, TopLevelEntryKind::Declaration);
        assert_eq!(rust_test_input.linkage, Linkage::External);
        assert_eq!(rust_test_input.visibility, GlobalVisibility::Default);
        assert_eq!(
            rust_test_input.typ,
            LLVMType::make_function(LLVMType::void, &[LLVMType::ptr])
        );

        Ok(())
    }
}
