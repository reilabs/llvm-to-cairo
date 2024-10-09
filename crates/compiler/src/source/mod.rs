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
/// # Self-Referential Structure Definition
///
/// Inkwell's [`Module`] (and many other structs returned from the Inkwell API)
/// are bound by lifetime to the LLVM context object. We don't want these
/// lifetimes to leak into our API and propagate throughout the compiler, so
/// instead we encapsulate them within this struct.
///
/// In order to do this, we use the [`ouroboros`] crate to create a
/// self-referential struct. What this means is that the struct can contain an
/// object, and also have fields that _reference_ those objects. This is
/// disallowed by Rust's ownership model without `unsafe` code, so by using a
/// crate we encapsulate that unsafety and take advantage of the fact that it
/// has likely been looked at by more people than just us.
///
/// As part of using this crate, @iamrecursion has checked it for correctness in
/// this use-case.
#[self_referencing]
#[derive(Debug)]
pub struct SourceContext {
    /// The underlying context that contains the LLVM representation of the
    /// input IR.
    llvm_context: LLVMContext,

    /// The module in this LLVM context. This contains the objects that will be
    /// directly compiled here.
    #[borrows(llvm_context)]
    #[not_covariant]
    module: Module<'this>,
}

impl SourceContext {
    /// Creates a new, empty, source compilation context, wrapping the provided
    /// `module` for compilation.
    ///
    /// # Errors
    ///
    /// - [`Error::UnableToAddModuleToContext`] if the provided `module` cannot
    ///   be added to the context.
    pub fn create(module: impl TryInto<SourceModule, Error = impl ToString>) -> Result<Self> {
        let llvm_context = LLVMContext::create();
        let module_source = module
            .try_into()
            .map_err(|e| Error::UnableToAddModuleToContext(e.to_string()))?;

        SourceContextTryBuilder {
            llvm_context,
            module_builder: |llvm_context| {
                let module = llvm_context
                    .create_module_from_ir(module_source.into())
                    .map_err(|e| Error::UnableToAddModuleToContext(e.to_string()))?;
                Ok(module)
            },
        }
        .try_build()
    }

    /// Runs analysis on the module in the context using the provided function,
    /// and returns the analysis results.
    ///
    /// It does not have the ability to modify the underlying module at all.
    ///
    /// # Function Mutability
    ///
    /// As we may want to pass methods with mutable receivers as the operation
    /// here, we say it can be an instance of [`FnMut`]. Do note that this is a
    /// super-trait of [`Fn`] and hence `op` is not _required_ to capture
    /// anything mutably.
    ///
    /// # Errors
    ///
    /// - [`Error`] if the provided `op` returns an error.
    pub fn analyze_module<T>(&self, op: impl FnMut(&Module) -> Result<T>) -> Result<T> {
        self.with_module(op)
    }

    /// Runs a transformation on the module in the context using the provided
    /// function, returning any results from the modification.
    ///
    /// # Function Mutability
    ///
    /// As we may want to pass methods with mutable receivers as the operation
    /// here, we say it can be an instance of [`FnMut`]. Do note that this is a
    /// super-trait of [`Fn`] and hence `op` is not _required_ to capture
    /// anything mutably.
    ///
    /// # Errors
    ///
    /// - [`Error`] if the provided `op` returns an error.
    pub fn modify_module<T>(&mut self, op: impl FnMut(&mut Module) -> Result<T>) -> Result<T> {
        self.with_module_mut(op)
    }
}

impl From<SourceContext> for LLVMContext {
    fn from(value: SourceContext) -> Self {
        value.into_heads().llvm_context
    }
}
