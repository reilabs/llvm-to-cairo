//! This module contains the output structures for the code generation process.

use hieratika_flo::FlatLoweredObject;

/// The data store for the in-progress work of the code generator.
///
/// This is intended to be handed as a mutable reference through each of the
/// steps of the code generation process, and modified by each step.
#[derive(Debug)]
pub struct CodegenData {
    /// The underlying FLO that is being generated as part of the code
    /// generation process.
    ///
    /// Please note that at any point this object may be in an **invalid
    /// state**, as this is allowed to aid compilation. The consumer of the API
    /// is responsible for ensuring that the FLO is in a coherent or valid state
    /// _before_ finalizing its generation.
    flat_lowered_object: FlatLoweredObject,
}

impl CodegenData {
    /// Constructs a new code generator data store for the module with the
    /// provided name.
    pub fn new(name: &str) -> Self {
        let flat_lowered_object = FlatLoweredObject::new(name);

        Self {
            flat_lowered_object,
        }
    }

    /// Gets an immutable reference to the underlying FLO being built.
    pub fn flo(&self) -> &FlatLoweredObject {
        &self.flat_lowered_object
    }

    /// Gets a mutable reference to the underlying FLO being built.
    pub fn flo_mut(&mut self) -> &mut FlatLoweredObject {
        &mut self.flat_lowered_object
    }
}

impl From<CodegenData> for FlatLoweredObject {
    fn from(value: CodegenData) -> Self {
        value.flat_lowered_object
    }
}
