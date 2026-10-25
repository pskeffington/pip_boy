//! Offline-first data and capability layer for the PIP_BOY native console.
//!
//! This crate does not open network connections, interact with serial ports,
//! discover nearby devices, or perform radio transmissions.

pub mod catalog;
pub mod metrics;
pub mod notes;
pub mod registry;
