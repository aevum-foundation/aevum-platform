//! Growth event ledger.
//!
//! Events describe what happened inside the Growth subsystem and why the
//! current state of Sources, Publications, Topics and Opportunities looks
//! the way it does.
//!
//! Storage trait and implementations are added in `storage.rs` and
//! `aevumdb.rs`.

pub mod models;

pub use models::{GrowthEvent, GrowthEventKind};
