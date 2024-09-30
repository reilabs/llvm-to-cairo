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

pub mod pass;
pub mod source;

use ltc_errors::compile::Result;

use crate::{
    compile::{
        pass::{data::DynPassDataMap, PassManager, PassManagerReturnData},
        source::SourceContext,
    },
    polyfill::PolyfillMap,
};

/// The compiler is responsible for processing the LLVM IR input to generate a
/// `FlatLowered` output.
#[allow(dead_code)]
pub struct Compiler {
    /// The source context, containing references to the LLVM module to be
    /// compiled.
    context: SourceContext,

    /// The passes that this compiler is configured to run.
    passes: PassManager,

    /// The mapping between LLVM names and polyfill names for the compiler to
    /// use during compilation.
    polyfill_map: PolyfillMap,
}

impl Compiler {
    /// Constructs a new compiler instance, wrapping the provided `context`
    /// describing the LLVM module to compile, the `passes` to run, and the
    /// `polyfill_map` from LLVM names to polyfill names.
    fn new(context: SourceContext, passes: PassManager, polyfill_map: PolyfillMap) -> Self {
        Self {
            context,
            passes,
            polyfill_map,
        }
    }

    /// Executes the compiler on the configured LLVM module.
    ///
    /// Note that this invokes a state transition that leaves the compiler in an
    /// invalid state, and hence it consumes the compiler to prevent API misuse.
    ///
    /// # Errors
    ///
    /// - [`ltc_errors::compile::Error`] if the compilation process fails for
    ///   any reason.
    pub fn run(mut self) -> Result<CompilationResult> {
        let PassManagerReturnData {
            context: _context,
            data,
        } = self.passes.run(self.context)?;

        // TODO (#24) Actually compile to FLIR.

        Ok(CompilationResult::new(data))
    }
}

/// The result of compiling an LLVM IR module.
#[derive(Debug)]
pub struct CompilationResult {
    /// The final state of the pass data after the compiler passes have been
    /// executed.
    pub pass_results: DynPassDataMap,

    /// The `FLIR` module that results from compilation.
    pub result_module: (),
}

impl CompilationResult {
    /// Constructs a new compilation result wrapping the final `FLIR` module
    /// and also containing the final output of any compiler passes.
    #[must_use]
    pub fn new(pass_results: DynPassDataMap) -> Self {
        let result_module = ();
        Self {
            pass_results,
            result_module,
        }
    }
}

/// Allows for building a [`Compiler`] instance while retaining the defaults for
/// fields that do not need to be customized.
pub struct CompilerBuilder {
    /// The source context, containing references to the LLVM module to be
    /// compiled.
    context: SourceContext,

    /// The passes that this compiler is configured to run.
    passes: Option<PassManager>,

    /// The mapping between LLVM names and polyfill names for the compiler to
    /// use during compilation.
    polyfill_map: Option<PolyfillMap>,
}

impl CompilerBuilder {
    /// Creates a new compiler builder wrapping the provided context.
    ///
    /// The compiler's passes configuration and polyfill configuration will be
    /// left as default.
    #[must_use]
    pub fn new(context: SourceContext) -> Self {
        let passes = None;
        let polyfill_map = None;
        Self {
            context,
            passes,
            polyfill_map,
        }
    }

    /// Specifies the pass configuration for the compiler.
    #[must_use]
    pub fn with_passes(mut self, pass_manager: PassManager) -> Self {
        self.passes = Some(pass_manager);
        self
    }

    /// Specifies the polyfill configuration for the compiler.
    #[must_use]
    pub fn with_polyfills(mut self, polyfill_map: PolyfillMap) -> Self {
        self.polyfill_map = Some(polyfill_map);
        self
    }

    /// Builds a compiler from the specified configuration.
    #[must_use]
    pub fn build(self) -> Compiler {
        Compiler::new(
            self.context,
            self.passes.unwrap_or_default(),
            self.polyfill_map.unwrap_or_default(),
        )
    }
}
