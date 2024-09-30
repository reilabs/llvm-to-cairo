//! Contains the compilation context, which is a way of tracking the compilation
//! units being processed by the compiler.

use inkwell::context::Context as LLVMContext;

/// The compilation context manages the compiler's state across operations.
struct Context {
    llvm_context: LLVMContext,
}

impl Context {
    /// Creates a new, empty, compilation context.
    pub fn new() -> Self {
        let llvm_context = LLVMContext::create();
        Self { llvm_context }
    }

    // pub fn add_ir_file(&mut self, path: impl Into<Path>) -> Result<()>
}
