# ALU design

This document describes the research done for the task #27 Design ALU. It aims to capture which
operations fall under the ALU umbrella, that are not already accounted for by the existing tasks to
implement polyfills.

The ALU will be implemented as a source code written in Cairo. During the
[LLVM-to-Cairo compilation pipeline](https://www.notion.so/reilabs/System-Architecture-113d2f80c874802b8480d997347933a2?pvs=4)
it will be translated to `FlatLowered` objects. Then, on the linking phase, arithmetic operations
from `FLIR` objects created from the input LLVM IR will be linked with their Cairo implementations.

## Interesting parts of LLVM IR

ALU will have to target two concepts: instructions and intrinsics.

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

- `<ty>` is type, e.g. `u32`,
- `nuw` - No Unsigned Wrap,
- `nsw` - No Signed Wrap.

#### Poison

In the example of `add`, if `nuw` or `nsw` keywords occur, they guarantee specific behavior, i.e. no
(un)signed overflow. However, if the operands cause the overflow, the instruction returns a poison,
which is an equivalent of a value indicating undefined behavior that can propagate throughout the
program.

According to @Ara's research LLVM does not seem to emit such instructions from the Rust code, so the
initial version of ALU will not handle `nuw` and `nsw` keywords in any specific way.

### Intrinsics

The example by @Ara above includes the following line:

```llvm
 %0 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %left, i64 %right), !dbg !17
```

There is no `add` instruction here. The adding operation is done by an intrinsic named
`llvm.uadd.with.overflow.i64` and called with the `call` instruction. The intrinsic exists somewhere
in LLVM and does not make its way into the `.ll` file produced out of the
`fn add(a: u64, b: u64) -> u64 { a+b }` Rust code.

Luckily, Langref has an extensive list of them. Here's the example of
[`llvm.uadd.with.overflow.<ty>`](https://llvm.org/docs/LangRef.html#llvm-uadd-with-overflow-intrinsics).

## Statefulness

A real Arithmetic-Logic Unit in a CPU is a finite state machine. Some states, interesting from the
programmer's point of view, can be captured as contents of the CPU registers. Such state is e.g. the
next instruction (as pointed to by Program Counter or its equivalent), values of operands stored in
two general purpose registers or the result of the last operation stored in another GP register and
a flag register, where specific bits signal certain conditions (e.g. the result being zero or an
integer overflow).

The LLVM-to-Cairo infrastructure needs to deliver pieces of code translating generic LLVM arithmetic
operations to their counterparts specific to the CairoVM architecture. This translation will be done
on the code level, during one of the LLVM-to-Cairo pipeline stages. Namely, this will be not
_runtime_ translation, but rather a _compilation time_ one. Therefore, there is no global _state_ to
manage during that time.

Additionally,
[it has been noticed by @Ara](https://github.com/reilabs/llvm-to-cairo/issues/27#issuecomment-2391893640),
that LLVM IR follows the same principle of not worrying about the internal state of arithmetic
operations, by making its more procedures to return a tuple containing both the operation result and
the state information:

```llvm
%0 = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %left, i64 %right), !dbg !17
%_3.0 = extractvalue { i64, i1 } %0, 0, !dbg !17
%_3.1 = extractvalue { i64, i1 } %0, 1, !dbg !17
br i1 %_3.1, label %panic, label %bb1, !dbg !17
```

Based on this operation we decide to deliver the ALU os a collection of stateless arithmetic
operations.

## Naming convention

As discussed with @Ara:

### Instruction polyfills

Name template: `__llvm_<opcode>_<ty1>_<ty2>`. In case the instruction works with both operands of
the same data type, the template degrades to `__llvm_<opcode>_<ty>_<ty>`.

In the above example of `add i32 %a, %b`, the polyfill would be named `__llvm_add_i32_i32`.

### Intrinsic polyfills

Name template: `__<actual name with _ instead of .>`.

In the above example of `llvm.uadd.with.overflow.i64`, the polyfil would be named
`__llvm_uadd_with_overflow_i64`.

## Tests

Cairo has an
[integrated test framework](https://book.cairo-lang.org/ch10-01-how-to-write-tests.html), similar to
the one offered by Rust. Our ALU implementation will then come with a test suite to verify that we
implement the desired behavior, i.e. indicate overflow on some obvious situations like
`0xFFFFFFFF + 1` for a U32 add.

## Other observations

### Type conversion

Cairo does not have Rust's `as` keyword, so it's not possible to do e.g. `let a = b as u32` given
`b` is a U64.

An equivalent operation in Cairo is `let a: u32: b.try_into().unwrap();`. This approach has two
disadvantages:

- it will panic if `b`'s value is larger than `0xFFFFFFFF`,
- there's no free wraparound as in the case of `as`.

We will need to have to handle the type conversion with pattern matching:

```rust
    let result: u32 = match sum.try_into() {
        Ok(val) => val,
        Err(_) => {
                  // Handle the wraparound manually
        }
    };
```
