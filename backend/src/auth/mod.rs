//! Authentication module for Aevum Platform.

pub mod aevumdb_storage;
pub mod api;
pub mod authenticator;
pub mod backup_codes;
pub mod contracts;
pub mod csrf;
pub mod csrf_middleware;
pub mod email;
pub mod events;

pub use events::storage::{InMemorySecurityEventStorage, SecurityEventStorage};
pub mod middleware;
pub mod models;
pub mod password;
pub mod rate_limit;
pub mod service;
pub mod storage;
