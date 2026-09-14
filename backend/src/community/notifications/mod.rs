//! Notifications subsystem for the Community domain.
//!
//! B-2 — durable, idempotent notification storage and delivery.
//!
//! See `docs/architecture/notifications-v1.md` for the frozen contract.

pub mod contracts;
pub mod models;
pub mod storage;
pub mod validation;
