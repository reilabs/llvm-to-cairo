//! Error types and utilities to do with the compilation from LLVM IR to Cairo
//! IR.

use inkwell::support::LLVMString;
use thiserror::Error;

/// The result type for use in the compiler.
pub type Result<T> = std::result::Result<T, Error>;

/// This error type is for use during the process of compilation from LLVM IR to
/// the Cairo IR.
#[derive(Debug, Error)]
pub enum Error {
    /// Emitted when code tries to construct an invalid ordering of compiler
    /// passes.
    #[error("Pass ordering was invalid: {_0}")]
    InvalidPassOrdering(String),

    /// An error when doing IO during compilation.
    #[error(transparent)]
    IOError(#[from] std::io::Error),

    /// An error coming from LLVM.
    ///
    /// Unfortunately this does not directly contain an `LLVMString` as we want
    /// our error types to be [`Send`] and `LLVMString` is not.
    #[error("LLVM Error: {_0}")]
    LLVMError(String),

    /// Emitted when an attempt is made to add a module to the compilation
    /// context, but cannot do soe compilation context, but cannot do so.
    #[error("Unable to add module to context: {_0}")]
    UnableToAddModuleToContext(String),
}

impl From<LLVMString> for Error {
    /// Wrap an error from LLVM into our error type.
    fn from(value: LLVMString) -> Self {
        Self::LLVMError(value.to_string())
    }
}
