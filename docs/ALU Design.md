# ALU Design

This document describes the research done for
[#27 Design ALU](https://github.com/reilabs/llvm-to-cairo/issues/27).

The first part, [**Research**](#Research), describes selected features of LLVM IR, Rust and Cairo
(both the virtual machine and the programming language), that impact the way we must handle
arithmetic and logic operations. The second part, [**Design**](#design), specifies decisions made
with regard to the shape of the ALU component of the project.

Most of the design decision are based on the outcomes of experiments described in the research part.
Some decisions are made arbitrarily. All decisions are subject to change, especially if more
information is gathered during the implementation phase. Nevertheless, we do not expect the final
shape of ALU to be much different from this design document. Should design changes occur, this
document will be updated.

## Research

ALU will have to target two concepts in the LLVM IR: instructions and intrinsics.

### Instructions

An example of IR using the `add` instruction for arithmetic operations:

```llvm
define i32 @add(i32 %a, i32 %b) {
entry:
  %sum = add i32 %a, %b      ; Add the two integers
  ret i32 %sum               ; Return the result
}
```

All the instructions we need to look for are already captured by our
[polyfill listing](https://www.notion.so/reilabs/LLVM-IR-Polyfills-10ed2f80c87480cb8694f581b726808c):
`add`, `sub`, `mul`, `udiv`, `sdiv`, `urem`, `srem`, `shl`, `lshr`, `ashr`, `and`, `or`, `xor`.

#### Keywords

An instruction is not just its opcode and operands, e.g. `$3 = add $1, $2`, but there are some
keywords modifying its behavior. An
[example for `add`](https://llvm.org/docs/LangRef.html#add-instruction):

```llvm
<result> = add <ty> <op1>, <op2>          ; yields ty:result
<result> = add nuw <ty> <op1>, <op2>      ; yields ty:result
<result> = add nsw <ty> <op1>, <op2>      ; yields ty:result
<result> = add nuw nsw <ty> <op1>, <op2>  ; yields ty:result
```

- `<ty>` is type, e.g. `i32`,
- `nuw` - No Unsigned Wrap,
- `nsw` - No Signed Wrap.

#### Poison

In the [example of `add`](#keywords), if `nuw` or `nsw` keywords occur, they guarantee specific
behavior, i.e. no (un)signed overflow. However, if the operands cause the overflow, the instruction
returns a poison, which is an equivalent of a value indicating undefined behavior that can propagate
throughout the program.

[According to the experiment](https://github.com/reilabs/llvm-to-cairo/issues/27#issuecomment-2397645979),
LLVM does not seem to emit such instructions from the Rust code, so the initial version of ALU will
not handle `nuw`, `nsw` or other keywords in any specific way.

### Intrinsics

Consider the following LLVM IR code:

```llvm
%0 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %left, i64 %right), !dbg !17
```

Unlike the [previous example](#keywords), this snippet does not contain the `add` instruction. The
adding operation is done by an intrinsic named `llvm.uadd.with.overflow.i64`, which is called with
the `call` instruction. The intrinsic is defined in the LLVM codebase and its source code does not
make it into the `.ll` file produced out of the adding operation in Rust code.

The LLVM Language Reference Manual has an extensive list of intrinsics. Here's the example of
[`llvm.uadd.with.overflow.<ty>`](https://llvm.org/docs/LangRef.html#llvm-uadd-with-overflow-intrinsics).

### Data Types

#### Integers

LLVM IR supports integers of arbitrary width. A general syntax for an integer type is `iN`, where
`N` can be anything from 1 to 2^32. Since LLVM does not have a dedicated type for boolean values,
`i1` is used instead.

The Cairo VM internally operates on 252-bit-long field elements - `felt252`. On the higher level of
abstraction, the Cairo language supports
[integers of specific lengths](https://book.cairo-lang.org/ch02-02-data-types.html): 8 bit, 16 bit,
32 bit, 64 bit, 128 bit and 256 bit. Cairo also supports booleans.

[Rust supports integers of width from 8 to 128 bit](https://doc.rust-lang.org/book/ch03-02-data-types.html)
with the same increment Cairo does, plus architecture-dependent `isize` and `usize`. Rust also
supports booleans.

The Cairo VM does not have classical registers of a length constrained by the hardware. Therefore
there is no obvious indicator of how long `usize`/`isize` should be on that target. Since from the
LLVM point of view a pointer must have a finite size, this decision must be made based on some other
feature of the architecture. We have evaluated the following choices:

- The Cairo language already has 32 bit `usize`, so we can follow this approach, making `usize` and
  `isize` also 32 bit. This approach was rejected for the lack of the strong rationale behind
  Cairo's choice of this particular width. It does not seem to correspond with any feature of the
  platform architecture.
- The architecture's natural word size is 252 bit, being the length of the field element. It may be
  reasonable to set `usize` and `isize` length to 252 bit.
- 256 bit is the next power-of-2 after 252. Having `usize` and `isize` 256 bit long leaves 4 extra
  bits that may be used to keep some metadata.

Ultimately the size of `usize` and `isize` has been decided to be 64 bits, which is neither of the
above possibilities. This length is a consequence of using the `aarch64-unknown-none-softfloat`
target triple. The choice of the triple determines the length of the pointer which in turn
determines the length of `usize`. This specific triple has been chosen for its soft float support
and no host operating system. The pointer length is just one of its parameters we accept at this
stage of the project. This target triple is a temporary choice before a custom target triple is
proposed. When designing our custom triple, it is possible that the choice of `usize` and `isize`
width will be reevaluated and possibly changed to match the width of the field element.

Summing up, we expect to see in the IR integers of the following lengths: 1, 8, 16, 32, 64 and 128
bits. We do not intend to support operations over arbitrary-width integers. We also decided to add
support for 128 bit integers in later phase of the project.

#### Pointers

LLVM IR has only one generic pointer type - `ptr`, which works as a rough equivalent of the `void *`
in C. Type-specific pointers (e.g. equivalent of C's `int *`) existed in LLVM in the past, but are
now deprecated. Therefore, we expect to see in the input IR only the generic `ptr` pointer. This
does not translate well to higher-level programming languages.

No-std Rust support for pointers is twofold:

- strongly typed smart pointers with the `Box<>` construct, that handle their own memory under the
  hood,
- strongly typed raw pointers that can be dereferenced within an `unsafe {}` block.

Usage of smart pointers translate to multiple LLVM IR instructions, involving heap memory
allocation. The IR generated from a similar code involving raw pointers is simpler in comparison.
Both approaches ultimately generate the `ptr` LLVM type.

The Cairo language operates solely on strongly typed smart pointers. There is no raw pointers. The
`unsafe` keyword is reserved for the future use.

Based on these observations, ALU operations must be strongly typed, by the limitation of the
language they will be implemented in. Specifically, we are unable to provide a generic
`__llvm_instr_ptr` polyfill for an instruction accepting a single pointer argument, because such
construct is not supported by Cairo. Instead, we must provide the whole family of polyfills, one for
each possible pointer type:

- `__llvm_instr_pbool`,
- `__llvm_instr_pi8`,
- `__llvm_instr_pi16`,
- `__llvm_instr_pi32`,
- `__llvm_instr_pi64`,
- `__llvm_instr_pi128` _(optionally, if a triple supporting 128 bit integers is introduced)_.

Some of the implementations may be omitted if they do not reflect LLVM IR semantics for a given
instruction.

When the input IR will is parsed and a generic pointer is found, its concrete type must be inferred
from the context, to allow matching with the proper implementation. For example this IR snippet:

```llvm
%num = alloca [4 x i8], align 4
%_5 = ptrtoint ptr %num to i64, !dbg !13
```

must be mapped to the following implementation: `__llvm_ptrtoint_pi8_to_i64`, as `num` is a pointer
to an array of `i8`.

#### Vectors

Neither the Cairo VM, Cairo language nor no-std Rust have support for vectorized operations.

LLVM IR has vectors as first class citizens. However,
_[vector types are used where multiple primitive data are operated in parallel using a single instruction (SIMD)](https://llvm.org/docs/LangRef.html#vector-type)_.
If Cairo target definition supplied to `rustc` does not suggest the existence of vector extension on
the target platform, we would not expect any vector intrinsics to appear in the IR. Therefore,
vector support is not planned as part of the initial phase of the project.

#### Type Conversion

Cairo does not have Rust's `as` keyword, so it's not possible to do e.g. `let a = b as u32` given
`b` is a `u64`.

An equivalent operation in Cairo is `let a: u32: b.try_into().unwrap();`. This approach has two
disadvantages:

- it will panic if the value of `b` is larger than `0xFFFFFFFF`,
- there is no automatic wraparound as in the case of `as`.

Should type conversion be necessary in the implementation of the operations, it will need to handle
the type conversion with `try_into()` and manually recover from errors:

```rust
let result: u32 = match sum.try_into() {
  Ok(val) => val,
  Err(_) => {
    // Handle the wraparound manually
  }
};
```

### Statefulness

A real Arithmetic-Logic Unit in a CPU is a finite state machine. Some states, interesting from the
programmer's point of view, can be captured as contents of the CPU registers. Such state is e.g. the
next instruction (as pointed to by Program Counter or its equivalent), values of operands stored in
two general purpose registers or the result of the last operation stored in another GP register and
a flag register, where specific bits signal certain conditions (e.g. the result being zero or an
integer overflow).

The LLVM-to-Cairo infrastructure needs to deliver pieces of code translating generic LLVM arithmetic
operations to their counterparts specific to the Cairo VM architecture. This translation will be
done on the code level, during one of the LLVM-to-Cairo pipeline stages. Namely, this will be not
runtime translation, but rather a compilation time one. Therefore, there is no global state to be
managed during that time.

Additionally, it has been noticed
[in one of the experiments](https://github.com/reilabs/llvm-to-cairo/issues/27#issuecomment-2391893640),
that LLVM IR follows the same principle of not managing the internal state of arithmetic operations.
This is either done by:

- returning a tuple containing both the operation result and the state information:

```llvm
%0 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %left, i64 %right), !dbg !17
%_3.0 = extractvalue { i64, i1 } %0, 0, !dbg !17
%_3.1 = extractvalue { i64, i1 } %0, 1, !dbg !17
br i1 %_3.1, label %panic, label %bb1, !dbg !17
```

- by demanding that input data is correct and producing undefined behavior otherwise,
- by emitting poison values, if producing a correct value is not possible.

Based on these observations, we have decided to deliver the ALU as a collection of stateless
arithmetic operations.

### Tests

Cairo has an
[integrated test framework](https://book.cairo-lang.org/ch10-01-how-to-write-tests.html), similar to
the one offered by Rust. Our ALU implementation will come with a test suite to verify that we
implement the desired behavior, e.g. to make sure we indicate overflow on obvious situations like
`0xFFFFFFFF + 1` for a `u32` add.

## Design

### Overview

The ALU will be implemented as a source code written in
[Cairo](https://book.cairo-lang.org/title-page.html). During the
[LLVM-to-Cairo compilation pipeline](https://www.notion.so/reilabs/System-Architecture-113d2f80c874802b8480d997347933a2?pvs=4)
the polyfills implementations will be translated to `FlatLowered` objects and then extracted to
`.flo` files. Then, on the linking phase, all the `.flo` files (those created from arithmetic
operations implementations and those from the LLVM IR) will be linked together.

As discussed in the [relevant section of the Research part](#statefulness), each operation will be a
stateless block of code composed of a single Cairo
[function](https://book.cairo-lang.org/ch02-03-functions.html)(possibly composed of subroutines for
common parts) which is an equivalent concept of a function in any other procedural programming
language.

Each function will follow the semantics of its LLVM IR counterpart. This requires:

- accepting the same number of arguments, of the same type and in the same order as the original
  operation's operands,
- returning the same number or values, of the same type and in the same order as the original
  operation,
- implementing the exact semantics as expected by LLVM.

As an example, for the `sub` instruction, our polyfill operating on `i8` operands must:

- accept exactly two `i8` operands,
- the operands must be in the same order, i.e. for `sub %a, %b` our polyfill `__llvm_sub_i8_i8` must
  accept `a` and `b` in the same order,
- as `sub %a, %b` performs the `a-b` subtraction, our polyfill must not perform `b-a` instead and
  all corner cases, like underflow, must be handled in the same way as LLVM handles them.

Each function will follow the naming scheme described in the
[relevant section below](#naming-convention).

### Naming Convention

#### Instruction Polyfills

Name template: `__llvm_<opcode>_<ty1>_<ty2>`. In case the instruction works with both operands of
the same data type, the template degrades to `__llvm_<opcode>_<ty>_<ty>`.

In the example of `inst i32 %a, %b`, the polyfill would be named `__llvm_inst_i32_i32`.

If `<ty>` is `i1`, it is translated into `bool`. For an example instruction `inst i1 %a, %b`, the
polyfill would be named `__llvm_inst_bool_bool`.

In case the instruction works with pointer type, the generic LLVM keyword `ptr` is translated to
`p<ty>`. For an example instruction `inst ptr %a, i8 %b`, where `%a` is a pointer to the value of
the same type as `%b`, the polyfill would be named `__llvm_inst_pi8_i8`.

#### Intrinsic Polyfills

Name template: `__<actual name with _ instead of .>_<ty>_<ty>`, where type `<ty>` indicates every
argument accepted by the intrinsic.

In the example of `llvm.uadd.with.overflow.i64(i64 %left, i64 %right)`, the polyfill would be named
`__llvm_uadd_with_overflow_i64_i64`.

All other [naming rules defined for instructions](#instruction-polyfills) also apply to intrinsics.

### Operations

The list below specifies all implementations of arithmetic operations that will be provided by the
ALU. The list is divided to two parts:

- [Implementations emulating LLVM instructions](#based-on-instructions),
- [Implementations emulating LLVM intrinsics](#based-on-intrinsics).

Implementations for every supported integer lengths are specified. Their names follow the naming
convention explained in the section above. Each instruction or intrinsic name is a link to the
relevant part of the LLVM Language Reference Manual.

Operations prepended with (\*) are the extended goal and will not be implemented in the initial
phase. See [Data Types](#data-types) for details.

#### Based on Instructions

- [`add`](https://llvm.org/docs/LangRef.html#add-instruction):
  - `__llvm_add_i8_i8 -> i8`
  - `__llvm_add_i16_i16 -> i16`
  - `__llvm_add_i32_i32 -> i32`
  - `__llvm_add_i64_i64 -> i64`
  - (\*) `__llvm_add_i128_i128 -> i128`
- [`sub`](https://llvm.org/docs/LangRef.html#sub-instruction):
  - `__llvm_sub_i8_i8 -> i8`
  - `__llvm_sub_i16_i16 -> i16`
  - `__llvm_sub_i32_i32 -> i32`
  - `__llvm_sub_i64_i64 -> i64`
  - (\*) `__llvm_sub_i128_i128 -> i128`
- [`mul`](https://llvm.org/docs/LangRef.html#mul-instruction):
  - `__llvm_mul_i8_i8 -> i8`
  - `__llvm_mul_i16_i16 -> i16`
  - `__llvm_mul_i32_i32 -> i32`
  - `__llvm_mul_i64_i64 -> i64`
  - (\*) `__llvm_mul_i128_i128 -> i128`
- [`udiv`](https://llvm.org/docs/LangRef.html#udiv-instruction):
  - `__llvm_udiv_i8_i8 -> i8`
  - `__llvm_udiv_i16_i16 -> i16`
  - `__llvm_udiv_i32_i32 -> i32`
  - `__llvm_udiv_i64_i64 -> i64`
  - (\*) `__llvm_udiv_i128_i128 -> i128`
- [`sdiv`](https://llvm.org/docs/LangRef.html#sdiv-instruction):
  - `__llvm_sdiv_i8_i8 -> i8`
  - `__llvm_sdiv_i16_i16 -> i16`
  - `__llvm_sdiv_i32_i32 -> i32`
  - `__llvm_sdiv_i64_i64 -> i64`
  - (\*) `__llvm_sdiv_i128_i128 -> i128`
- [`urem`](https://llvm.org/docs/LangRef.html#urem-instruction):
  - `__llvm_urem_i8_i8 -> i8`
  - `__llvm_urem_i16_i16 -> i16`
  - `__llvm_urem_i32_i32 -> i32`
  - `__llvm_urem_i64_i64 -> i64`
  - (\*) `__llvm_urem_i128_i128 -> i128`
- [`srem`](https://llvm.org/docs/LangRef.html#srem-instruction):
  - `__llvm_srem_i8_i8 -> i8`
  - `__llvm_srem_i16_i16 -> i16`
  - `__llvm_srem_i32_i32 -> i32`
  - `__llvm_srem_i64_i64 -> i64`
  - (\*) `__llvm_srem_i128_i128 -> i128`
- [`shl`](https://llvm.org/docs/LangRef.html#shl-instruction):
  - `__llvm_shl_i8_i8 -> i8`
  - `__llvm_shl_i16_i16 -> i16`
  - `__llvm_shl_i32_i32 -> i32`
  - `__llvm_shl_i64_i64 -> i64`
  - (\*) `__llvm_shl_i128_i128 -> i128`
- [`lshr`](https://llvm.org/docs/LangRef.html#lshr-instruction):
  - `__llvm_lshr_i8_i8 -> i8`
  - `__llvm_lshr_i16_i16 -> i16`
  - `__llvm_lshr_i32_i32 -> i32`
  - `__llvm_lshr_i64_i64 -> i64`
  - (\*) `__llvm_lshr_i128_i128 -> i128`
- [`ashr`](https://llvm.org/docs/LangRef.html#ashr-instruction):
  - `__llvm_ashr_i8_i8 -> i8`
  - `__llvm_ashr_i16_i16 -> i16`
  - `__llvm_ashr_i32_i32 -> i32`
  - `__llvm_ashr_i64_i64 -> i64`
  - (\*) `__llvm_ashr_i128_i128 -> i128`
- [`and`](https://llvm.org/docs/LangRef.html#and-instruction):
  - `__llvm_and_bool_bool -> bool`
  - `__llvm_and_i8_i8 -> i8`
  - `__llvm_and_i16_i16 -> i16`
  - `__llvm_and_i32_i32 -> i32`
  - `__llvm_and_i64_i64 -> i64`
  - (\*) `__llvm_and_i128_i128 -> i128`
- [`or`](https://llvm.org/docs/LangRef.html#or-instruction):
  - `__llvm_or_bool_bool -> bool`
  - `__llvm_or_i8_i8 -> i8`
  - `__llvm_or_i16_i16 -> i16`
  - `__llvm_or_i32_i32 -> i32`
  - `__llvm_or_i64_i64 -> i64`
  - (\*) `__llvm_or_i128_i128 -> i128`
- [`xor`](https://llvm.org/docs/LangRef.html#xor-instruction):
  - `__llvm_xor_bool_bool -> bool`
  - `__llvm_xor_i8_i8 -> i8`
  - `__llvm_xor_i16_i16 -> i16`
  - `__llvm_xor_i32_i32 -> i32`
  - `__llvm_xor_i64_i64 -> i64`
  - (\*) `__llvm_xor_i128_i128 -> i128`
- [`cmpxchg`](https://llvm.org/docs/LangRef.html#cmpxchg-instruction):
  - Unlike the previous instructions, `cmpxchg` accepts three arguments. Some of the arguments can
    be integers or pointers. Pointer arguments have the `p` prefix, e.g. `pi8` is a pointer to
    `int8`.
  - Unlike the previous instructions, `cmpxchg` returns a tuple: the first value is the original
    value at the memory location pointed to by the first argument (before exchange) and the second
    value indicates if the value loaded equals the value of the second argument.
  - `__llvm_cmpxchg_pi8_i8_i8 -> (i8, bool)`
  - `__llvm_cmpxchg_pi8_pi8_pi8 -> (i8, bool)`
  - `__llvm_cmpxchg_pi16_i16_i16 -> (i16, bool)`
  - `__llvm_cmpxchg_pi16_pi16_pi16 -> (i16, bool)`
  - `__llvm_cmpxchg_pi32_i32_i32 -> (i32, bool)`
  - `__llvm_cmpxchg_pi32_pi32_pi32 -> (i32, bool)`
  - `__llvm_cmpxchg_pi64_i64_i64 -> (i64, bool)`
  - `__llvm_cmpxchg_pi64_pi64_pi64 -> (i64, bool)`
  - (\*) `__llvm_cmpxchg_pi128_i128_i128 -> (i128, bool)`
  - (\*) `__llvm_cmpxchg_pi128_pi128_pi128 -> (i128, bool)`
- [`trunc .. to`](https://llvm.org/docs/LangRef.html#trunc-to-instruction):
  - (\*) `__llvm_trunc_i128_to_i64 -> i64`
  - (\*) `__llvm_trunc_i128_to_i32 -> i32`
  - (\*) `__llvm_trunc_i128_to_i16 -> i16`
  - (\*) `__llvm_trunc_i128_to_i8 -> i8`
  - (\*) `__llvm_trunc_i128_to_bool -> bool`
  - `__llvm_trunc_i64_to_i32 -> i32`
  - `__llvm_trunc_i64_to_i16 -> i16`
  - `__llvm_trunc_i64_to_i8 -> i8`
  - `__llvm_trunc_i64_to_bool -> bool`
  - `__llvm_trunc_i32_to_i16 -> i16`
  - `__llvm_trunc_i32_to_i8 -> i8`
  - `__llvm_trunc_i32_to_bool -> bool`
  - `__llvm_trunc_i16_to_i8 -> i8`
  - `__llvm_trunc_i16_to_bool -> bool`
  - `__llvm_trunc_i8_to_bool -> bool`
- [`zext .. to`](https://llvm.org/docs/LangRef.html#zext-to-instruction):
  - (\*) `__llvm_zext_bool_to_i128 -> i128`
  - `__llvm_zext_bool_to_i64 -> i64`
  - `__llvm_zext_bool_to_i32 -> i32`
  - `__llvm_zext_bool_to_i16 -> i16`
  - `__llvm_zext_bool_to_i8 -> i8`
  - (\*) `__llvm_zext_i8_to_i128 -> i128`
  - `__llvm_zext_i8_to_i64 -> i64`
  - `__llvm_zext_i8_to_i32 -> i32`
  - `__llvm_zext_i8_to_i16 -> i16`
  - (\*) `__llvm_zext_i16_to_i128 -> i128`
  - `__llvm_zext_i16_to_i64 -> i64`
  - `__llvm_zext_i16_to_i32 -> i32`
  - (\*) `__llvm_zext_i32_to_i128 -> i128`
  - `__llvm_zext_i32_to_i64 -> i64`
  - (\*) `__llvm_zext_i64_to_i128 -> i128`
- [`sext .. to`](https://llvm.org/docs/LangRef.html#sext-to-instruction):
  - (\*) `__llvm_sext_bool_to_i128 -> i128`
  - `__llvm_sext_bool_to_i64 -> i64`
  - `__llvm_sext_bool_to_i32 -> i32`
  - `__llvm_sext_bool_to_i16 -> i16`
  - `__llvm_sext_bool_to_i8 -> i8`
  - (\*) `__llvm_sext_i8_to_i128 -> i128`
  - `__llvm_sext_i8_to_i64 -> i64`
  - `__llvm_sext_i8_to_i32 -> i32`
  - `__llvm_sext_i8_to_i16 -> i16`
  - (\*) `__llvm_sext_i16_to_i128 -> i128`
  - `__llvm_sext_i16_to_i64 -> i64`
  - `__llvm_sext_i16_to_i32 -> i32`
  - (\*) `__llvm_sext_i32_to_i128 -> i128`
  - `__llvm_sext_i32_to_i64 -> i64`
  - (\*) `__llvm_sext_i64_to_i128 -> i128`
- [`ptrtoint .. to`](https://llvm.org/docs/LangRef.html#ptrtoint-to-instruction):
  - `__llvm_ptrtoint_pbool_to_bool -> bool`
  - `__llvm_ptrtoint_pbool_to_i8 -> i8`
  - `__llvm_ptrtoint_pbool_to_i16 -> i16`
  - `__llvm_ptrtoint_pbool_to_i32 -> i32`
  - `__llvm_ptrtoint_pbool_to_i64 -> i64`
  - (\*) `__llvm_ptrtoint_pbool_to_i128 -> i128`
  - `__llvm_ptrtoint_pi8_to_bool -> bool`
  - `__llvm_ptrtoint_pi8_to_i8 -> i8`
  - `__llvm_ptrtoint_pi8_to_i16 -> i16`
  - `__llvm_ptrtoint_pi8_to_i32 -> i32`
  - `__llvm_ptrtoint_pi8_to_i64 -> i64`
  - (\*) `__llvm_ptrtoint_pi8_to_i128 -> i128`
  - `__llvm_ptrtoint_pi16_to_bool -> bool`
  - `__llvm_ptrtoint_pi16_to_i8 -> i8`
  - `__llvm_ptrtoint_pi16_to_i16 -> i16`
  - `__llvm_ptrtoint_pi16_to_i32 -> i32`
  - `__llvm_ptrtoint_pi16_to_i64 -> i64`
  - (\*) `__llvm_ptrtoint_pi16_to_i128 -> i128`
  - `__llvm_ptrtoint_pi32_to_bool -> bool`
  - `__llvm_ptrtoint_pi32_to_i8 -> i8`
  - `__llvm_ptrtoint_pi32_to_i16 -> i16`
  - `__llvm_ptrtoint_pi32_to_i32 -> i32`
  - `__llvm_ptrtoint_pi32_to_i64 -> i64`
  - (\*) `__llvm_ptrtoint_pi32_to_i128 -> i128`
  - `__llvm_ptrtoint_pi64_to_bool -> bool`
  - `__llvm_ptrtoint_pi64_to_i8 -> i8`
  - `__llvm_ptrtoint_pi64_to_i16 -> i16`
  - `__llvm_ptrtoint_pi64_to_i32 -> i32`
  - `__llvm_ptrtoint_pi64_to_i64 -> i64`
  - (\*) `__llvm_ptrtoint_pi64_to_i128 -> i128`
  - (\*) `__llvm_ptrtoint_pi128_to_bool -> bool`
  - (\*) `__llvm_ptrtoint_pi128_to_i8 -> i8`
  - (\*) `__llvm_ptrtoint_pi128_to_i16 -> i16`
  - (\*) `__llvm_ptrtoint_pi128_to_i32 -> i32`
  - (\*) `__llvm_ptrtoint_pi128_to_i64 -> i64`
  - (\*) `__llvm_ptrtoint_pi128_to_i128 -> i128`
- [`inttoptr .. to`](https://llvm.org/docs/LangRef.html#inttoptr-to-instruction):
  - `__llvm_inttoptr_bool_to_pbool -> pbool`
  - `__llvm_inttoptr_bool_to_pi8 -> pi8`
  - `__llvm_inttoptr_bool_to_pi16 -> pi16`
  - `__llvm_inttoptr_bool_to_pi32 -> pi32`
  - `__llvm_inttoptr_bool_to_pi64 -> pi64`
  - (\*) `__llvm_inttoptr_bool_to_pi128 -> pi128`
  - `__llvm_inttoptr_i8_to_pbool -> pbool`
  - `__llvm_inttoptr_i8_to_pi8 -> pi8`
  - `__llvm_inttoptr_i8_to_pi16 -> pi16`
  - `__llvm_inttoptr_i8_to_pi32 -> pi32`
  - `__llvm_inttoptr_i8_to_pi64 -> pi64`
  - (\*) `__llvm_inttoptr_i8_to_pi128 -> pi128`
  - `__llvm_inttoptr_i16_to_pbool -> pbool`
  - `__llvm_inttoptr_i16_to_pi8 -> pi8`
  - `__llvm_inttoptr_i16_to_pi16 -> pi16`
  - `__llvm_inttoptr_i16_to_pi32 -> pi32`
  - `__llvm_inttoptr_i16_to_pi64 -> pi64`
  - (\*) `__llvm_inttoptr_i16_to_pi128 -> pi128`
  - `__llvm_inttoptr_i32_to_pbool -> pbool`
  - `__llvm_inttoptr_i32_to_pi8 -> pi8`
  - `__llvm_inttoptr_i32_to_pi16 -> pi16`
  - `__llvm_inttoptr_i32_to_pi32 -> pi32`
  - `__llvm_inttoptr_i32_to_pi64 -> pi64`
  - (\*) `__llvm_inttoptr_i32_to_pi128 -> pi128`
  - `__llvm_inttoptr_i64_to_pbool -> pbool`
  - `__llvm_inttoptr_i64_to_pi8 -> pi8`
  - `__llvm_inttoptr_i64_to_pi16 -> pi16`
  - `__llvm_inttoptr_i64_to_pi32 -> pi32`
  - `__llvm_inttoptr_i64_to_pi64 -> pi64`
  - (\*) `__llvm_inttoptr_i64_to_pi128 -> pi128`
  - (\*) `__llvm_inttoptr_i128_to_pbool -> pbool`
  - (\*) `__llvm_inttoptr_i128_to_pi8 -> pi8`
  - (\*) `__llvm_inttoptr_i128_to_pi16 -> pi16`
  - (\*) `__llvm_inttoptr_i128_to_pi32 -> pi32`
  - (\*) `__llvm_inttoptr_i128_to_pi64 -> pi64`
  - (\*) `__llvm_inttoptr_i128_to_pi128 -> pi128`
- [`bitcast .. to`](https://llvm.org/docs/LangRef.html#bitcast-to-instruction):
  - `__llvm_bitcast_bool_to_bool -> bool`
  - `__llvm_bitcast_bool_to_i8 -> i8`
  - `__llvm_bitcast_bool_to_i16 -> i16`
  - `__llvm_bitcast_bool_to_i32 -> i32`
  - `__llvm_bitcast_bool_to_i64 -> i64`
  - (\*) `__llvm_bitcast_bool_to_i128 -> i128`
  - `__llvm_bitcast_i8_to_bool -> bool`
  - `__llvm_bitcast_i8_to_i8 -> i8`
  - `__llvm_bitcast_i8_to_i16 -> i16`
  - `__llvm_bitcast_i8_to_i32 -> i32`
  - `__llvm_bitcast_i8_to_i64 -> i64`
  - (\*) `__llvm_bitcast_i8_to_i128 -> i128`
  - `__llvm_bitcast_i16_to_bool -> bool`
  - `__llvm_bitcast_i16_to_i8 -> i8`
  - `__llvm_bitcast_i16_to_i16 -> i16`
  - `__llvm_bitcast_i16_to_i32 -> i32`
  - `__llvm_bitcast_i16_to_i64 -> i64`
  - (\*) `__llvm_bitcast_i16_to_i128 -> i128`
  - `__llvm_bitcast_i32_to_bool -> bool`
  - `__llvm_bitcast_i32_to_i8 -> i8`
  - `__llvm_bitcast_i32_to_i16 -> i16`
  - `__llvm_bitcast_i32_to_i32 -> i32`
  - `__llvm_bitcast_i32_to_i64 -> i64`
  - (\*) `__llvm_bitcast_i32_to_i128 -> i128`
  - `__llvm_bitcast_i64_to_bool -> bool`
  - `__llvm_bitcast_i64_to_i8 -> i8`
  - `__llvm_bitcast_i64_to_i16 -> i16`
  - `__llvm_bitcast_i64_to_i32 -> i32`
  - `__llvm_bitcast_i64_to_i64 -> i64`
  - (\*) `__llvm_bitcast_i64_to_i128 -> i128`
  - (\*) `__llvm_bitcast_i128_to_bool -> bool`
  - (\*) `__llvm_bitcast_i128_to_i8 -> i8`
  - (\*) `__llvm_bitcast_i128_to_i16 -> i16`
  - (\*) `__llvm_bitcast_i128_to_i32 -> i32`
  - (\*) `__llvm_bitcast_i128_to_i64 -> i64`
  - (\*) `__llvm_bitcast_i128_to_i128 -> i128`
  - `__llvm_bitcast_pbool_to_pbool -> pbool`
  - `__llvm_bitcast_pbool_to_pi8 -> pi8`
  - `__llvm_bitcast_pbool_to_pi16 -> pi16`
  - `__llvm_bitcast_pbool_to_pi32 -> pi32`
  - `__llvm_bitcast_pbool_to_pi64 -> pi64`
  - (\*) `__llvm_bitcast_pbool_to_pi128 -> pi128`
  - `__llvm_bitcast_pi8_to_pbool -> pbool`
  - `__llvm_bitcast_pi8_to_pi8 -> pi8`
  - `__llvm_bitcast_pi8_to_pi16 -> pi16`
  - `__llvm_bitcast_pi8_to_pi32 -> pi32`
  - `__llvm_bitcast_pi8_to_pi64 -> pi64`
  - (\*) `__llvm_bitcast_pi8_to_pi128 -> pi128`
  - `__llvm_bitcast_pi16_to_pbool -> pbool`
  - `__llvm_bitcast_pi16_to_pi8 -> pi8`
  - `__llvm_bitcast_pi16_to_pi16 -> pi16`
  - `__llvm_bitcast_pi16_to_pi32 -> pi32`
  - `__llvm_bitcast_pi16_to_pi64 -> pi64`
  - (\*) `__llvm_bitcast_pi16_to_pi128 -> pi128`
  - `__llvm_bitcast_pi32_to_pbool -> pbool`
  - `__llvm_bitcast_pi32_to_pi8 -> pi8`
  - `__llvm_bitcast_pi32_to_pi16 -> pi16`
  - `__llvm_bitcast_pi32_to_pi32 -> pi32`
  - `__llvm_bitcast_pi32_to_pi64 -> pi64`
  - (\*) `__llvm_bitcast_pi32_to_pi128 -> pi128`
  - `__llvm_bitcast_pi64_to_pbool -> pbool`
  - `__llvm_bitcast_pi64_to_pi8 -> pi8`
  - `__llvm_bitcast_pi64_to_pi16 -> pi16`
  - `__llvm_bitcast_pi64_to_pi32 -> pi32`
  - `__llvm_bitcast_pi64_to_pi64 -> pi64`
  - (\*) `__llvm_bitcast_pi64_to_pi128 -> pi128`
  - (\*) `__llvm_bitcast_pi128_to_pbool -> pbool`
  - (\*) `__llvm_bitcast_pi128_to_pi8 -> pi8`
  - (\*) `__llvm_bitcast_pi128_to_pi16 -> pi16`
  - (\*) `__llvm_bitcast_pi128_to_pi32 -> pi32`
  - (\*) `__llvm_bitcast_pi128_to_pi64 -> pi64`
  - (\*) `__llvm_bitcast_pi128_to_pi128 -> pi128`
- [`icmp`](https://llvm.org/docs/LangRef.html#icmp-instruction):
  - `icmp` accepts three arguments. The first is a comparison condition. The two others are operands
    of the same type. The condition is defined as an enum consisting of these values: `eq`, `ne`,
    `ugt`, `uge`, `ult`, `ule`, `sgt`, `sge`, `slt`, `sle`.
  - `__llvm_icmp_cond_bool_bool -> bool`,
  - `__llvm_icmp_cond_i8_i8 -> bool`,
  - `__llvm_icmp_cond_i16_i16 -> bool`,
  - `__llvm_icmp_cond_i32_i32 -> bool`,
  - `__llvm_icmp_cond_i64_i64 -> bool`,
  - (\*) `__llvm_icmp_cond_i128_i128 -> bool`,
- [`select`](https://llvm.org/docs/LangRef.html#select-instruction):
  - `__llvm_select_bool_bool_bool -> bool`,
  - `__llvm_select_bool_i8_i8 -> bool`,
  - `__llvm_select_bool_i16_i16 -> bool`,
  - `__llvm_select_bool_i32_i32 -> bool`,
  - `__llvm_select_bool_i64_i64 -> bool`,
  - (\*) `__llvm_select_bool_i128_i128 -> bool`,

#### Based on Intrinsics

- [`llvm.abs.*`](https://llvm.org/docs/LangRef.html#llvm-abs-intrinsic):
  - `__llvm_abs_i8 -> i8`,
  - `__llvm_abs_i16 -> i16`,
  - `__llvm_abs_i32 -> i32`,
  - `__llvm_abs_i64 -> i64`,
  - (\*) `__llvm_abs_i128 -> i128`,
- [`llvm.smax.*`](https://llvm.org/docs/LangRef.html#llvm-smax-intrinsic):
  - `__llvm_smax_bool_bool -> bool`,
  - `__llvm_smax_i8_i8 -> i8`,
  - `__llvm_smax_i16_i8 -> i16`,
  - `__llvm_smax_i32_i8 -> i32`,
  - `__llvm_smax_i64_i8 -> i64`,
  - (\*) `__llvm_smax_i128_i8 -> i128`,
- [`llvm.smin.*`](https://llvm.org/docs/LangRef.html#llvm-smin-intrinsic):
  - `__llvm_smin_bool_bool -> bool`,
  - `__llvm_smin_i8_i8 -> i8`,
  - `__llvm_smin_i16_i8 -> i16`,
  - `__llvm_smin_i32_i8 -> i32`,
  - `__llvm_smin_i64_i8 -> i64`,
  - (\*) `__llvm_smin_i128_i8 -> i128`,
- [`llvm_umax.*`](https://llvm.org/docs/LangRef.html#llvm-umax-intrinsic):
  - `__llvm_umax_bool_bool -> bool`,
  - `__llvm_umax_i8_i8 -> i8`,
  - `__llvm_umax_i16_i8 -> i16`,
  - `__llvm_umax_i32_i8 -> i32`,
  - `__llvm_umax_i64_i8 -> i64`,
  - (\*) `__llvm_umax_i128_i8 -> i128`,
- [`llvm.umin.*`](https://llvm.org/docs/LangRef.html#llvm-umin-intrinsic):
  - `__llvm_umin_bool_bool -> bool`,
  - `__llvm_umin_i8_i8 -> i8`,
  - `__llvm_umin_i16_i8 -> i16`,
  - `__llvm_umin_i32_i8 -> i32`,
  - `__llvm_umin_i64_i8 -> i64`,
  - (\*) `__llvm_umin_i128_i8 -> i128`,
- [`llvm.scmp.*`](https://llvm.org/docs/LangRef.html#llvm-scmp-intrinsic):
  - As per the LLVM Language Reference Manual, `scmp` returns needs to return at least `i2`. Since
    ALU does not operate on such type, the closest possible type is `i8`. Luckily,
    [this is what LLVM generates](https://blog.llvm.org/posts/2024-08-29-gsoc-three-way-comparison/)
    for these intrinsics, therefore our implementations will follow this pattern.
  - `__llvm_ucmp_bool_bool -> i8`,
  - `__llvm_scmp_i8_i8 -> i8`,
  - `__llvm_scmp_i16_i8 -> i8`,
  - `__llvm_scmp_i32_i8 -> i8`,
  - `__llvm_scmp_i64_i8 -> i8`,
  - (\*) `__llvm_scmp_i128_i8 -> i8`,
- [`llvm.ucmp.*`](https://llvm.org/docs/LangRef.html#llvm-ucmp-intrinsic):
  - As per the LLVM Language Reference Manual, `ucmp` returns needs to return at least `i2`. Since
    ALU does not operate on such type, the closest possible type is `i8`. Luckily,
    [this is what LLVM generates](https://blog.llvm.org/posts/2024-08-29-gsoc-three-way-comparison/)
    for these intrinsics, therefore our implementations will follow this pattern.
  - `__llvm_ucmp_bool_bool -> i8`,
  - `__llvm_ucmp_i8_i8 -> i8`,
  - `__llvm_ucmp_i16_i8 -> i8`,
  - `__llvm_ucmp_i32_i8 -> i8`,
  - `__llvm_ucmp_i64_i8 -> i8`,
  - (\*) `__llvm_ucmp_i128_i8 -> i8`,
- [`llvm.bitreverse.*`]https://llvm.org/docs/LangRef.html#llvm-bitreverse-intrinsics):
  - `__llvm_bitreverse_bool -> bool`,
  - `__llvm_bitreverse_i8 -> i8`,
  - `__llvm_bitreverse_i16 -> i16`,
  - `__llvm_bitreverse_i32 -> i32`,
  - `__llvm_bitreverse_i64 -> i64`,
  - (\*) `__llvm_bitreverse_i128 -> i128`,
- [`llvm.bswap.*`](https://llvm.org/docs/LangRef.html#llvm-bswap-intrinsics):
  - `__llvm_bswap_i8 -> i8`,
  - `__llvm_bswap_i16 -> i16`,
  - `__llvm_bswap_i32 -> i32`,
  - `__llvm_bswap_i64 -> i64`,
  - (\*) `__llvm_bswap_i128 -> i128`,
- [`llvm.ctpop.*`](https://llvm.org/docs/LangRef.html#llvm-ctpop-intrinsics):
  - `__llvm_ctpop_bool -> bool`,
  - `__llvm_ctpop_i8 -> i8`,
  - `__llvm_ctpop_i16 -> i16`,
  - `__llvm_ctpop_i32 -> i32`,
  - `__llvm_ctpop_i64 -> i64`,
  - (\*) `__llvm_ctpop_i128 -> i128`,
- [`llvm.ctlz.*`](https://llvm.org/docs/LangRef.html#llvm-ctlz-intrinsics):
  - `__llvm_ctlz_bool -> bool`,
  - `__llvm_ctlz_i8 -> i8`,
  - `__llvm_ctlz_i16 -> i16`,
  - `__llvm_ctlz_i32 -> i32`,
  - `__llvm_ctlz_i64 -> i64`,
  - `__llvm_ctlz_i128 -> i128`,
- [`llvm.cttz.*`](https://llvm.org/docs/LangRef.html#llvm-cttz-intrinsics):
  - `__llvm_cttz_bool -> bool`,
  - `__llvm_cttz_i8 -> i8`,
  - `__llvm_cttz_i16 -> i16`,
  - `__llvm_cttz_i32 -> i32`,
  - `__llvm_cttz_i64 -> i64`,
  - (\*) `__llvm_cttz_i128 -> i128`,
- [`llvm.fshl.*`](https://llvm.org/docs/LangRef.html#llvm-fshl-intrinsics):
  - `__llvm_fshl_i8_i8_i8 -> i8`,
  - `__llvm_fshl_i16_i16_i16 -> i16`,
  - `__llvm_fshl_i32_i32_i32 -> i32`,
  - `__llvm_fshl_i64_i64_i64 -> i64`,
  - (\*) `__llvm_fshl_i128_i128_i128 -> i128`,
- [`llvm.fshr.*`](https://llvm.org/docs/LangRef.html#llvm-fshr-intrinsics):
  - `__llvm_fshr_i8_i8_i8 -> i8`,
  - `__llvm_fshr_i16_i16_i16 -> i16`,
  - `__llvm_fshr_i32_i32_i32 -> i32`,
  - `__llvm_fshr_i64_i64_i64 -> i64`,
  - (\*) `__llvm_fshr_i128_i128_i128 -> i128`,
- [`llvm.sadd.with.overflow.*`](https://llvm.org/docs/LangRef.html#llvm-sadd-with-overflow-intrinsics):
  - `__llvm_sadd_with_overflow_i8_i8 -> (i8, bool)`,
  - `__llvm_sadd_with_overflow_i16_i16 -> (i16, bool)`,
  - `__llvm_sadd_with_overflow_i32_i32 -> (i32, bool)`,
  - `__llvm_sadd_with_overflow_i64_i64 -> (i64, bool)`,
  - (\*) `__llvm_sadd_with_overflow_i128_i128 -> (i128, bool)`,
- [`llvm.uadd.with.overflow.*`](https://llvm.org/docs/LangRef.html#llvm-uadd-with-overflow-intrinsics):
  - `__llvm_uadd_with_overflow_i8_i8 -> (i8, bool)`,
  - `__llvm_uadd_with_overflow_i16_i16 -> (i16, bool)`,
  - `__llvm_uadd_with_overflow_i32_i32 -> (i32, bool)`,
  - `__llvm_uadd_with_overflow_i64_i64 -> (i64, bool)`,
  - (\*) `__llvm_uadd_with_overflow_i128_i128 -> (i128, bool)`,
- [`llvm.ssub.with.overflow.*`](https://llvm.org/docs/LangRef.html#llvm-ssub-with-overflow-intrinsics):
  - `__llvm_ssub_with_overflow_i8_i8 -> (i8, bool)`,
  - `__llvm_ssub_with_overflow_i16_i16 -> (i16, bool)`,
  - `__llvm_ssub_with_overflow_i32_i32 -> (i32, bool)`,
  - `__llvm_ssub_with_overflow_i64_i64 -> (i64, bool)`,
  - (\*) `__llvm_ssub_with_overflow_i128_i128 -> (i128, bool)`,
- [`llvm.usub.with.overflow.*`](https://llvm.org/docs/LangRef.html#llvm-usub-with-overflow-intrinsics):
  - `__llvm_usub_with_overflow_i8_i8 -> (i8, bool)`,
  - `__llvm_usub_with_overflow_i16_i16 -> (i16, bool)`,
  - `__llvm_usub_with_overflow_i32_i32 -> (i32, bool)`,
  - `__llvm_usub_with_overflow_i64_i64 -> (i64, bool)`,
  - (\*) `__llvm_usub_with_overflow_i128_i128 -> (i128, bool)`,
- [`llvm.smul.with.overflow.*`](https://llvm.org/docs/LangRef.html#llvm-smul-with-overflow-intrinsics):
  - `__llvm_smul_with_overflow_i8_i8 -> (i8, bool)`,
  - `__llvm_smul_with_overflow_i16_i16 -> (i16, bool)`,
  - `__llvm_smul_with_overflow_i32_i32 -> (i32, bool)`,
  - `__llvm_smul_with_overflow_i64_i64 -> (i64, bool)`,
  - (\*) `__llvm_smul_with_overflow_i128_i128 -> (i128, bool)`,
- [`llvm.umul.with.overflow.*`](https://llvm.org/docs/LangRef.html#llvm-umul-with-overflow-intrinsics):
  - `__llvm_umul_with_overflow_i8_i8 -> (i8, bool)`,
  - `__llvm_umul_with_overflow_i16_i16 -> (i16, bool)`,
  - `__llvm_umul_with_overflow_i32_i32 -> (i32, bool)`,
  - `__llvm_umul_with_overflow_i64_i64 -> (i64, bool)`,
  - (\*) `__llvm_umul_with_overflow_i128_i128 -> (i128, bool)`,
- [`llvm.sadd.sat.*`](https://llvm.org/docs/LangRef.html#llvm-sadd-sat-intrinsics):
  - `__llvm_sadd_sat_i8_i8 -> i8`,
  - `__llvm_sadd_sat_i16_i16 -> i16`,
  - `__llvm_sadd_sat_i32_i32 -> i32`,
  - `__llvm_sadd_sat_i64_i64 -> i64`,
  - (\*) `__llvm_sadd_sat_i128_i128 -> i128`,
- [`llvm.uadd.sat.*`](https://llvm.org/docs/LangRef.html#llvm-uadd-sat-intrinsics):
  - `__llvm_uadd_sat_i8_i8 -> i8`,
  - `__llvm_uadd_sat_i16_i16 -> i16`,
  - `__llvm_uadd_sat_i32_i32 -> i32`,
  - `__llvm_uadd_sat_i64_i64 -> i64`,
  - (\*) `__llvm_uadd_sat_i128_i128 -> i128`,
- [`llvm.ssub.sat.*`](https://llvm.org/docs/LangRef.html#llvm-ssub-sat-intrinsics):
  - `__llvm_ssub_sat_i8_i8 -> i8`,
  - `__llvm_ssub_sat_i16_i16 -> i16`,
  - `__llvm_ssub_sat_i32_i32 -> i32`,
  - `__llvm_ssub_sat_i64_i64 -> i64`,
  - (\*) `__llvm_ssub_sat_i128_i128 -> i128`,
- [`llvm.usub.sat.*`](https://llvm.org/docs/LangRef.html#llvm-usub-sat-intrinsics):
  - `__llvm_usub_sat_i8_i8 -> i8`,
  - `__llvm_usub_sat_i16_i16 -> i16`,
  - `__llvm_usub_sat_i32_i32 -> i32`,
  - `__llvm_usub_sat_i64_i64 -> i64`,
  - (\*) `__llvm_usub_sat_i128_i128 -> i128`,
- [`llvm.sshl.sat.*`](https://llvm.org/docs/LangRef.html#llvm-sshl-sat-intrinsics):
  - `__llvm_sshl_sat_i8_i8 -> i8`,
  - `__llvm_sshl_sat_i16_i16 -> i16`,
  - `__llvm_sshl_sat_i32_i32 -> i32`,
  - `__llvm_sshl_sat_i64_i64 -> i64`,
  - (\*) `__llvm_sshl_sat_i128_i128 -> i128`,
- [`llvm.ushl.sat.*`](https://llvm.org/docs/LangRef.html#llvm-ushl-sat-intrinsics):
  - `__llvm_ushl_sat_i8_i8 -> i8`,
  - `__llvm_ushl_sat_i16_i16 -> i16`,
  - `__llvm_ushl_sat_i32_i32 -> i32`,
  - `__llvm_ushl_sat_i64_i64 -> i64`,
  - (\*) `__llvm_ushl_sat_i128_i128 -> i128`,
