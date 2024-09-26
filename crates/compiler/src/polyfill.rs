//! In the context of this project, a polyfill is an implementation of some
//! functionality that is _not_ supported by our target CPU in terms of
//! functionality that _is_ supported by our target.
//!
//! By way of example, consider that our CPU does not support floating point
//! arithmetic, so to compile LLVM code that uses such a thing we need to
//! implement it and call _our_ functions where it needs to perform these
//! operations.
//!
//! Our polyfill mechanism aims to be generic, such that we can implement and
//! improve our polyfills without requiring invasive changes to the code-base.
//! In order to do this, we have created a _library_ of polyfills that the
//! compilation process (see [`crate::compiler`]) can select from dynamically.
//!
//! # Polyfills and Optimization
//!
//! We are implementing our polyfills in Cairo-the-language, thereby enabling us
//! to have them in the same form as our compiled LLVM IR: `FlatLowered`. This
//! means that we can combine the polyfills and source into a compilation unit
//! seamlessly.
//!
//! While implementing these in Cairo means that they are amenable to rapid
//! iteration and experimentation, the polyfill is not the _end goal_ of this
//! process.
//!
//! 1. **Polyfills:** Implemented in Cairo, these implement functionality that
//!    our CPU is missing using functionality that it is not. They are slow in
//!    that they take more steps than the other options to perform an operation,
//!    but are much easier to experiment with and iterate on.
//! 2. **Builtins:** Builtins are units of functionality written in Rust that
//!    act as coprocessors using a DMA-like mechanism to receive operands and
//!    provide results back to the executing code. These are much faster to
//!    execute, taking few steps at most, but are more invasive to experiment
//!    with and change. They may also require more memory than an equivalent
//!    polyfill, which would increase the verification time.
//! 3. **AIR Instructions:** AIR instructions are the fastest option here, but
//!    adding a new instruction has the downside of increasing the width of the
//!    trace table. Any increase in table width increases the size of the table
//!    and also the time to prove the execution.
//!
//! Starting with the polyfills, however, allows us to experiment and iterate
//! rapidly to arrive at a design that we are happy with. This would be far more
//! complex for a builtin, and more complex still for an AIR instruction.
//!
//! Perhaps more importantly, the polyfills allow us to examine and profile to
//! find which operations will be most effective to "upgrade". Rather than a
//! scattershot approach based on hunches, the polyfills allow us to base these
//! decisions on real-world data.
//!
//! To that end, there are certainly polyfills that will still exist. It is very
//! unlikely that every single operation is beneficial to implement as a builtin
//! or AIR instruction.
