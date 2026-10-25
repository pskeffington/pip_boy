//! Offline-first data and capability layer for the Rust-Boy native console.
//!
//! This crate does not open network connections, interact with serial ports,
//! discover nearby devices, or perform radio transmissions.

pub mod branding;
pub mod blueprint;
pub mod buffer;
pub mod lifecycle;
pub mod observation;
pub mod capability;
pub mod runtime;
pub mod catalog;
pub mod metrics;
pub mod notes;
pub mod registry;
