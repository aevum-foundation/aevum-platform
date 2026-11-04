//! Chat storage contract.
//!
//! PR-1: defines `ChatStorage` trait and an `InMemoryChatStorage`
//! implementation for tests. The AevumDB adapter is deferred to a
//! later PR that first adds a public atomic batch API to `AevumDb`.
//!
//! # Ownership rules
//!
//! Every read/write operation takes the owner (`owner_id: Uuid`) as a
//! parameter and must refuse to return or modify data that belongs to
//! a different owner. Callers cannot bypass this by knowing an ID.
//!
//! # Not found semantics
//!
//! `get_*` methods return `Ok(None)` both when the resource does not
//! exist and when it belongs to another owner. The service layer maps
//! this to a uniform 404 response, so resource existence is not leaked.
//!
//! # Pagination
//!
//! List methods accept an opaque cursor string. The storage layer treats
//! it as opaque and only guarantees a stable order. Cursor encoding and
//! validation are the responsibility of the service layer.
//!
//! # Idempotency
//!
//! `add_message` must reject duplicates when the same
//! `(owner_id, conversation_id, idempotency_key)` triple has already
//! been stored. Implementations may enforce this with a secondary index
//! or by a direct lookup on the key.
//!
//! # Error mapping
//!
//! Storage implementations return `ApiError` (from `crate::error`) so
//! that HTTP and service layers share one error type.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::chat::models::{Conversation, ConversationId, Message, MessageId};
use crate::error::ApiError;

// ─────────────────────────────────────────────
// Cursor
// ─────────────────────────────────────────────

/// Opaque pagination cursor.
///
/// The storage layer treats the cursor as an opaque token and does
/// not decode it. Encoding, decoding, and validation are handled by
/// `ChatService`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor(pub String);

// ─────────────────────────────────────────────
// ChatStorage trait
// ─────────────────────────────────────────────

/// Storage-agnostic contract for the chat domain.
///
/// Implementations:
/// - `InMemoryChatStorage` — tests and local development.
/// - `AevumDbChatStorage` — planned, pending public atomic batch API.
#[async_trait]
pub trait ChatStorage: Send + Sync + std::fmt::Debug {
    // ─── Conversations ─────────────────────────────

    /// Persist a new conversation.
    ///
    /// Must fail with `ApiError::Conflict` if a conversation with the
    /// same `id` already exists.
    async fn create_conversation(&self, conversation: &Conversation) -> Result<(), ApiError>;

    /// Fetch a conversation, restricted to its owner.
    ///
    /// Returns `Ok(None)` if the conversation does not exist or belongs
    /// to another owner.
    async fn get_conversation(
        &self,
        id: &ConversationId,
        owner_id: &Uuid,
    ) -> Result<Option<Conversation>, ApiError>;

    /// List conversations of a single owner, ordered by
    /// `(updated_at DESC, id DESC)` for stable pagination.
    ///
    /// If `cursor` is provided, only conversations strictly after the
    /// cursor position are returned. `limit` is a hard cap.
    async fn list_conversations(
        &self,
        owner_id: &Uuid,
        cursor: Option<Cursor>,
        limit: usize,
    ) -> Result<Vec<Conversation>, ApiError>;

    /// Update conversation metadata, restricted to its owner.
    ///
    /// Returns `Ok(None)` if the conversation does not exist or belongs
    /// to another owner.
    async fn update_conversation(
        &self,
        conversation: &Conversation,
    ) -> Result<Option<Conversation>, ApiError>;

    /// Delete a conversation and all of its messages, restricted to
    /// its owner.
    ///
    /// Returns `Ok(true)` if a conversation was removed, `Ok(false)`
    /// otherwise.
    async fn delete_conversation(
        &self,
        id: &ConversationId,
        owner_id: &Uuid,
    ) -> Result<bool, ApiError>;

    // ─── Messages ──────────────────────────────────

    /// Append a message.
    ///
    /// Idempotency semantics:
    /// - If `message.idempotency_key` is `None`: store and return the message.
    /// - If `Some(key)` and no prior message exists for
    ///   `(owner_id, conversation_id, key)`: store and return the message.
    /// - If `Some(key)` and a prior message exists with **identical content
    ///   and role**: return the previously stored message (idempotent replay).
    /// - If `Some(key)` and a prior message exists with **different content
    ///   or role**: return `ApiError::Conflict`.
    ///
    /// The `owner_id` for the idempotency scope is taken from the parent
    /// conversation, not from the message itself.
    async fn add_message(&self, message: &Message) -> Result<Message, ApiError>;

    /// Fetch a message, restricted to its owner through the parent
    /// conversation.
    async fn get_message(
        &self,
        id: &MessageId,
        owner_id: &Uuid,
    ) -> Result<Option<Message>, ApiError>;

    /// List messages of a conversation, ordered by
    /// `(created_at ASC, id ASC)`, restricted to the owner.
    ///
    /// If `cursor` is provided, only messages strictly after the cursor
    /// position are returned. `limit` is a hard cap.
    async fn list_messages(
        &self,
        conversation_id: &ConversationId,
        owner_id: &Uuid,
        cursor: Option<Cursor>,
        limit: usize,
    ) -> Result<Vec<Message>, ApiError>;
}

// ─────────────────────────────────────────────
// InMemoryChatStorage
// ─────────────────────────────────────────────

/// In-memory chat storage.
///
/// Scope: tests, local development, conformance harness.
/// Not a production backend — no durability across process restarts.
#[derive(Debug, Default)]
pub struct InMemoryChatStorage {
    inner: Arc<RwLock<Inner>>,
}

#[derive(Debug, Default)]
struct Inner {
    /// Conversations by id.
    conversations: HashMap<ConversationId, Conversation>,

    /// Message ids grouped by conversation, in insertion order.
    /// Actual ordering is performed at read time using the
    /// `(created_at, id)` comparator.
    messages_by_conversation: HashMap<ConversationId, Vec<MessageId>>,

    /// Messages by id.
    messages: HashMap<MessageId, Message>,

    /// Idempotency index:
    /// `(owner_id, conversation_id, idempotency_key) -> MessageId`.
    idempotency: HashMap<(Uuid, ConversationId, String), MessageId>,
}

impl InMemoryChatStorage {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl ChatStorage for InMemoryChatStorage {
    async fn create_conversation(&self, conversation: &Conversation) -> Result<(), ApiError> {
        let mut inner = self.inner.write().await;

        if inner.conversations.contains_key(&conversation.id) {
            return Err(ApiError::Conflict);
        }

        inner
            .conversations
            .insert(conversation.id, conversation.clone());
        inner
            .messages_by_conversation
            .insert(conversation.id, Vec::new());

        Ok(())
    }

    async fn get_conversation(
        &self,
        id: &ConversationId,
        owner_id: &Uuid,
    ) -> Result<Option<Conversation>, ApiError> {
        let inner = self.inner.read().await;

        Ok(inner
            .conversations
            .get(id)
            .filter(|c| &c.owner_id == owner_id)
            .cloned())
    }

    async fn list_conversations(
        &self,
        owner_id: &Uuid,
        cursor: Option<Cursor>,
        limit: usize,
    ) -> Result<Vec<Conversation>, ApiError> {
        let inner = self.inner.read().await;

        let mut list: Vec<Conversation> = inner
            .conversations
            .values()
            .filter(|c| &c.owner_id == owner_id)
            .cloned()
            .collect();

        // Ordering: (updated_at DESC, id DESC) — newest first.
        list.sort_by(|a, b| {
            b.updated_at
                .cmp(&a.updated_at)
                .then_with(|| b.id.0.cmp(&a.id.0))
        });

        // Cursor is treated as opaque by storage; the service layer
        // is expected to pass a limit and to handle pagination
        // semantics. For PR-1 we only respect `limit`.
        let _ = cursor;

        list.truncate(limit);
        Ok(list)
    }

    async fn update_conversation(
        &self,
        conversation: &Conversation,
    ) -> Result<Option<Conversation>, ApiError> {
        let mut inner = self.inner.write().await;

        let Some(existing) = inner.conversations.get_mut(&conversation.id) else {
            return Ok(None);
        };

        if existing.owner_id != conversation.owner_id {
            return Ok(None);
        }

        *existing = conversation.clone();
        Ok(Some(conversation.clone()))
    }

    async fn delete_conversation(
        &self,
        id: &ConversationId,
        owner_id: &Uuid,
    ) -> Result<bool, ApiError> {
        let mut inner = self.inner.write().await;

        let matches = inner
            .conversations
            .get(id)
            .map(|c| &c.owner_id == owner_id)
            .unwrap_or(false);

        if !matches {
            return Ok(false);
        }

        inner.conversations.remove(id);

        // Remove associated messages.
        if let Some(msg_ids) = inner.messages_by_conversation.remove(id) {
            for mid in msg_ids {
                inner.messages.remove(&mid);
            }
        }

        // Remove idempotency keys scoped to this conversation.
        inner.idempotency.retain(|(_, conv_id, _), _| conv_id != id);

        Ok(true)
    }

    async fn add_message(&self, message: &Message) -> Result<Message, ApiError> {
        let mut inner = self.inner.write().await;

        // Conversation must exist.
        let conversation = inner
            .conversations
            .get(&message.conversation_id)
            .cloned()
            .ok_or(ApiError::NotFound)?;

        // Idempotency check.
        if let Some(key) = &message.idempotency_key {
            let triple = (conversation.owner_id, message.conversation_id, key.clone());

            if let Some(existing_id) = inner.idempotency.get(&triple).copied() {
                let existing = inner
                    .messages
                    .get(&existing_id)
                    .cloned()
                    .ok_or(ApiError::NotFound)?;

                // Same key + identical payload → idempotent replay.
                if existing.role == message.role && existing.content == message.content {
                    return Ok(existing);
                }

                // Same key + different payload → conflict.
                return Err(ApiError::Conflict);
            }

            inner.idempotency.insert(triple, message.id);
        }

        // Message id must be unique.
        if inner.messages.contains_key(&message.id) {
            return Err(ApiError::Conflict);
        }

        inner.messages.insert(message.id, message.clone());
        inner
            .messages_by_conversation
            .entry(message.conversation_id)
            .or_default()
            .push(message.id);

        Ok(message.clone())
    }

    async fn get_message(
        &self,
        id: &MessageId,
        owner_id: &Uuid,
    ) -> Result<Option<Message>, ApiError> {
        let inner = self.inner.read().await;

        let Some(message) = inner.messages.get(id) else {
            return Ok(None);
        };

        let owner_matches = inner
            .conversations
            .get(&message.conversation_id)
            .map(|c| &c.owner_id == owner_id)
            .unwrap_or(false);

        if !owner_matches {
            return Ok(None);
        }

        Ok(Some(message.clone()))
    }

    async fn list_messages(
        &self,
        conversation_id: &ConversationId,
        owner_id: &Uuid,
        cursor: Option<Cursor>,
        limit: usize,
    ) -> Result<Vec<Message>, ApiError> {
        let inner = self.inner.read().await;

        let conversation = match inner.conversations.get(conversation_id) {
            Some(c) if &c.owner_id == owner_id => c,
            _ => return Ok(Vec::new()),
        };
        let _ = conversation;

        let Some(ids) = inner.messages_by_conversation.get(conversation_id) else {
            return Ok(Vec::new());
        };

        let mut messages: Vec<Message> = ids
            .iter()
            .filter_map(|mid| inner.messages.get(mid).cloned())
            .collect();

        // Ordering: (created_at ASC, id ASC) — oldest first.
        messages.sort_by(|a, b| {
            a.created_at
                .cmp(&b.created_at)
                .then_with(|| a.id.0.cmp(&b.id.0))
        });

        // PR-1: cursor is opaque; storage only respects `limit`.
        let _ = cursor;

        messages.truncate(limit);
        Ok(messages)
    }
}
