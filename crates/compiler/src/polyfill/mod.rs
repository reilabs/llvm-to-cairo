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
//! compilation process (see [`crate::Compiler`]) can select from dynamically.
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

pub mod mappings;

use bimap::{BiHashMap, BiMap};

use crate::polyfill::mappings::LLVM_UADD_WITH_OVERFLOW_I64;

/// A bidirectional mapping from the builtin names for LLVM to the internal
/// names for the corresponding polyfills.
///
/// This exists in order to enable external linkage of symbols not part of the
/// current translation unit.
///
/// # LLVM Opcodes
///
/// Note that some LLVM opcodes (e.g. `add`) map to potentially multiple
/// implementations. For such opcodes, the expected LLVM name is given by the
/// [`Self::of_opcode`] function.
#[derive(Clone, Debug, PartialEq)]
pub struct PolyfillMap {
    /// A mapping from the LLVM-side names to the corresponding polyfill names.
    mapping: BiMap<String, String>,
}

impl PolyfillMap {
    /// Constructs a new polyfill map from the provided `mapping`.
    #[must_use]
    pub fn new(mapping: BiHashMap<String, String>) -> Self {
        Self { mapping }
    }

    /// Queries for the polyfill name that corresponds to the provided
    /// `llvm_name`, returning it if it exists or returning [`None`] otherwise.
    pub fn polyfill(&self, llvm_name: impl Into<String>) -> Option<&String> {
        self.mapping.get_by_left(&llvm_name.into())
    }

    /// Queries for the LLVM opcode (as modified by [`Self::of_opcode`]) that
    /// corresponds to the provided `polyfill_name`, returning it if it exists
    /// or returning [`None`] otherwise.
    pub fn llvm(&self, polyfill_name: impl Into<String>) -> Option<&String> {
        self.mapping.get_by_right(&polyfill_name.into())
    }

    /// Provides more information to assist in resolving the correct polyfill
    /// based on the types associated with the particular opcode invocation.
    ///
    /// Note that this is a purely _syntactic_ transformation, and does not
    /// account for type aliases and the like. Please ensure that any types are
    /// fully resolved before calling this.
    ///
    /// ```
    /// use ltc_compiler::polyfill::PolyfillMap;
    ///
    /// let opcode_name = "add";
    /// let arg_types = vec!["i8", "i64"];
    ///
    /// assert_eq!(
    ///     PolyfillMap::of_opcode(opcode_name, arg_types.as_slice()),
    ///     "__llvm_add_i8_i64"
    /// );
    /// ```
    #[must_use]
    pub fn of_opcode(opcode: &str, types: &[&str]) -> String {
        let types_str = if types.is_empty() {
            "void".to_string()
        } else {
            types.join("_")
        };
        format!("__llvm_{opcode}_{types_str}")
    }
}

impl Default for PolyfillMap {
    /// Contains the default mapping from opcodes and builtins to the
    /// corresponding polyfill names.
    fn default() -> Self {
        let defaults = [LLVM_UADD_WITH_OVERFLOW_I64];

        Self::new(
            defaults
                .into_iter()
                .map(|(l, r)| (l.to_string(), r.to_string()))
                .collect(),
        )
    }
}

#[cfg(test)]
mod test {
    use crate::polyfill::PolyfillMap;

    #[test]
    fn llvm_lookup_works() {
        let map = PolyfillMap::default();

        assert_eq!(
            map.llvm("__llvm_uadd_with_overflow_i64_i64").unwrap(),
            "llvm.uadd.with.overflow.i64"
        );
    }

    #[test]
    fn polyfill_lookup_works() {
        let map = PolyfillMap::default();

        assert_eq!(
            map.polyfill("llvm.uadd.with.overflow.i64").unwrap(),
            "__llvm_uadd_with_overflow_i64_i64"
        );
    }

    #[test]
    fn of_opcode_works() {
        let opcode_name = "my_opcode";
        let tys_1 = vec!["i8", "i64"];
        let tys_2 = vec!["i1"];
        let tys_3 = vec![];

        assert_eq!(
            PolyfillMap::of_opcode(opcode_name, tys_1.as_slice()),
            "__llvm_my_opcode_i8_i64"
        );
        assert_eq!(
            PolyfillMap::of_opcode(opcode_name, tys_2.as_slice()),
            "__llvm_my_opcode_i1"
        );
        assert_eq!(
            PolyfillMap::of_opcode(opcode_name, tys_3.as_slice()),
            "__llvm_my_opcode_void"
        );
    }
}
