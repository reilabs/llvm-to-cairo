//! Error types and utilities to do with the compilation from LLVM IR to Cairo
//! IR.

use thiserror::Error;

/// This error type is for use during the process of compilation from LLVM IR to
/// the Cairo IR.
#[derive(Clone, Debug, Error)]
pub enum Error {
    #[error("Miscellaneous compilation error: {_0}")]
    Miscellaneous(String),
}
