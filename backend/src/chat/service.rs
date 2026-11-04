//! Chat service — business rules on top of `ChatStorage`.

use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::chat::models::{
    Conversation, ConversationId, CreateConversationRequest, CreateMessageRequest, Message,
    MessageId, MessageRole, MessageStatus, UpdateConversationRequest, DEFAULT_PAGE_SIZE,
    MAX_CONTENT_LEN, MAX_IDEMPOTENCY_KEY_LEN, MAX_PAGE_SIZE, MAX_TITLE_LEN, MIN_TITLE_LEN,
};
use crate::chat::storage::{ChatStorage, Cursor};
use crate::error::ApiError;

#[derive(Debug, Clone)]
pub struct ChatService {
    storage: Arc<dyn ChatStorage>,
}

impl ChatService {
    pub fn new<S: ChatStorage + 'static>(storage: S) -> Self {
        Self {
            storage: Arc::new(storage),
        }
    }

    pub fn from_arc(storage: Arc<dyn ChatStorage>) -> Self {
        Self { storage }
    }

    pub async fn create_conversation(
        &self,
        owner_id: Uuid,
        req: CreateConversationRequest,
    ) -> Result<Conversation, ApiError> {
        let title = validate_title(&req.title)?;
        let now = Utc::now();

        let conversation = Conversation {
            id: ConversationId::new(),
            owner_id,
            title,
            created_at: now,
            updated_at: now,
        };

        self.storage.create_conversation(&conversation).await?;
        Ok(conversation)
    }

    pub async fn get_conversation(
        &self,
        owner_id: Uuid,
        id: ConversationId,
    ) -> Result<Conversation, ApiError> {
        self.storage
            .get_conversation(&id, &owner_id)
            .await?
            .ok_or(ApiError::NotFound)
    }

    pub async fn list_conversations(
        &self,
        owner_id: Uuid,
        cursor: Option<String>,
        limit: Option<u32>,
    ) -> Result<Vec<Conversation>, ApiError> {
        let limit = normalize_limit(limit);
        let cursor = decode_cursor(cursor)?;

        self.storage
            .list_conversations(&owner_id, cursor, limit)
            .await
    }

    pub async fn update_conversation(
        &self,
        owner_id: Uuid,
        id: ConversationId,
        req: UpdateConversationRequest,
    ) -> Result<Conversation, ApiError> {
        let title = validate_title(&req.title)?;

        let existing = self
            .storage
            .get_conversation(&id, &owner_id)
            .await?
            .ok_or(ApiError::NotFound)?;

        let updated = Conversation {
            id: existing.id,
            owner_id: existing.owner_id,
            title,
            created_at: existing.created_at,
            updated_at: Utc::now(),
        };

        self.storage
            .update_conversation(&updated)
            .await?
            .ok_or(ApiError::NotFound)
    }

    pub async fn delete_conversation(
        &self,
        owner_id: Uuid,
        id: ConversationId,
    ) -> Result<(), ApiError> {
        let removed = self.storage.delete_conversation(&id, &owner_id).await?;
        if !removed {
            return Err(ApiError::NotFound);
        }
        Ok(())
    }

    pub async fn add_user_message(
        &self,
        owner_id: Uuid,
        conversation_id: ConversationId,
        req: CreateMessageRequest,
    ) -> Result<Message, ApiError> {
        let content = validate_content(&req.content)?;
        let idempotency_key = validate_idempotency_key(req.idempotency_key)?;

        let _ = self
            .storage
            .get_conversation(&conversation_id, &owner_id)
            .await?
            .ok_or(ApiError::NotFound)?;

        let message = Message {
            id: MessageId::new_v7(),
            conversation_id,
            created_at: Utc::now(),
            role: MessageRole::User,
            content,
            status: MessageStatus::Pending,
            idempotency_key,
        };

        self.storage.add_message(&message).await
    }

    pub async fn get_message(&self, owner_id: Uuid, id: MessageId) -> Result<Message, ApiError> {
        self.storage
            .get_message(&id, &owner_id)
            .await?
            .ok_or(ApiError::NotFound)
    }

    pub async fn list_messages(
        &self,
        owner_id: Uuid,
        conversation_id: ConversationId,
        cursor: Option<String>,
        limit: Option<u32>,
    ) -> Result<Vec<Message>, ApiError> {
        let limit = normalize_limit(limit);
        let cursor = decode_cursor(cursor)?;

        let _ = self
            .storage
            .get_conversation(&conversation_id, &owner_id)
            .await?
            .ok_or(ApiError::NotFound)?;

        self.storage
            .list_messages(&conversation_id, &owner_id, cursor, limit)
            .await
    }
}

fn validate_title(raw: &str) -> Result<String, ApiError> {
    let trimmed = raw.trim();
    if trimmed.chars().count() < MIN_TITLE_LEN {
        return Err(ApiError::ValidationFailed {
            code: "chat.title.empty",
            message: "Title must not be empty",
        });
    }
    if trimmed.chars().count() > MAX_TITLE_LEN {
        return Err(ApiError::ValidationFailed {
            code: "chat.title.too_long",
            message: "Title is too long",
        });
    }
    Ok(trimmed.to_string())
}

fn validate_content(raw: &str) -> Result<String, ApiError> {
    if raw.is_empty() {
        return Err(ApiError::ValidationFailed {
            code: "chat.content.empty",
            message: "Content must not be empty",
        });
    }
    if raw.chars().count() > MAX_CONTENT_LEN {
        return Err(ApiError::ValidationFailed {
            code: "chat.content.too_long",
            message: "Content is too long",
        });
    }
    Ok(raw.to_string())
}

fn validate_idempotency_key(raw: Option<String>) -> Result<Option<String>, ApiError> {
    match raw {
        None => Ok(None),
        Some(key) => {
            let trimmed = key.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            if trimmed.chars().count() > MAX_IDEMPOTENCY_KEY_LEN {
                return Err(ApiError::ValidationFailed {
                    code: "chat.idempotency_key.too_long",
                    message: "Idempotency key is too long",
                });
            }
            if !trimmed
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            {
                return Err(ApiError::ValidationFailed {
                    code: "chat.idempotency_key.invalid_chars",
                    message: "Idempotency key contains invalid characters",
                });
            }
            Ok(Some(trimmed.to_string()))
        }
    }
}

fn normalize_limit(limit: Option<u32>) -> usize {
    let raw = limit.unwrap_or(DEFAULT_PAGE_SIZE);
    let capped = raw.min(MAX_PAGE_SIZE).max(1);
    capped as usize
}

fn decode_cursor(raw: Option<String>) -> Result<Option<Cursor>, ApiError> {
    match raw {
        None => Ok(None),
        Some(token) => {
            if !token.starts_with("v1:") {
                return Err(ApiError::ValidationFailed {
                    code: "chat.cursor.invalid",
                    message: "Cursor has invalid format",
                });
            }
            Ok(Some(Cursor(token)))
        }
    }
}

pub fn encode_cursor(created_at: chrono::DateTime<Utc>, id: Uuid) -> String {
    format!("v1:{}:{}", created_at.timestamp_micros(), id)
}
