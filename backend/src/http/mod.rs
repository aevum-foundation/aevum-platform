//! Cross-cutting HTTP infrastructure.
//!
//! This module hosts HTTP transport policies that apply to the
//! application as a whole and do not belong to any domain
//! (`api/`, `auth/`, `growth/`, ...).
//!
//! Distinction:
//!
//! - `api/`   — WHAT an endpoint does.
//! - `auth/`  — WHO may access it.
//! - `http/`  — HOW HTTP transport behaves.
//!
//! See `docs/architecture/intelligence-integration-v1.md` Section 5.8
//! for the HEAD contract.

pub mod head;
