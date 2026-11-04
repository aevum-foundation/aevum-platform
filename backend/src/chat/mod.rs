//! Aevum Platform — Chat domain module.
//!
//! PR-1: Chat Domain & Storage Contract.
//!
//! Scope:
//! - Domain models for conversations and messages.
//! - ChatStorage trait (storage-agnostic).
//! - InMemoryChatStorage for tests.
//! - ChatService with ownership and validation rules.
//!
//! Out of scope (deferred):
//! - AevumDB adapter (needs public atomic batch API).
//! - HTTP routes and middleware wiring.
//! - Streaming and Agent integration.
//!
//! Design decisions:
//! - Messages are ordered by `(created_at, message_id)`.
//! - `MessageId` is a UUIDv7 (time-ordered, sortable).
//! - No integer sequence is assumed; atomic counters are not available
//!   in AevumDB yet.
//! - Every storage operation requires the owner context.

pub mod models;
pub mod service;
pub mod storage;

pub use models::{
    Conversation, ConversationId, CreateConversationRequest, CreateMessageRequest,
    ListConversationsQuery, ListMessagesQuery, Message, MessageId, MessageRole, MessageStatus,
    UpdateConversationRequest, DEFAULT_PAGE_SIZE, MAX_CONTENT_LEN, MAX_IDEMPOTENCY_KEY_LEN,
    MAX_PAGE_SIZE, MAX_TITLE_LEN, MIN_TITLE_LEN,
};
pub use service::ChatService;
pub use storage::{ChatStorage, InMemoryChatStorage};
