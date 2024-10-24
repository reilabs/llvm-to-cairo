//! The infrastructure for building `FLO` objects to support the additional
//! non-generic functionality not embedded in [`FlatLoweredObject`] itself.

pub mod data;

use hieratika_errors::compile::{Error, Result};
use hieratika_flo::FlatLoweredObject;
use inkwell::{
    module::Module,
    values::{FunctionValue, GlobalValue},
};

use crate::{
    codegen::data::CodegenData,
    context::SourceContext,
    llvm::TopLevelEntryKind,
    pass::{
        analysis::module_map::{BuildModuleMap, FunctionInfo, GlobalInfo},
        data::DynPassDataMap,
    },
};

/// Handles the minutiae of building a [`FlatLoweredObject`], as well as
/// tracking all the additional metadata required to properly construct that
/// object.
///
/// Please note that ensuring the validity of the object is up to the
/// **consumer** of this API. It is intended to be allowed to be left in invalid
/// states to aid construction, but will be checked for validity at completion
/// of the construction process. See the documentation for [`FlatLoweredObject`]
/// for more detail.
#[derive(Debug)]
pub struct CodeGenerator {
    /// The name of the module being compiled.
    name: String,

    /// The result data from all the analysis passes that have been run at the
    /// stage of building.
    ///
    /// It is intended to never be mutated during the building process.
    pass_data: DynPassDataMap,

    /// The source LLVM context that serves as the source of data from which
    /// compilation occurs.
    source_context: SourceContext,
}

/// Basic operations for construction and accessing fields.
impl CodeGenerator {
    /// Constructs a new code generator instance for the module with name
    /// `module_name`, as well as the provided `pass_data` and `source_context`.
    ///
    /// # Errors
    ///
    /// - [`Error::MissingModuleName`] if the provided `module_name` is empty as
    ///   it should already have been populated even if otherwise empty by the
    ///   [`crate::pass::analysis::module_map::BuildModuleMap`] pass.
    pub fn new(
        module_name: &str,
        pass_data: DynPassDataMap,
        source_context: SourceContext,
    ) -> Result<Self> {
        let name = if !module_name.is_empty() {
            module_name.to_string()
        } else {
            Err(Error::MissingModuleName)?
        };

        Ok(Self {
            name,
            pass_data,
            source_context,
        })
    }

    /// Gets the name of the module being built.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Gets an immutable reference to the pass data that the builder has been
    /// provided with.
    pub fn pass_data(&self) -> &DynPassDataMap {
        &self.pass_data
    }

    /// Gets an immutable reference to the LLVM context serving as the source of
    /// inputs to the code generation process.
    pub fn context(&self) -> &SourceContext {
        &self.source_context
    }
}

/// The functionality that actually performs code generation.
impl CodeGenerator {
    /// Executes the code generation process on a freshly created
    /// [`FlatLoweredObject`].
    ///
    /// # Errors
    ///
    /// - [`Error`], if the code generation process fails for any reason.
    pub fn run(&self) -> Result<FlatLoweredObject> {
        let mut cg_data = CodegenData::new(&self.name);

        self.context()
            .analyze_module(|m| self.generate_module(m, &mut cg_data))?;

        Ok(cg_data.into())
    }

    // TODO Operate as follows:
    //
    // 1. Iterate over _definitions_ of functions and for each:
    //    1. Stub out a new "initial block" with that function signature.
    //    2. Go through the basic blocks and generate based on them.
    // 2. Iterate over _definitions_ of globals and register them in the compilation
    //    context.
    //    1. If they are non-const it is an error.
    //    2. Take their definitions/initializers and register these.
    // 3. Every symbol in the `ModuleMap` that is a declaration rather than a
    //    definition should be declared as such in the symbol table.
    // 4. If a definition is encountered before being seen, it should then be
    //    registered in a lookup table (aux data in CodegenOutput).

    /// Performs the code generation process for the entire `module` currently
    /// being processed, generating equivalent object code into the provided
    /// `data` output.
    pub fn generate_module(&self, module: &Module, data: &mut CodegenData) -> Result<()> {
        // We need the module map to be able to make correct code generation decisions
        // here, so we start by grabbing this. If it doesn't exist, this is a programmer
        // error, so we crash loudly.
        let module_map = self.pass_data().get::<BuildModuleMap>().expect(
            "The module mapping pass does not appear to have been run but is required for code \
             generation.",
        );

        // We start by going through our functions, as these are the things we actually
        // need to generate code for.
        for function in module.get_functions() {
            // We can only generate code for a function if it actually is _defined_.
            // Otherwise, we just have to skip it; declarations are used for sanity checks
            // but cannot result in generated code.
            if let Some(f) = module_map.functions.get(function.get_name().to_str()?) {
                if matches!(f.kind, TopLevelEntryKind::Definition) {
                    self.generate_function(function, f, data)?
                }
            }
        }

        // We also potentially need to generate code for a global if it is initialized.
        for global in module.get_globals() {
            if let Some(g) = module_map.globals.get(global.get_name().to_str()?) {
                if matches!(g.kind, TopLevelEntryKind::Definition) {
                    self.generate_global(global, g, data)?
                }
            }
        }

        // Having generated both of these portions into the FLO, we are done for now.
        Ok(())
    }

    /// Generates code for the provided `func`, described by `func_info`.
    ///
    /// Behavior will not be well-formed if `func_info` is not the function
    /// information that corresponds to `func` at runtime.
    pub fn generate_function(
        &self,
        _func: FunctionValue,
        _func_info: &FunctionInfo,
        _data: &mut CodegenData,
    ) -> Result<()> {
        println!("FUNCTION");
        Ok(())
    }

    pub fn generate_global(
        &self,
        _global: GlobalValue,
        _global_info: &GlobalInfo,
        _data: &mut CodegenData,
    ) -> Result<()> {
        println!("GLOBAL");
        Ok(())
    }

    pub fn generate_statement(&self, _data: &mut CodegenData) -> Result<()> {
        Ok(())
    }

    pub fn generate_expression(&self, _data: &mut CodegenData) -> Result<()> {
        Ok(())
    }
}

/// Builder functionality that is not part of the stateful process of code
/// generation.
impl CodeGenerator {
    pub fn make_if(_output: &mut CodegenData) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::path::Path;

    use crate::{codegen::CodeGenerator, context::SourceContext, pass::data::DynPassDataMap};

    #[test]
    fn errors_on_invalid_name() -> anyhow::Result<()> {
        let source_module = Path::new("input/add.ll");
        let cg = CodeGenerator::new(
            "",
            DynPassDataMap::new(),
            SourceContext::create(source_module)?,
        );

        assert!(cg.is_err());

        Ok(())
    }
}
