# LLVM IR Generation Features

## Introduction

The process of Rust compilation involves several steps to transform Rust source code into an
executable binary. The `rustc` compiler first picks up the Rust code file and generates an
intermediate representation known as HIR (High-Level Intermediate Representation). It is then
converted to MIR (Mid-Level) and then, finally, to LLVM IR. HIR and MIR are out of scope of this
document.

LLVM then performs multiple optimization and transformation passes over the IR. The final output
from LLVM's operations is a binary file containing machine code. This document focuses on generating
LLVM IR from Rust code and the various passes performed on the LLVM IR.

## Rustc (LLVM IR generation)

See [Getting LLVM IR Output](Getting LLVM IR Output.md).

## LLVM IR Passes

[LLVM passes](https://llvm.org/docs/Passes.html) come in three flavors: **analysis**, **transform**,
and **utils**.

### Analysis Passes

Analysis passes read the IR and output some data about the code. These passes can be helpful to
generate insights and optimize further transformations:

- Control graph analysis
- Memory access dependencies
- Call graph printing
- Natural loops detection
- Instruction type counting

These analytical insights can serve as valuable inputs for optimizing the compiler's performance.

### Transform Passes

Transform passes modify the IR in various ways, often utilizing data from the analysis passes:

- Dead code, arguments, store, globals, loops, and tail call elimination
- Global variable optimization and value numbering
- Function inlining and loop unrolling
- Lower `invoke`s to `call`s and `SwitchInst`s to branches
- Combining redundant instructions (note: should e.g. merge two `add`s into one; can be problematic
  if instruction swapping is undesired)
- Lower atomic intrinsics to non-atomic form
- Promoting memory to registers (possibly useful if this leverages Cairo's memory model with
  read-only variables?)

### Utility Passes

Utility passes encapsulate operations that don't fit into the above categories. I didn't notice any
particularly interesting utility passes.

### `opt` - the LLVM optimization tool

The official LLVM documentation may not always be up-to-date. Refer to `opt -help` for the latest
source of information for LLVM passes and options.

To get the list of all flags `opt` accepts, without arch-specific options, call this command:

```sh
$ opt -help | grep -ivE 'aarch64|amdgpu|arm|avr|hexagon|mips|msp430|nvptx|ppc|r600|riscv|si|systemz|wasm|x86'
```

```

On my machine (LLVM version 18.1.8 aarch64-apple-darwin23.6.0) this command returns 463 different
flags.

#### Optimization Levels

LLVM offers several optimization levels:

- `-O0`: No optimization
- `-O1`: Moderate optimization
- `-O2`: Default optimization
- `-O3`: High-level optimization
- `-Os`: Optimize for size
- `-Oz`: Optimize aggressively for size

#### Architecture-Specific Passes

There are numerous architecture-specific passes prefixed with the arch name:

- `aarch64-`,
- `amdgpu-`,
- `arm-`,
- `avr-`,
- `hexagon-`,
- `mips-`,
- `msp430-`,
- `nvptx-`,
- `ppc-`,
- `r600-`,
- `riscv-`,
- `si-`,
- `systemz-`,
- `wasm-`,
- `x86-`.

Since we're not generating any machine code for these architectures, these passes are irrelevant to
us. Some arch-specific flags do not have matching prefixes but flags descriptions mention these
names, so they can be filtered out from the help output. Arch, CPU and EABI selectors also seem
irrelevant.

#### Code Organization

LLVM provides options for code organization:

- Emit basic blocks, functions data into separate sections,
- Configure data layout with a string value.

#### Floating Point Optimization

Several flags allow the selection of denormal number handling (IEEE 754, preserved sign, positive
zero, unknown) and further optimizations for floating-point calculations. They sound like they may
have impact on ALU and/or FPU. There are also multiple optimization flags for floating points, which
may be worth investigating again when we get to FP design and implementation.

#### Memory Analysis

`opt` offers a `--memoryssa` pass which provides
[updated memory dependency analysis](https://llvm.org/docs/Passes.html#memdep-memory-dependence-analysis),
replacing [the older pass](https://llvm.org/docs/MemorySSA.html).

#### Polly Optimizer

A set of `--polly-` flags is available to control the
[Polly loop optimizer](https://polly.llvm.org/docs/Architecture.html), which is extensive and may
require its own detailed research to understand fully.

By understanding and utilizing these passes effectively, we can optimize the compilation process,
resulting in efficient and performant executable binaries.
```
