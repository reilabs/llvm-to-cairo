//! Pass data is data that results from the operation of some pass that _cannot_
//! be represented in the standard output of the pass.

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt::Debug,
};

use derivative::Derivative;
use downcast_rs::Downcast;

use crate::{
    context::SourceContext,
    pass::{ConcretePass, PassKey},
};

/// Pass data is output by any given pass
pub type PassData = Box<dyn PassDataOps>;

/// The operations that we expect one of our pass data objects to have.
///
/// The implementation is designed to be used via dynamic dispatch, and hence
/// can provide the requisite operations however it is able.
///
/// # Recommended Functions
///
/// On the concrete type that implements this trait, it is recommended to
/// implement:
///
/// - A `new(...) -> Self` associated function.
/// - A `new_dyn(...) -> PassData` associated function. This one can usually
///   simply call `Box::new(Self::new(...))`.
///
/// These aid in providing a uniform way to construct pass data.
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
/// trait also implement [`ConcretePassData`].
pub trait PassDataOps
where
    Self: Any + Debug + Downcast,
{
}

/// Operations implemented on `dyn PassDataOps` are **only** available on the
/// concrete trait object and hence not equivalent to a blanket implementation
/// of a method for `trait PassDataOps`.
impl dyn PassDataOps {
    /// Checks if the pass is an instance of the concrete pass `T`, returning
    /// `true` if it is and `false` otherwise.
    pub fn is<T: ConcretePassData>(&self) -> bool {
        self.as_any().is::<T>()
    }

    /// Allows you to view the dynamic pass `self` as the concrete pass type
    /// `T`, returning a `&T` if possible and `None` otherwise.
    pub fn view_as<T: ConcretePassData>(&self) -> Option<&T> {
        self.as_any().downcast_ref::<T>()
    }

    /// Allows you to view the dynamic pass `self` as the concrete pass type
    /// `T`, returning a `&mut T` if possible and `None` otherwise.
    pub fn view_as_mut<T: ConcretePassData>(&mut self) -> Option<&mut T> {
        self.as_any_mut().downcast_mut::<T>()
    }

    /// Allows you to view the dynamic pass `self` as the concrete pass type
    /// `T`, returning a `&T` if possible.
    ///
    /// # Panics
    ///
    /// If `self` is not `T`.
    pub fn unwrap_as<T: ConcretePassData>(&self) -> &T {
        self.view_as()
            .unwrap_or_else(|| panic!("self was not a {:?}", TypeId::of::<T>()))
    }

    /// Allows you to view the dynamic pass `self` as the concrete pass type
    /// `T`, returning a `&mut T` if possible.
    ///
    /// # Panics
    ///
    /// If `self` is not `T`.
    pub fn unwrap_as_mut<T: ConcretePassData>(&mut self) -> &mut T {
        self.view_as_mut()
            .unwrap_or_else(|| panic!("self was not a {:?}", TypeId::of::<T>()))
    }
}

/// Provides additional operations that can be called when operating on a
/// concrete instance of a specific pass, rather than any pass instance.
///
/// # Recommended Functions
///
/// On the concrete type that implements this trait, it is recommended to
/// implement:
///
/// - A `new(...) -> Self` associated function.
/// - A `new_dyn(...) -> PassData` associated function. This one can usually
///   simply call `Box::new(Self::new(...))`.
///
/// These aid in providing a uniform way to construct pass data.
pub trait ConcretePassData
where
    Self: Clone + Debug + PassDataOps,
{
    /// The pass with which the data is associated.
    type Pass: ConcretePass;
}

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

/// A mapping from pass keys to the associated pass data.
///
/// It will always contain the latest pass data, as there is no need to re-run a
/// pass unless it was invalidated by a subsequent pass.
pub type DynPassDataMap = PassDataMap<PassData>;

/// A mapping from pass keys to the associated pass data.
///
/// It will always contain the latest pass data, as there is no need to re-run a
/// pass unless it was invalidated by a subsequent pass.
#[derive(Derivative)]
#[derivative(
    Clone(bound = "T: Clone"),
    Debug(bound = "T: Debug"),
    PartialEq(bound = "T: PartialEq")
)]
pub struct PassDataMap<T> {
    /// The mapping from pass keys to pass data.
    mapping: HashMap<PassKey, T>,
}

impl<T> PassDataMap<T> {
    /// Constructs a new, empty, mapping from pass keys to pass data.
    #[must_use]
    pub fn new() -> Self {
        let mapping = HashMap::new();
        Self { mapping }
    }

    /// Clears all pass data.
    pub fn clear_all(&mut self) {
        self.mapping.clear();
    }

    /// Gets a reference to the last-written data for the pass given by the
    /// provided `key` if it exists. exists, and returns `None` otherwise.
    #[must_use]
    pub fn get_key(&self, key: PassKey) -> Option<&T> {
        self.mapping.get(&key)
    }

    /// Writes the provided `data` into the container associating it with the
    /// pass described by `key`, overwriting any existing data for that pass.
    pub fn put_key(&mut self, key: PassKey, data: T) {
        self.mapping.insert(key, data);
    }

    /// Clears the data for the pass given by the provided `key`, if it exists.
    pub fn clear_key(&mut self, key: PassKey) {
        self.mapping.remove(&key);
    }
}

impl PassDataMap<PassData> {
    /// Gets a reference to the last-written data for the pass `P` if it exists,
    /// and returns `None` otherwise.
    ///
    /// The data returned is returned as the concrete type.
    #[must_use]
    pub fn get<P: ConcretePass>(&self) -> Option<&P::Data> {
        self.mapping.get(&P::key())?.view_as::<P::Data>()
    }

    /// Writes the provided `data` into the container associating it with the
    /// pass `P`, overwriting any existing data for that pass.
    ///
    /// This expects the data to be the concrete pass data type for the pass in
    /// question.
    pub fn put<P: ConcretePass>(&mut self, data: P::Data) {
        let data = Box::new(data);
        self.mapping.insert(P::key(), data);
    }

    /// Clears the data for the pass `P` if it exists.
    pub fn clear<P: ConcretePass>(&mut self) {
        self.mapping.remove(&P::key());
    }
}

impl<T> Default for PassDataMap<T> {
    fn default() -> Self {
        Self::new()
    }
}
