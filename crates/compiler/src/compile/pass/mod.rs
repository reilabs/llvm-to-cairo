//! This module contains both the definition of the [`Pass`] type and the
//! [`PassManager`] object.
//!
//! Every Pass should implement [`ConcretePass`], as this provides the full set
//! of features required of a pass. It is also expected that each pass provide a
//! type-specific constructor function called `new`.
//!
//! This compiler will take advantage of passes defined in LLVM—that we can use
//! via [`inkwell`]—and also custom passes tailored to the `CairoVM` CPU model
//! that may depend on LLVM-defined analyses.
//!
//! # Passes
//!
//! A pass is a self-contained unit of functionality that performs some
//! operation over the LLVM IR. They tend to fall into the following three
//! categories:
//!
//! - **Analysis:** These examine the structure of the IR to infer information
//!   about it without changing its structure. The information produced by
//!   analysis passes can be invalidated by transformation passes.
//! - **Transformation:** Transformation passes use either information from
//!   analysis passes or structural information about the IR to change the
//!   structure of the IR. These structural changes can happen for many reasons,
//!   but usually involve optimizing some metric (e.g. runtime or code size).
//! - **Utility:** These passes do not fall neatly into either of the above
//!   categories.
//!
//! # Note: Skeleton
//!
//! The implementations in this file are deliberately left incomplete, and exist
//! only as skeletons to serve the purposes of correctly designing the compiler
//! state. A proper implementation will take place later in the project, as
//! tracked by [#56](https://github.com/reilabs/llvm-to-cairo/issues/56).

pub mod analysis;
pub mod data;

use std::{
    any::{Any, TypeId},
    fmt::Debug,
};

use derivative::Derivative;
use downcast_rs::Downcast;
use ltc_errors::compile::{Error, Result};

use crate::compile::{
    pass::data::{ConcretePassData, DynPassDataMap, PassData},
    source::SourceContext,
};

/// A pass is a self-contained unit of functionality that performs some
/// operation over the LLVM IR.
pub type Pass = Box<dyn PassOps>;

/// A handle that uniquely identifies the pass.
pub type PassKey = TypeId;

/// Pass return data that returns a dynamic [`PassData`].
pub type DynPassReturnData = PassReturnData<PassData>;

/// The data returned when executing a pass.
#[derive(Derivative)]
#[derivative(Debug(bound = "T: Debug"))]
pub struct PassReturnData<T> {
    /// The newly-modified source context.
    pub source_context: SourceContext,

    /// The data returned by the pass.
    pub data: T,
}
impl<T> PassReturnData<T> {
    /// Creates a new instance of the pass return data.
    pub fn new(source_context: SourceContext, data: T) -> Self {
        Self {
            source_context,
            data,
        }
    }
}

impl PassReturnData<PassData> {
    /// Allows you to get the returned pass data as the concrete data type `T`,
    /// returning `&T` if possible and `None` otherwise.
    #[must_use]
    pub fn data_as<T: ConcretePassData>(&self) -> Option<&T> {
        self.data.as_any().downcast_ref::<T>()
    }

    /// Allows you to get the returned pass data as the concrete data type `T`,
    /// returning `&T` if possible and `None` otherwise.
    pub fn data_as_mut<T: ConcretePassData>(&mut self) -> Option<&mut T> {
        self.data.as_any_mut().downcast_mut::<T>()
    }

    /// Allows you to get the returned pass data as the concrete data type `T`,
    /// returning `&T` if possible.
    ///
    /// # Panics
    ///
    /// If `self.data` is not an instance of `T`.
    #[must_use]
    pub fn unwrap_data_as<T: ConcretePassData>(&self) -> &T {
        self.data_as::<T>().unwrap()
    }

    /// Allows you to get the returned pass data as the concrete data type `T`,
    /// returning `&mut T` if possible.
    ///
    /// # Panics
    ///
    /// If `self.data` is not an instance of `T`.
    pub fn unwrap_data_as_mut<T: ConcretePassData>(&mut self) -> &mut T {
        self.data_as_mut::<T>().unwrap()
    }
}

/// The data returned when executing a pass where the pass data is of a
/// dynamically-dispatched type.
pub type DynamicPassReturnData = PassReturnData<PassData>;

/// The operations that we expect one of our passes to have.
///
/// The implementation is designed te be used via dynamic dispatch, and hence
/// can provide the requisite operations however it is able.
///
/// # Self Bounds
///
/// The bounds on `Self` are required by these traits for the following reasons:
///
/// - [`Any`] allows downcasting to concrete implementations of `Opcode` if
///   needed.
/// - [`Debug`] to provide representations to aid in debugging. It is
///   recommended to use the derive feature for this.
/// - [`Downcast`] for easy conversions _to_ [`Any`] for downcasting.
///
/// In addition, it is required but not enforced that implementors of this
/// trait also implement [`ConcretePass`].
pub trait PassOps
where
    Self: Any + Debug + Downcast,
{
    /// Executes the pass on the provided `context`, returning both the
    /// potentially-modified context and any data returned by the pass.
    ///
    /// It takes a map of `pass_data` that allows the running pass to get at the
    /// data required by
    ///
    /// # Errors
    ///
    /// - [`Error`] if pass execution fails for any reason.
    fn run(
        &mut self,
        context: SourceContext,
        pass_data: &DynPassDataMap,
    ) -> Result<DynamicPassReturnData>;

    /// Gets a slice containing the keys of the passes whose output this pass
    /// depends on.
    fn depends(&self) -> &[PassKey];

    /// Gets a slice containing the keys of the passes
    fn invalidates(&self) -> &[PassKey];

    /// Returns a duplicate of this pass.
    fn dupe(&self) -> Pass;

    /// Gets a key that uniquely represents the pass.
    ///
    /// This **must** return the same value as [`ConcretePass::key`].
    fn key_dyn(&self) -> PassKey {
        self.type_id()
    }
}

/// Operations implemented on `dyn PassOps` are **only** available on the
/// concrete trait object and hence not equivalent to a blanket implementation
/// of a method for `trait PassOps`.
impl dyn PassOps {
    /// Checks if the pass is an instance of the concrete pass `T`, returning
    /// `true` if it is and `false` otherwise.
    pub fn is<T: ConcretePass>(&self) -> bool {
        self.as_any().is::<T>()
    }

    /// Allows you to view the dynamic pass `self` as the concrete pass type
    /// `T`, returning a `&T` if possible and `None` otherwise.
    pub fn view_as<T: ConcretePass>(&self) -> Option<&T> {
        self.as_any().downcast_ref::<T>()
    }

    /// Allows you to view the dynamic pass `self` as the concrete pass type
    /// `T`, returning a `&mut T` if possible and `None` otherwise.
    pub fn view_as_mut<T: ConcretePass>(&mut self) -> Option<&mut T> {
        self.as_any_mut().downcast_mut::<T>()
    }

    /// Allows you to view the dynamic pass `self` as the concrete pass type
    /// `T`, returning a `&T` if possible.
    ///
    /// # Panics
    ///
    /// If `self` is not an instance of `T`.
    pub fn unwrap_as<T: ConcretePass>(&self) -> &T {
        self.view_as::<T>()
            .unwrap_or_else(|| panic!("self was not a {:?}", TypeId::of::<T>()))
    }

    /// Allows you to view the dynamic pass `self` as the concrete pass type
    /// `T`, returning a `&mut T` if possible.
    ///
    /// # Panics
    ///
    /// If `self` is not an instance of `T`.
    pub fn unwrap_as_mut<T: ConcretePass>(&mut self) -> &mut T {
        self.view_as_mut::<T>()
            .unwrap_or_else(|| panic!("self was not a {:?}", TypeId::of::<T>()))
    }
}

/// Provides extra operations that can be called when operating on a concrete
/// instance of a specific pass, rather than on any instance of a pass.
pub trait ConcretePass
where
    Self: Clone + Debug + PassOps,
{
    /// The type of data returned by the pass.
    type Data: ConcretePassData;

    /// Gets a key that uniquely represents the pass.
    ///
    /// This **must** return the same value as [`PassOps::key_dyn`].
    #[must_use]
    fn key() -> PassKey {
        TypeId::of::<Self>()
    }
}

/// The data returned when executing all passes via the pass manager.
#[derive(Debug)]
pub struct PassManagerReturnData {
    /// The newly-modified source context.
    pub context: SourceContext,

    /// A mapping from pass key to the data returned by the pass.
    pub data: DynPassDataMap,
}

impl PassManagerReturnData {
    /// Creates a new pass manager return data element wrapping the transformed
    /// source `context` and the result `data` from all the passes.
    #[must_use]
    pub fn new(context: SourceContext, data: DynPassDataMap) -> Self {
        Self { context, data }
    }
}

/// A manager for passes within the compiler.
///
/// The primary task of this pass manager is to automatically resolve a pass
/// ordering based on dependencies between passes. This ensures that pass
/// orderings are correct, without the need for costly manual validation.
pub struct PassManager {
    pass_ordering: Vec<Pass>,
}

impl PassManager {
    /// Creates a new pass manager wrapping the provided passes.
    ///
    /// # Errors
    ///
    /// - [`Error::InvalidPassOrdering`] if no valid pass ordering can be
    ///   generated from the provided `passes`.
    pub fn new(passes: Vec<Pass>) -> Result<Self> {
        let pass_ordering = Self::generate_pass_ordering(passes)?;
        Ok(Self { pass_ordering })
    }

    /// Executes the pass ordering on the provided `context`.
    ///
    /// # Errors
    ///
    /// - [`Error`] if any pass fails.
    pub fn run(&mut self, mut context: SourceContext) -> Result<PassManagerReturnData> {
        let mut pass_data_map = DynPassDataMap::new();

        for pass in &mut self.pass_ordering {
            let PassReturnData {
                source_context,
                data,
            } = pass.run(context, &pass_data_map)?;
            pass_data_map.put_dyn(pass, data);

            context = source_context;
        }

        let result = PassManagerReturnData::new(context, pass_data_map);
        Ok(result)
    }

    /// Gets the current pass ordering.
    ///
    /// This method is always guaranteed to return a valid pass ordering that
    /// respects the requirements of the passes.
    #[must_use]
    pub fn passes(&self) -> &[Pass] {
        &self.pass_ordering
    }

    /// Generates a valid pass ordering from `passes` wherever possible.
    ///
    /// # Errors
    ///
    /// - [`Error::InvalidPassOrdering`] if no valid pass ordering can be
    ///   generated from the provided `passes`. This will usually occur due to
    ///   circular dependencies between passes.
    pub fn generate_pass_ordering(passes: Vec<Pass>) -> Result<Vec<Pass>> {
        // TODO Actually implement this (#56). The current constraint is silly.
        let no_deps = passes.iter().all(|p| p.depends().is_empty());
        if no_deps {
            Ok(passes)
        } else {
            Err(Error::InvalidPassOrdering(
                "Passes had dependencies where they should not".to_string(),
            ))
        }
    }
}

impl Default for PassManager {
    /// Returns a pass manager with the default set of passes associated with
    /// it.
    ///
    /// # Default Passes
    ///
    /// The list of default passes is as follows. Please note that they will be
    /// assembled into a correct ordering, and will not necessarily be executed
    /// in the order in which they are presented here.
    ///
    /// - [`analysis::module_map::ModuleMap`]
    fn default() -> Self {
        Self::new(vec![analysis::module_map::ModuleMap::new()])
            .expect("Default pass ordering was invalid")
    }
}
