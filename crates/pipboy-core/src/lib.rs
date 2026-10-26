//! Offline-first data and capability layer for the Rust-Boy native console.
//!
//! This crate exposes explicit, operator-gated local network probes and
//! UniFi read-only HTTPS discovery; it never scans merely by being imported.
//! Serial port and radio interfaces remain inactive.

pub mod blueprint;
pub mod branding;
pub mod buffer;
pub mod capability;
pub mod catalog;
pub mod device_simulator;
pub mod diagnostics;
pub mod game;
pub mod host_report;
pub mod lifecycle;
pub mod meshtastic_passive;
pub mod message_keys;
pub mod message_pace;
pub mod message_protocol;
pub mod messenger;
pub mod metrics;
pub mod mock_adapters;
pub mod network_health;
pub mod network_probe;
pub mod network_suite;
pub mod notes;
pub mod observation;
pub mod peers;
pub mod permissions;
pub mod reclamation;
pub mod registry;
pub mod runtime;
pub mod scout_gis;
pub mod simulator_replay;
pub mod tasks;
pub mod telemetry;
pub mod unifi;
pub mod virtual_transport;
pub mod workbench;

pub mod host_telemetry;
