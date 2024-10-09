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
/// encapsulated within the library which claims to have been checked by many
/// people.
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
    /// Creates a new, empty, source compilation context.
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
    pub fn analyze_module<T>(&self, mut op: impl FnMut(&Module) -> Result<T>) -> Result<T> {
        self.with_module(|module| op(module))
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
    pub fn modify_module<T>(&mut self, mut op: impl FnMut(&mut Module) -> Result<T>) -> Result<T> {
        self.with_module_mut(|module| op(module))
    }
}

impl Into<LLVMContext> for SourceContext {
    fn into(self) -> LLVMContext {
        self.into_heads().llvm_context
    }
}
