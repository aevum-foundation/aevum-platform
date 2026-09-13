//! Authentication module for Aevum Platform.

pub mod aevumdb_secret_cipher;
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
pub mod secret_cipher;
pub mod service;
pub mod two_factor;
pub mod storage;
