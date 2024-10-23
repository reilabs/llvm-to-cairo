//! The `FlatLowered` Object (`FLO`) is the IR-based object format
//! for use as an interchange format between tools in this project;
//! and the format used for pre-linking objects.

#![warn(clippy::all, clippy::cargo, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)] // Allows for better API naming
#![allow(clippy::multiple_crate_versions)] // Enforced by our dependencies

pub mod flo;
pub mod intern;
pub mod poison;
pub mod types;

pub use flo::FlatLoweredObject;
