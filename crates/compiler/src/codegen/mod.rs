//! The infrastructure for building `FLO` objects to support the additional
//! non-generic functionality not embedded in [`FlatLoweredObject`] itself.

pub mod output;

use hieratika_errors::compile::{Error, Result};
use hieratika_flo::FlatLoweredObject;

use crate::{codegen::output::CodegenOutput, context::SourceContext, pass::data::DynPassDataMap};

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

/// The builder functions themselves.
impl CodeGenerator {
    /// Executes the code generation process on a freshly created
    /// [`FlatLoweredObject`].
    ///
    /// # Errors
    ///
    /// - [`Error`], if the code generation process fails for any reason.
    pub fn run(&self) -> Result<FlatLoweredObject> {
        let _output = CodegenOutput::new(&self.name);

        Err(Error::CompilationFailure(
            "The compilation process has not yet been implemented".to_string(),
        ))
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
