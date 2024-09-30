# Getting LLVM IR Output

Most toolchains that compile their source language using LLVM have a means by which they can output
LLVM IR instead of completing the compilation process to the target machine. This document exists as
a quick record of how to get such output from the various source languages we are working with in
this project.

## Rust

`rustc` will emit LLVM IR when passed the `--emit=llvm-ir` flag, and LLVM bytecode when passed the
`--emit=llvm-bc`. This will output `.ll` (or `.bc`) files into the `target` directory corresponding
to your compiled file (usually in `build-type/deps/crate-name-hash.ll`). For more information, see
the [`rustc` developer guide](https://rustc-dev-guide.rust-lang.org/backend/debugging.html).

- This can be passed to the correct compiler when using cargo by calling
  `cargo rustc -- --emit=llvm-ir`.
- You can also set the `RUSTC_FLAGS` environment variable before invoking cargo as normal:
  `RUSTFLAGS='--emit=llvm-ir'`.

There are some difficulties here with multiple-unit compilation that need to be figured out, but
this is a reasonable starting point.

For individual functions and compilation units, there is also the `cargo llvm-ir` command, which can
be run after installing `cargo-asm` (`cargo install cargo-asm`).
