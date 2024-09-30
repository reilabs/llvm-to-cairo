//! This pass is responsible for generating a map of the top-level structure of
//! an LLVM IR module as described by an `.ll` file. This is used for
//! consistency checking during the compilation step.

use ltc_errors::compile::Result;

use crate::compile::{
    pass::{
        data::{ConcretePassData, DynPassDataMap, PassDataOps},
        ConcretePass,
        DynamicPassReturnData,
        Pass,
        PassKey,
        PassOps,
    },
    source::SourceContext,
};

/// Generates a map of the top-level structure of an LLVM module.
#[derive(Clone, Debug, PartialEq)]
pub struct ModuleMap {
    depends:     Vec<PassKey>,
    invalidates: Vec<PassKey>,
}

impl ModuleMap {
    /// Creates a new instance of the module mapping pass.
    #[must_use]
    pub fn new() -> Box<Self> {
        let depends = vec![];
        let invalidates = vec![];
        Box::new(Self {
            depends,
            invalidates,
        })
    }
}

impl PassOps for ModuleMap {
    fn run(
        &mut self,
        context: SourceContext,
        _pass_data: &DynPassDataMap,
    ) -> Result<DynamicPassReturnData> {
        Ok(DynamicPassReturnData::new(context, ModuleMapData::new()))
    }

    fn depends(&self) -> &[PassKey] {
        self.depends.as_slice()
    }

    fn invalidates(&self) -> &[PassKey] {
        self.invalidates.as_slice()
    }

    fn dupe(&self) -> Pass {
        Box::new(self.clone())
    }
}

impl ConcretePass for ModuleMap {
    type Data = ModuleMapData;
}

/// The module map that results from executing this analysis pass on an LLVM IR
/// module.
#[derive(Clone, Debug, PartialEq)]
pub struct ModuleMapData {}

impl ModuleMapData {
    /// Creates a new instance of the output data for the module mapping pass.
    #[must_use]
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }
}

impl PassDataOps for ModuleMapData {}
impl ConcretePassData for ModuleMapData {
    type Pass = ModuleMap;
}
