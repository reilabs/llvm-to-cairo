//! Analysis passes are those that do not change the underlying IR structure,
//! but instead generate some kind of data that can be read by downstream
//! functionality to make decisions on the basis of.

pub mod module_map;
