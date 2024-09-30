//! Handles the compilation of LLVM IR to Cairo's internal `FlatLowered` IR.
//!
//! In the context of LLVM to Cairo, compilation refers to the process of
//! translating from [LLVM IR](https://llvm.org/docs/LangRef.html) to Cairo's
//! internal
//! [`FlatLowered`](https://github.com/starkware-libs/cairo/blob/main/crates/cairo-lang-lowering/src/objects.rs#L135)
//! structure.
//!
//! LLVM IR is designed around a virtual processor model that is expected to
//! have a multitude of operations common to real CPUs. As we are compiling to
//! target the Cairo VM, we have to work out how to take each of these
//! operations, and represent them in our extremely restricted instruction set.
//!
//! Doing this involves two major approaches:
//!
//! 1. **Translation:** Where there is a good match between the structure of the
//!    LLVM IR and the structure of `FlatLowered`, we can translate one to the
//!    other. This is useful both in terms of code structure—as LLVM IR is still
//!    a structured IR—and in terms of basic operations that are common to both
//!    representations.
//! 2. **Polyfills:** Where LLVM expects an operation that we do not have an
//!    equivalent for, we instead emit a call to an _implementation of that
//!    operation_ in Cairo. We term these implementations _polyfills_ as an
//!    analogy to the term used on the web, and they are _software_
//!    implementations of features and capabilities that our hardware is
//!    missing. For more information on polyfills, see the [`crate::polyfill`]
//!    module.
//!
//! We aim for this compilation process to both achieve a 1:1 semantic match to
//! the original LLVM IR—through use of translation and polyfills as needed—and
//! to retain as much context information as possible so to ensure the
//! possibility of a good user experience in the future.
//!
//! # Targeting `FlatLowered` instead of `Sierra`
//!
//! It might seem strange to target `FlatLowered` instead of something like
//! [Sierra](https://docs.starknet.io/architecture-and-concepts/smart-contracts/cairo-and-sierra/#why_do_we_need_sierra)
//! which is _intended_ as a target for compilation.
//!
//! While we definitely want the benefits of Sierra—particularly model checking
//! for the underlying machine, and the gas monitoring—we do not want to perform
//! all the necessary bookkeeping to make Sierra work on our own. By targeting
//! `FlatLowered` instead, we gain the benefits of the _already existing_
//! [`sierragen`](https://github.com/starkware-libs/cairo/blob/main/crates/cairo-lang-sierra-generator/src/lib.rs)
//! functionality, which ingests `FlatLowered` and handles the required Sierra
//! bookkeeping for us.
//!
//! While this does give us less control—as we rely on the existing
//! translation—the benefits of not having to manually perform this additional
//! work far outweighs that downside. If we _do_ need any additional control, we
//! can always modify this process at a later date.

#[cfg(test)]
mod test {}
