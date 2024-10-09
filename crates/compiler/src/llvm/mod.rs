//! Utilities for working with LLVM concepts inside the codebase. They are
//! intended to bridge between the worlds of LLVM and the worlds of our compiler
//! itself, and hence aid in analysis and transformation of the LLVM IR.

use crate::llvm::typesystem::LLVMType;

pub mod data_layout;
pub mod typesystem;

/// The type of top-level entry that is encountered in the module.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum TopLevelEntryKind {
    /// A declaration of an external symbol including the name, attributes, and
    /// signature.
    Declaration,

    /// A definition of symbol including the name, attributes, signature, and
    /// **body** (consisting of basic blocks).
    Definition,
}

/// A trait representing objects that have an [`LLVMType`] ascribed to them.
///
/// This is to enable a uniform interface for richer compilation and metadata
/// structures to easily provide their type to a caller.
pub trait HasLLVMType {
    /// Gets the LLVM type for the implementing object.
    fn get_type(&self) -> LLVMType;
}

/// An `LLVMType` _obviously_ has an LLVM type, so we provide a blanket
/// implementation here.
impl HasLLVMType for LLVMType {
    fn get_type(&self) -> LLVMType {
        self.clone()
    }
}
