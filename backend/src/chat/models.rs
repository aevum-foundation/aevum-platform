//! Chat domain models.
//!
//! Types for conversations, messages, roles, and statuses.
//!
//! # Identifiers
//!
//! - `ConversationId` — UUID v4 (random).
//! - `MessageId` — UUID v7 (time-ordered prefix, sortable).
//!
//! `owner_id` is a raw `Uuid`, matching `AuthContext.user_id`.
//! A dedicated `UserId` newtype is deferred until Auth introduces one.
//!
//! # Ordering
//!
//! Messages are displayed in order of `(created_at, message_id)`.
//! This is a **display order**, not a strictly causal order:
//! UUIDv7 prefixes are time-based but not synchronized across processes,
//! and `created_at` may not reflect real causal sequence under concurrent
//! writes or clock skew.
//!
//! A strict causal sequence number would require an atomic counter in
//! the storage layer, which AevumDB does not currently expose.
//!
//! # Roles
//!
//! Clients may only submit messages with role `User`.
//! Roles `Assistant`, `System`, and `Tool` are server-only and must be
//! assigned by the backend, never accepted from untrusted input.
//! Client-facing DTOs (`CreateMessageRequest`) must not carry a `role` field.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─────────────────────────────────────────────
// Identifiers
// ─────────────────────────────────────────────

/// Conversation identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConversationId(pub Uuid);

impl ConversationId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ConversationId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ConversationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Message identifier.
///
/// UUIDv7 provides a time-ordered prefix, useful for stable sort keys
/// and for grouping messages by rough creation time. It is **not** a
/// strict causal order and must not be used as a substitute for an
/// atomic sequence number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(pub Uuid);

impl MessageId {
    /// Generate a new UUIDv7.
    pub fn new_v7() -> Self {
        Self(Uuid::now_v7())
    }
}

impl std::fmt::Display for MessageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ─────────────────────────────────────────────
// Roles and statuses
// ─────────────────────────────────────────────

/// Message role.
///
/// Client-facing APIs may only produce `User` messages.
/// `Assistant`, `System`, and `Tool` are assigned by the server and
/// must never be accepted from untrusted input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

impl MessageRole {
    /// True only for roles a client is allowed to submit directly.
    pub fn is_client_allowed(self) -> bool {
        matches!(self, Self::User)
    }
}

/// Message lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageStatus {
    /// Stored but not yet processed.
    Pending,
    /// Generation in progress.
    Streaming,
    /// Generation completed normally.
    Complete,
    /// Generation failed; an error may be attached separately.
    Failed,
}

// ─────────────────────────────────────────────
// Domain entities
// ─────────────────────────────────────────────

/// Conversation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Conversation {
    pub id: ConversationId,
    /// Owner of the conversation. Every read/write checks this.
    pub owner_id: Uuid,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub conversation_id: ConversationId,
    pub created_at: DateTime<Utc>,
    pub role: MessageRole,
    pub content: String,
    pub status: MessageStatus,
    /// Optional idempotency key to prevent duplicate creation on retry.
    ///
    /// Scope: `(owner_id, conversation_id, idempotency_key)`.
    /// Storage-level uniqueness is enforced by the ChatStorage
    /// implementation. The service validates format and length.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
}

// ─────────────────────────────────────────────
// Requests and queries
// ─────────────────────────────────────────────

/// Client request to create a conversation.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateConversationRequest {
    pub title: String,
}

/// Client request to update a conversation.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateConversationRequest {
    pub title: String,
}

/// Client request to append a message.
///
/// Note: no `role` field is accepted. The server always assigns
/// `MessageRole::User` for client-submitted messages.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateMessageRequest {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListConversationsQuery {
    /// Opaque cursor returned by a previous call. Decoded and validated
    /// by the service layer; treated as opaque by the HTTP layer.
    pub cursor: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListMessagesQuery {
    /// Opaque cursor returned by a previous call. Decoded and validated
    /// by the service layer; treated as opaque by the HTTP layer.
    pub cursor: Option<String>,
    pub limit: Option<u32>,
}

// ─────────────────────────────────────────────
// Validation constants
// ─────────────────────────────────────────────

/// Maximum conversation title length, in characters.
pub const MAX_TITLE_LEN: usize = 200;

/// Minimum conversation title length, in characters.
pub const MIN_TITLE_LEN: usize = 1;

/// Maximum message content length, in characters.
/// Enforced by the service layer before storage.
pub const MAX_CONTENT_LEN: usize = 32_768;

/// Maximum idempotency key length, in characters.
pub const MAX_IDEMPOTENCY_KEY_LEN: usize = 128;

/// Default page size for list endpoints.
pub const DEFAULT_PAGE_SIZE: u32 = 50;

/// Maximum page size for list endpoints.
pub const MAX_PAGE_SIZE: u32 = 200;
