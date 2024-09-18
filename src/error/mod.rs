//! Error handling types and utilities.
//!
//! # Error Conventions
//!
//! As we are providing a library that others may want to interact with from
//! _code_ as well as from the CLI driver, we keep our errors strongly typed at
//! all times. While libraries like
//! [anyhow](https://docs.rs/anyhow/latest/anyhow/) are well-suited for
//! application code, they make it more difficult than is necessary to handle
//! specific errors in library code. To that end, we make sure that our errors
//! are kept strongly typed within the library as much as is possible.

pub mod llvm_compile;

use thiserror::Error;

/// The result type to be used at the boundaries of the library.
pub type Result<T> = std::result::Result<T, Error>;

/// The root of the error hierarchy for this crate.
///
/// All errors should be able to be implicitly converted to this error type as
/// this is the type that is used at the boundaries of the library. Though we do
/// not make a habit of hiding things, any function intended to be part of the
/// _truly_ public interface of this library should return this error type.
#[derive(Clone, Debug, Error)]
pub enum Error {
    #[error(transparent)]
    LlvmCompile(#[from] llvm_compile::Error),

    #[error("An unknown error occurred: {_0}")]
    Miscellaneous(String),
}
