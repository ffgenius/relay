//! Relay — secure cross-platform command router.
//!
//! See `product.md` at the repository root for the design specification.
//! The crate is organised as a thin `main.rs` wrapper around the modules
//! exposed here so that integration tests can drive the same code paths.

pub mod cli;
pub mod config;
pub mod discover;
pub mod doctor;
pub mod error;
pub mod path_setup;
pub mod registry;
pub mod runner;
pub mod shim;

pub use error::{RelayError, Result};
