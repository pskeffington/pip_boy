//! Offline-first data and capability layer for the Rust-Boy native console.
//!
//! This crate does not open network connections, interact with serial ports,
//! discover nearby devices, or perform radio transmissions.

pub mod blueprint;
pub mod branding;
pub mod buffer;
pub mod capability;
pub mod catalog;
pub mod device_simulator;
pub mod game;
pub mod host_report;
pub mod lifecycle;
pub mod metrics;
pub mod mock_adapters;
pub mod notes;
pub mod observation;
pub mod permissions;
pub mod registry;
pub mod runtime;
pub mod scout_gis;
pub mod simulator_replay;
pub mod telemetry;
pub mod virtual_transport;
pub mod workbench;

pub mod host_telemetry;
