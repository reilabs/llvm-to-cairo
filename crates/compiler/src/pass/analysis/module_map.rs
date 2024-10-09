//! This pass is responsible for generating a map of the top-level structure of
//! an LLVM IR module as described by an `.ll` file. This is used for
//! consistency checking during the compilation step.

use std::collections::HashMap;

use inkwell::{
    module::{Linkage, Module},
    values::{FunctionValue, GlobalValue},
};
use ltc_errors::compile::Result;

use crate::{
    llvm::typesystem::LLVMType,
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
#[derive(Clone, Debug, PartialEq)]
pub struct BuildModuleMap {
    /// The passes that this pass depends upon the results of for its execution.
    depends: Vec<PassKey>,

    /// The passes that this pass invalidates the results of by executing.
    invalidates: Vec<PassKey>,

    /// LLVM intrinsics that need to be handled specially.
    special_intrinsics: SpecialIntrinsics,
}

impl BuildModuleMap {
    /// Creates a new instance of the module mapping pass.
    pub fn new() -> Self {
        let depends = vec![];
        let invalidates = vec![];
        let special_intrinsics = SpecialIntrinsics::new();
        Self {
            depends,
            invalidates,
            special_intrinsics,
        }
    }

    /// Creates a new trait object of the module mapping pass.
    pub fn new_dyn() -> Box<Self> {
        Box::new(Self::new())
    }
}

impl BuildModuleMap {
    /// Generates a module map for the provided module in the source context,
    /// returning the module map.
    pub fn map_module(&mut self, module: &Module) -> Result<ModuleMap> {
        let mut mod_map = ModuleMap::new();

        // We start by analyzing the data-layout of the module, which is important to
        // ensure that things match later on.

        // TODO datalayout checks
        let data_layout = module.get_data_layout();
        dbg!(&data_layout);

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

        // Our map is done, so we can just return it!
        Ok(mod_map)
    }

    /// Gathers the data for a global at the level of the module, and writes it
    /// into `mod_map` for later usage.
    pub fn map_global(&mut self, _global: &GlobalValue, _mod_map: &mut ModuleMap) -> Result<()> {
        unimplemented!()
    }

    /// Gathers the data for a function at the level of the module, and
    /// writes it into `mod_map` for later usage.
    pub fn map_function(&mut self, func: &FunctionValue, mod_map: &mut ModuleMap) -> Result<()> {
        let name = func.get_name().to_str()?.to_string();

        if let Some(intrinsic) = self.special_intrinsics.info_for(&name) {
            mod_map.functions.insert(name, intrinsic);
        } else {
            let kind = if func.as_global_value().is_declaration() {
                FunctionKind::Declaration
            } else {
                FunctionKind::Definition
            };
            let typ = LLVMType::try_from(func.get_type())?;
            let linkage = func.get_linkage();
            let intrinsic = func.get_intrinsic_id() != 0;
            let f_info = FunctionInfo {
                kind,
                intrinsic,
                typ,
                linkage,
            };

            mod_map.functions.insert(name, f_info);
        }

        Ok(())
    }
}

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

impl ConcretePass for BuildModuleMap {
    type Data = ModuleMap;
}

/// The module map that results from executing this analysis pass on an LLVM IR
/// module.
#[derive(Clone, Debug, PartialEq)]
pub struct ModuleMap {
    /// The functions that are contained within the module.
    ///
    /// TODO do these include decls?
    pub functions: HashMap<String, FunctionInfo>,
}

impl ModuleMap {
    /// Creates a new instance of the output data for the module mapping pass.
    pub fn new() -> Self {
        let functions = HashMap::new();
        Self { functions }
    }

    /// Creates a new trait object of the output data for the module mapping
    /// pass.
    pub fn new_dyn() -> Box<Self> {
        Box::new(Self::new())
    }
}

impl PassDataOps for ModuleMap {}
impl ConcretePassData for ModuleMap {
    type Pass = BuildModuleMap;
}

/// The type of function that is encountered in the module.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum FunctionKind {
    /// A declaration of an external symbol including the function name,
    /// attributes, and signature.
    Declaration,

    /// A definition of the function including the name, attributes, signature,
    /// and **body** (consisting of basic blocks).
    Definition,
}

/// Information about a function stored in the module map.
#[derive(Clone, Debug, PartialEq)]
pub struct FunctionInfo {
    /// The type of function entity that was encountered here.
    pub kind: FunctionKind,

    /// Set if this function is an LLVM intrinsic, and unset otherwise.
    pub intrinsic: bool,

    /// The LLVM type of our function.
    pub typ: LLVMType,

    /// The linkage for our function.
    pub linkage: Linkage,
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
    /// Constructs
    pub fn new() -> Self {
        let mut intrinsics = HashMap::new();
        intrinsics.insert(
            "llvm.dbg.declare".to_string(),
            FunctionInfo {
                kind:      FunctionKind::Declaration,
                intrinsic: true,
                typ:       LLVMType::make_function(
                    LLVMType::void,
                    &[LLVMType::Metadata, LLVMType::Metadata, LLVMType::Metadata],
                ),
                linkage:   Linkage::External,
            },
        );
        intrinsics.insert(
            "llvm.dbg.value".to_string(),
            FunctionInfo {
                kind:      FunctionKind::Declaration,
                intrinsic: true,
                typ:       LLVMType::make_function(
                    LLVMType::void,
                    &[LLVMType::Metadata, LLVMType::Metadata, LLVMType::Metadata],
                ),
                linkage:   Linkage::External,
            },
        );
        intrinsics.insert(
            "llvm.dbg.assign".to_string(),
            FunctionInfo {
                kind:      FunctionKind::Declaration,
                intrinsic: true,
                typ:       LLVMType::make_function(
                    LLVMType::void,
                    &[
                        LLVMType::Metadata,
                        LLVMType::Metadata,
                        LLVMType::Metadata,
                        LLVMType::Metadata,
                        LLVMType::Metadata,
                    ],
                ),
                linkage:   Linkage::External,
            },
        );

        Self { intrinsics }
    }

    /// Gets the function information for `function_name` if it exists, and
    /// returns [`None`] otherwise.
    pub fn info_for(&self, function_name: &str) -> Option<FunctionInfo> {
        self.intrinsics.get(&function_name.to_string()).cloned()
    }

    /// Gets the function information for `function_name` if it exists.
    ///
    /// # Panics
    ///
    /// If `function_name` does not exist in the special intrinsics container.
    pub fn info_for_unchecked(&self, function_name: &str) -> FunctionInfo {
        self.info_for(function_name)
            .expect(&format!("No information found for {function_name}"))
    }
}

impl Default for SpecialIntrinsics {
    fn default() -> Self {
        Self::new()
    }
}
