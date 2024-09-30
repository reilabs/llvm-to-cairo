//! Contains the source compilation context, which is a way of tracking the
//! compilation units being processed by the compiler.

use inkwell::{context::Context as LLVMContext, module::Module};
use ltc_errors::compile::{Error, Result};
use ouroboros::self_referencing;

pub mod module;

use module::SourceModule;

/// The source compilation context manages the LLVM state across compiler
/// operations.
///
/// It is intended to exist only throughout the compilation process, after which
/// it may be safely discarded.
///
/// # Self-Reference
///
/// We use the [`ouroboros`] crate to create a struct that can contain a field
/// and references to that field at once. These references do not leak into the
/// crate boundary.
///
/// Do note that _this requires unsafe code_, but that said unsafe code is
/// encapsulated within the library.
#[self_referencing]
#[derive(Debug)]
pub struct SourceContext {
    /// The underlying context that contains the LLVM representation of the
    /// input IR.
    llvm_context: LLVMContext,

    /// The modules that have been added to the LLVM context. These are the
    /// objects that are directly compiled here.
    #[borrows(llvm_context)]
    #[not_covariant]
    modules: Vec<Module<'this>>,
}

impl SourceContext {
    /// Creates a new, empty, source compilation context.
    #[must_use]
    pub fn create() -> Self {
        let llvm_context = LLVMContext::create();

        SourceContextBuilder {
            llvm_context,
            modules_builder: |_| Vec::new(),
        }
        .build()
    }

    /// Adds the provided `module` to the compilation context.
    ///
    /// # Errors
    ///
    /// - [`Error::UnableToAddModuleToContext`] if the module cannot be added to
    ///   the context for some reason.
    pub fn add_module(
        &mut self,
        module: impl TryInto<SourceModule, Error = impl ToString>,
    ) -> Result<()> {
        let module_source = module
            .try_into()
            .map_err(|e| Error::UnableToAddModuleToContext(e.to_string()))?;

        self.with_mut(|all_fields| -> Result<()> {
            let ctx = &all_fields.llvm_context;
            let module = ctx
                .create_module_from_ir(module_source.into())
                .map_err(|e| Error::UnableToAddModuleToContext(e.to_string()))?;
            all_fields.modules.push(module);

            Ok(())
        })?;

        Ok(())
    }

    /// Runs analysis on each module in the context using the provided function,
    /// and returns the analysis results.
    ///
    /// It does not have the ability to modify the underlying modules at all.
    ///
    /// # Errors
    ///
    /// - [`Error`] if any of the passes fail
    pub fn analyze_modules<T>(&self, op: impl Fn(&Module) -> Result<T>) -> Result<Vec<T>> {
        self.with_modules(|mods| mods.iter().map(op).collect())
    }

    /// Runs a transformation on all modules in the context using the provided
    /// function, returning any results from the modification.
    ///
    /// # Errors
    ///
    /// - [`Error`] if any of the passes fail
    pub fn modify_modules<T>(&mut self, op: impl Fn(&mut Module) -> Result<T>) -> Result<Vec<T>> {
        self.with_modules_mut(|mods| mods.iter_mut().map(op).collect())
    }

    /// Gets a reference to the underlying LLVM context.
    #[must_use]
    pub fn context_raw(&self) -> &LLVMContext {
        self.borrow_llvm_context()
    }
}

impl From<SourceContext> for LLVMContext {
    fn from(value: SourceContext) -> Self {
        value.into_heads().llvm_context
    }
}
