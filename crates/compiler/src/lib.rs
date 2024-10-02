//! This library implements the functionality necessary for the compilation of
//! [LLVM IR](https://llvm.org/docs/LangRef.html) to the
//! [Cairo](https://www.cairo-lang.org) programming language in order to enable
//! the execution of LLVM-compiled languages on top of the
//! [CairoVM](https://github.com/lambdaclass/cairo-vm) and hence on the
//! [Starknet](https://www.starknet.io) rollup L2.
//!
//! The goals of this project are twofold:
//!
//! 1. To enable writing contracts in languages that compile to LLVM.
//! 2. To enable use of libraries written in such languages as part of the Cairo
//!    ecosystem (e.g. from a contract written in Cairo itself).
//!
//! # Process Overview
//!
//! While more information can be found in the module-level documentation of
//! each part of this codebase, a brief overview of the compilation process can
//! be stated as follows:
//!
//! 1. We ingest LLVM IR in textual format.
//! 2. We translate that LLVM IR to a combination of Cairo's internal IR, and
//!    invocation of polyfills for operations that our target CPU does not
//!    support.
//! 3. We optimize those polyfills to achieve better performance.
//!
//! It should be noted that point 2 above is doing a lot of heavy lifting. As
//! part of this translation we have to account for mismatches between calling
//! conventions, stack and memory semantics, and perform translations of these
//! things where they cannot directly be implemented using a polyfill.
//!
//! # Language Support
//!
//! The major focus in the initial phases of the project is on using
//! [Rust](https://rust-lang.org) as the source language, but the goal is to
//! eventually support _any_ major language (Swift, C++, and so on) that can
//! target LLVM.
//!
//! While most of the work is source-language agnostic, each language does
//! require _some_ specialized work to allow those languages to properly call
//! intrinsics that can interact with the chain and the larger Starknet
//! ecosystem.

#![warn(clippy::all, clippy::cargo, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)] // Allows for better API naming
#![allow(clippy::multiple_crate_versions)] // Enforced by our dependencies

pub mod compile;
pub mod polyfill;

#[cfg(test)]
mod test {
    use std::path::Path;

    use crate::compile::{source::SourceContext, CompilerBuilder};

    #[test]
    fn run() -> anyhow::Result<()> {
        let test_input = r"input/add.ll";
        let mut ctx = SourceContext::create();
        ctx.add_module(Path::new(test_input))?;

        let compiler = CompilerBuilder::new(ctx).build();
        let result = compiler.run()?;
        assert_eq!(result.result_module, ());

        Ok(())
    }
}
