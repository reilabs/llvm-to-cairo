//! There are many ways that we can add a module to the compilation context, so
//! rather than creating a proliferation of methods with subtly-different input
//! types, we instead take one type that can be created from many.

use std::path::Path;

use inkwell::{memory_buffer::MemoryBuffer, support::LLVMString};

/// A unified type for all the different ways that we support adding a module to
/// the compiler's [`crate::compile::source::SourceContext`].
pub struct SourceModule {
    /// The underlying representation of the module to be passed to LLVM.
    memory_buffer: MemoryBuffer,
}

impl TryFrom<(String, String)> for SourceModule {
    type Error = LLVMString;

    /// Try to create a module source from the provided tuple of `name` and
    /// `contents`.
    fn try_from((name, contents): (String, String)) -> Result<Self, Self::Error> {
        let memory_buffer =
            MemoryBuffer::create_from_memory_range(contents.as_bytes(), name.as_str());
        Ok(Self { memory_buffer })
    }
}

impl TryFrom<&Path> for SourceModule {
    type Error = LLVMString;

    /// Try to create a module source from the LLVM IR at the provided `path`.
    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        let memory_buffer = MemoryBuffer::create_from_file(path)?;
        Ok(Self { memory_buffer })
    }
}

impl From<SourceModule> for MemoryBuffer {
    fn from(value: SourceModule) -> Self {
        value.memory_buffer
    }
}
