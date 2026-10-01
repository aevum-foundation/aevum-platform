//! Aevum Growth.
//!
//! Knowledge intelligence and visibility subsystem for Aevum.
//!
//! Responsibilities:
//! - Source discovery and registry
//! - Publication ingestion (read-only)
//! - Topic classification
//! - Trend detection
//! - Opportunity analysis
//! - Event ledger
//! - Analytics API
//!
//! Boundaries:
//! - This subsystem does NOT interact with Aevum Protocol internals.
//! - It reads public sources only.
//! - It never stores user accounts from third-party platforms.
//! - It never sends messages on behalf of users.
//!
//! See PHASE_1_PLAN.md for the current implementation scope.

pub mod models;
pub mod contracts;
pub mod events;
pub mod storage;
pub mod validation;
pub mod aevumdb;
pub mod in_memory;
pub mod ingestion;
#[cfg(test)]
pub mod conformance;
