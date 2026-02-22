// Rust guideline compliant 2026-02-16

//! Adapters (secondary ports) for the fraud-detection binary.
//!
//! Each sub-module implements one or more hexagonal port traits defined in the
//! `domain` crate. Adapters are intentionally isolated from domain and producer
//! logic.

pub mod in_memory_buffer;
