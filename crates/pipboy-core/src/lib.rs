//! Offline-first data and capability layer for the Rust-Boy native console.
//!
//! This crate does not open network connections, interact with serial ports,
//! discover nearby devices, or perform radio transmissions.

pub mod blueprint;
pub mod branding;
pub mod buffer;
pub mod capability;
pub mod device_simulator;
pub mod catalog;
pub mod lifecycle;
pub mod metrics;
pub mod notes;
pub mod observation;
pub mod permissions;
pub mod registry;
pub mod runtime;
pub mod telemetry;

pub mod host_telemetry;
