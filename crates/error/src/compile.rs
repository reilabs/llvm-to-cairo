//! Error types and utilities to do with the compilation from LLVM IR to Cairo
//! IR.

use std::str::Utf8Error;

use inkwell::support::LLVMString;
use thiserror::Error;

/// The result type for use in the compiler.
pub type Result<T> = std::result::Result<T, Error>;

/// This error type is for use during the process of compilation from LLVM IR to
/// the Cairo IR.
#[derive(Debug, Error)]
pub enum Error {
    /// A generic compilation failure with a string message, used as a catch-all
    /// for cases that are uncommon enough to not have specific error variants
    /// for them.
    #[error("Compilation failed: {_0}")]
    CompilationFailure(String),

    /// An error that occurs when trying to convert from the LLVM string
    /// representation used by Inkwell to the UTF-8 string representation used
    /// by Rust.
    #[error("Could not create Rust string from C string: {_0}")]
    CStrConversionError(#[from] Utf8Error),

    #[error("`{_0}` with invalid segment `{_1}` could not be parsed as an LLVM data layout")]
    InvalidDataLayoutSpecification(String, String),

    /// Emitted when code tries to construct an invalid ordering of compiler
    /// passes.
    #[error("Invalid Pass Ordering: {_0}")]
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

    #[error("We only support targets that use a single address space numbered 0")]
    UnsupportedAdditionalAddressSpaces,

    #[error("We do not support targets with non-integral pointers configured.")]
    UnsupportedNonIntegralPointerConfiguration,

    /// Emitted when we encounter an LLVM type that we do not support.
    #[error("The LLVM basic type {_0} is not supported")]
    UnsupportedType(String),
}

impl From<LLVMString> for Error {
    /// Wrap an error from LLVM into our error type.
    fn from(value: LLVMString) -> Self {
        Self::LLVMError(value.to_string())
    }
}
