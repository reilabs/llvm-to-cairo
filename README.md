# LLVM on CairoVM

This repository contains the efforts to enable compilation of LLVM bytecode to run on top of the
[CairoVM](https://github.com/lambdaclass/cairo-vm) and [Starknet](https://www.starknet.io). The
goals of this project are threefold:

1. **Provable Rust Execution:** To provide the ability to prove the execution of LLVM bytecode using
   Starknet's proving infrastructure, thereby allowing verification of said execution.
2. **Writing Contracts in LLVM Languages:** To provide the ability to write contracts for execution
   on Starknet using any language that compiles to LLVM (while recognizing that this will require a
   small per-language effort).
3. **Using Foreign Libraries from Cairo:** There exist a wealth of useful cryptographic libraries in
   languages such as Rust that compile to LLVM. Rather than requiring users to reimplement them in
   [Cairo](https://www.cairo-lang.org), this effort would allow them to be exposed directly.

The initial efforts for this project are focusing on using [Rust](https://rust-lang.org) as the
source of the LLVM bytecode, and will see support libraries implemented for this language.
Nevertheless, this project aims to be agnostic to the actual source language of the bytecode besides
the language-specific libraries.

## Architectural Overview

As there is a significant mismatch between the computational model expected by LLVM, and the Cairo
model of execution, we take a multi-layered approach.

- **Compilation:** Where there is a direct correspondence between LLVM and Cairo's semantics, we can
  perform direct compilation, generating Cairo's `FlatLowered` IR from LLVM IR.
- **Polyfills:** Where there is a mismatch, we have to implement the mismatched operation as an
  _emulation_ or a _polyfill_. These will implement the expected semantics (often given by
  [compiler-rt](https://compiler-rt.llvm.org)) as Cairo code that can then be called into by the
  compilation process.

As Cairo is not an operating system, we restrict our source code to be `#[no_std]` or the equivalent
for other languages. This is necessary as we cannot provide useful primitives for things like the
filesystem, network, or even threading.

In the future, we intend to move many of our polyfilled operations into native operations as part of
the CASM instruction set, or as Cairo
[builtins](https://book.cairo-lang.org/ch204-00-builtins.html#builtins) to improve performance.
Starting with the emulation or polyfill layer, however, lets us determine which operations are going
to be most effective to transfer first, and help us arrive to the final design of such operations
before moving them to AIR.

## Contributing

If you want to contribute code or documentation (non-code contributions are always welcome) to this
project, please take a look at our [contributing](./docs/CONTRIBUTING.md) documentation. It provides
an overview of how to get up and running, as well as what the contribution process looks like for
this repository.
