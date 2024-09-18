//! This library implements the functionality necessary for the compilation of
//! [LLVM IR](https://llvm.org/docs/LangRef.html) to the
//! [Cairo](https://www.cairo-lang.org) programming language in order to enable
//! the execution of LLVM-compiled languages on top of the
//! [CairoVM](https://github.com/lambdaclass/cairo-vm) and hence on
//! [Starknet](https://www.starknet.io).
//!
//! The goals of this project are twofold:
//!
//! 1. To enable writing contracts in languages that compile to LLVM.
//! 2. To enable use of libraries in such languages as part of the Cairo
//!    ecosystem.
//!
//! The major focus in the initial phases of the project is on using
//! [Rust](https://rust-lang.org) as the source language, but the goal is to
//! eventually support _any_ major language (Swift, C++, and so on) that can
//! target LLVM.

#![warn(clippy::all, clippy::cargo, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)] // Allows for better API naming
#![allow(clippy::multiple_crate_versions)] // Enforced by our dependencies

pub mod error;
