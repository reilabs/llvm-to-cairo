//! Unfortunately [`inkwell`] does not deal well with `metadata`-typed function
//! arguments, despite them being valid argument types for function-typed values
//! in LLVM IR. For now, we handle them by delegating to known signatures for
//! these functions, rather than trying to introspect the functions themselves.
//!
//! See [this issue](https://github.com/TheDan64/inkwell/issues/546) for more
//! information.

use std::collections::HashMap;

use inkwell::{module::Linkage, GlobalVisibility};

use crate::{
    llvm::{typesystem::LLVMType, TopLevelEntryKind},
    pass::analysis::module_map::FunctionInfo,
};

/// A registry of LLVM intrinsic functions that need to be handled specially.
#[derive(Clone, Debug, PartialEq)]
pub struct SpecialIntrinsics {
    /// The intrinsics that need to be handled specially.
    intrinsics: HashMap<String, FunctionInfo>,
}

impl SpecialIntrinsics {
    /// Constructs the special intrinsics mapping, providing the appropriate
    /// [`FunctionInfo`] metadata for the intrinsics that we insert.
    #[must_use]
    pub fn new() -> Self {
        let mut intrinsics = HashMap::new();
        intrinsics.insert(
            "llvm.dbg.declare".to_string(),
            FunctionInfo {
                kind:       TopLevelEntryKind::Declaration,
                intrinsic:  true,
                typ:        LLVMType::make_function(
                    LLVMType::void,
                    &[LLVMType::Metadata, LLVMType::Metadata, LLVMType::Metadata],
                ),
                linkage:    Linkage::External,
                visibility: GlobalVisibility::Default,
            },
        );
        intrinsics.insert(
            "llvm.dbg.value".to_string(),
            FunctionInfo {
                kind:       TopLevelEntryKind::Declaration,
                intrinsic:  true,
                typ:        LLVMType::make_function(
                    LLVMType::void,
                    &[LLVMType::Metadata, LLVMType::Metadata, LLVMType::Metadata],
                ),
                linkage:    Linkage::External,
                visibility: GlobalVisibility::Default,
            },
        );
        intrinsics.insert(
            "llvm.dbg.assign".to_string(),
            FunctionInfo {
                kind:       TopLevelEntryKind::Declaration,
                intrinsic:  true,
                typ:        LLVMType::make_function(
                    LLVMType::void,
                    &[
                        LLVMType::Metadata,
                        LLVMType::Metadata,
                        LLVMType::Metadata,
                        LLVMType::Metadata,
                        LLVMType::Metadata,
                    ],
                ),
                linkage:    Linkage::External,
                visibility: GlobalVisibility::Default,
            },
        );

        Self { intrinsics }
    }

    /// Gets the function information for `function_name` if it exists, and
    /// returns [`None`] otherwise.
    #[must_use]
    pub fn info_for(&self, function_name: &str) -> Option<FunctionInfo> {
        self.intrinsics.get(function_name).cloned()
    }

    /// Gets the function information for `function_name` if it exists.
    ///
    /// # Panics
    ///
    /// If `function_name` does not exist in the special intrinsics container.
    #[must_use]
    pub fn info_for_unchecked(&self, function_name: &str) -> FunctionInfo {
        self.info_for(function_name)
            .unwrap_or_else(|| panic!("No information found for {function_name}"))
    }
}

impl Default for SpecialIntrinsics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use crate::llvm::special_intrinsics::SpecialIntrinsics;

    #[test]
    fn contains_dbg_declare() {
        assert!(SpecialIntrinsics::new().intrinsics.contains_key("llvm.dbg.declare"));
    }

    #[test]
    fn contains_dbg_value() {
        assert!(SpecialIntrinsics::new().intrinsics.contains_key("llvm.dbg.value"));
    }

    #[test]
    fn contains_dbg_assign() {
        assert!(SpecialIntrinsics::new().intrinsics.contains_key("llvm.dbg.assign"));
    }
}
