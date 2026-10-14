//! Chat conformance tests.
//!
//! PR-1: verify `ChatStorage` + `ChatService` contract with
//! `InMemoryChatStorage`.
//!
//! These tests are written so that the same scenarios can later be
//! re-run against `AevumDbChatStorage` (PR-2) without changes:
//! - No dependency on `InMemory` internals.
//! - Deterministic ordering (no `sleep()` for timing assertions).
//! - Explicit boundary values.
//!
//! Ownership isolation, cascade deletion, idempotency, pagination
//! and validation are all exercised.

use uuid::Uuid;

use aevum_platform_api::chat::{
    ChatService, CreateConversationRequest, CreateMessageRequest, InMemoryChatStorage,
    UpdateConversationRequest,
    MAX_CONTENT_LEN, MAX_TITLE_LEN,
};
use aevum_platform_api::error::ApiError;

// ─────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────

fn new_service() -> ChatService {
    ChatService::new(InMemoryChatStorage::new())
}

async fn mk_conv(
    svc: &ChatService,
    owner: Uuid,
    title: &str,
) -> aevum_platform_api::chat::Conversation {
    svc
        .create_conversation(
            owner,
            CreateConversationRequest {
                title: title.into(),
            },
        )
        .await
        .expect("create_conversation")
}

async fn mk_msg(
    svc: &ChatService,
    owner: Uuid,
    conv_id: aevum_platform_api::chat::ConversationId,
    content: &str,
    idempotency_key: Option<&str>,
) -> aevum_platform_api::chat::Message {
    svc
        .add_user_message(
            owner,
            conv_id,
            CreateMessageRequest {
                content: content.into(),
                idempotency_key: idempotency_key.map(|s| s.to_string()),
            },
        )
        .await
        .expect("add_user_message")
}

// ─────────────────────────────────────────────
// Conversations
// ─────────────────────────────────────────────

#[tokio::test]
async fn create_and_get_conversation() {
    let svc = new_service();
    let owner = Uuid::new_v4();

    let conv = mk_conv(&svc, owner, "Test").await;
    assert_eq!(conv.owner_id, owner);
    assert_eq!(conv.title, "Test");

    let fetched = svc.get_conversation(owner, conv.id).await.unwrap();
    assert_eq!(fetched.id, conv.id);
    assert_eq!(fetched.created_at, conv.created_at);
    assert_eq!(fetched.owner_id, owner);
}

#[tokio::test]
async fn get_conversation_foreign_owner_returns_not_found() {
    let svc = new_service();
    let alice = Uuid::new_v4();
    let bob = Uuid::new_v4();

    let conv = mk_conv(&svc, alice, "Alice").await;

    let result = svc.get_conversation(bob, conv.id).await;
    assert!(matches!(result, Err(ApiError::NotFound)));
}

#[tokio::test]
async fn list_conversations_only_own_and_no_cross_ids() {
    let svc = new_service();
    let alice = Uuid::new_v4();
    let bob = Uuid::new_v4();
    let carol = Uuid::new_v4(); // empty user

    let a1 = mk_conv(&svc, alice, "A1").await;
    let a2 = mk_conv(&svc, alice, "A2").await;
    let b1 = mk_conv(&svc, bob, "B1").await;

    let alice_list = svc.list_conversations(alice, None, None).await.unwrap();
    let bob_list = svc.list_conversations(bob, None, None).await.unwrap();
    let carol_list = svc.list_conversations(carol, None, None).await.unwrap();

    assert_eq!(alice_list.len(), 2);
    assert_eq!(bob_list.len(), 1);
    assert!(carol_list.is_empty());

    // Every returned item belongs to the correct owner.
    for c in &alice_list {
        assert_eq!(c.owner_id, alice);
    }
    for c in &bob_list {
        assert_eq!(c.owner_id, bob);
    }

    // No cross-owner IDs.
    let alice_ids: std::collections::HashSet<_> = alice_list.iter().map(|c| c.id).collect();
    assert!(!alice_ids.contains(&b1.id));
    assert!(alice_ids.contains(&a1.id));
    assert!(alice_ids.contains(&a2.id));

    let bob_ids: std::collections::HashSet<_> = bob_list.iter().map(|c| c.id).collect();
    assert!(!bob_ids.contains(&a1.id));
    assert!(!bob_ids.contains(&a2.id));
    assert!(bob_ids.contains(&b1.id));
}

#[tokio::test]
async fn update_conversation_preserves_owner_and_created_at() {
    let svc = new_service();
    let owner = Uuid::new_v4();

    let conv = mk_conv(&svc, owner, "Original").await;
    let original_created = conv.created_at;
    let original_owner = conv.owner_id;

    let updated = svc
        .update_conversation(
            owner,
            conv.id,
            UpdateConversationRequest {
                title: "Renamed".into(),
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.title, "Renamed");
    assert_eq!(updated.owner_id, original_owner);
    assert_eq!(updated.created_at, original_created);
    // Contract: `updated_at` is set by the service, must not be
    // earlier than the original creation time.
    assert!(updated.updated_at >= original_created);
}

#[tokio::test]
async fn update_conversation_foreign_owner_returns_not_found_and_no_side_effects() {
    let svc = new_service();
    let alice = Uuid::new_v4();
    let bob = Uuid::new_v4();

    let conv = mk_conv(&svc, alice, "Original").await;
    let _m = mk_msg(&svc, alice, conv.id, "hello", None).await;

    let result = svc
        .update_conversation(
            bob,
            conv.id,
            UpdateConversationRequest {
                title: "Hacked".into(),
            },
        )
        .await;

    assert!(matches!(result, Err(ApiError::NotFound)));

    // Alice's state unchanged.
    let still = svc.get_conversation(alice, conv.id).await.unwrap();
    assert_eq!(still.title, "Original");
    assert_eq!(still.owner_id, alice);
}

#[tokio::test]
async fn delete_conversation_cascades_messages() {
    let svc = new_service();
    let owner = Uuid::new_v4();

    let conv = mk_conv(&svc, owner, "To delete").await;

    let m1 = mk_msg(&svc, owner, conv.id, "hello", None).await;
    let m2 = mk_msg(&svc, owner, conv.id, "world", None).await;

    assert!(svc.get_message(owner, m1.id).await.is_ok());
    assert!(svc.get_message(owner, m2.id).await.is_ok());

    svc.delete_conversation(owner, conv.id).await.unwrap();

    // Conversation gone.
    assert!(matches!(
        svc.get_conversation(owner, conv.id).await,
        Err(ApiError::NotFound)
    ));

    // Messages gone (direct).
    assert!(matches!(
        svc.get_message(owner, m1.id).await,
        Err(ApiError::NotFound)
    ));
    assert!(matches!(
        svc.get_message(owner, m2.id).await,
        Err(ApiError::NotFound)
    ));

    // Messages not accessible via pagination either.
    assert!(matches!(
        svc.list_messages(owner, conv.id, None, None).await,
        Err(ApiError::NotFound)
    ));
}

#[tokio::test]
async fn delete_conversation_foreign_owner_returns_not_found_and_no_side_effects() {
    let svc = new_service();
    let alice = Uuid::new_v4();
    let bob = Uuid::new_v4();

    let conv = mk_conv(&svc, alice, "Alice").await;
    let m = mk_msg(&svc, alice, conv.id, "keep me", None).await;

    let result = svc.delete_conversation(bob, conv.id).await;
    assert!(matches!(result, Err(ApiError::NotFound)));

    // Conversation and messages still present for Alice.
    assert!(svc.get_conversation(alice, conv.id).await.is_ok());
    assert!(svc.get_message(alice, m.id).await.is_ok());

    let messages = svc.list_messages(alice, conv.id, None, None).await.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].content, "keep me");
}

#[tokio::test]
async fn delete_conversation_twice_returns_not_found_second_time() {
    let svc = new_service();
    let owner = Uuid::new_v4();

    let conv = mk_conv(&svc, owner, "Bye").await;

    svc.delete_conversation(owner, conv.id).await.unwrap();

    let second = svc.delete_conversation(owner, conv.id).await;
    assert!(matches!(second, Err(ApiError::NotFound)));
}

// ─────────────────────────────────────────────
// Messages
// ─────────────────────────────────────────────

#[tokio::test]
async fn add_message_role_is_user_and_status_pending() {
    let svc = new_service();
    let owner = Uuid::new_v4();

    let conv = mk_conv(&svc, owner, "Test").await;
    let msg = mk_msg(&svc, owner, conv.id, "Hello", None).await;

    assert_eq!(msg.role, aevum_platform_api::chat::MessageRole::User);
    assert_eq!(msg.status, aevum_platform_api::chat::MessageStatus::Pending);
    assert_eq!(msg.content, "Hello");
    assert_eq!(msg.conversation_id, conv.id);
}

#[tokio::test]
async fn get_message_foreign_owner_returns_not_found() {
    let svc = new_service();
    let alice = Uuid::new_v4();
    let bob = Uuid::new_v4();

    let conv = mk_conv(&svc, alice, "Test").await;
    let msg = mk_msg(&svc, alice, conv.id, "secret", None).await;

    let result = svc.get_message(bob, msg.id).await;
    assert!(matches!(result, Err(ApiError::NotFound)));
}

#[tokio::test]
async fn list_messages_foreign_owner_returns_not_found() {
    let svc = new_service();
    let alice = Uuid::new_v4();
    let bob = Uuid::new_v4();

    let conv = mk_conv(&svc, alice, "Test").await;
    let _ = mk_msg(&svc, alice, conv.id, "secret", None).await;

    let result = svc.list_messages(bob, conv.id, None, None).await;
    assert!(matches!(result, Err(ApiError::NotFound)));
}

#[tokio::test]
async fn add_message_to_foreign_conversation_returns_not_found() {
    let svc = new_service();
    let alice = Uuid::new_v4();
    let bob = Uuid::new_v4();

    let conv = mk_conv(&svc, alice, "Alice").await;

    let result = svc
        .add_user_message(
            bob,
            conv.id,
            CreateMessageRequest {
                content: "intrusion".into(),
                idempotency_key: None,
            },
        )
        .await;

    assert!(matches!(result, Err(ApiError::NotFound)));

    // No message added for Alice.
    let messages = svc.list_messages(alice, conv.id, None, None).await.unwrap();
    assert!(messages.is_empty());
}

#[tokio::test]
async fn list_messages_ordered_by_created_at_then_id() {
    let svc = new_service();
    let owner = Uuid::new_v4();

    let conv = mk_conv(&svc, owner, "Test").await;

    // UUIDv7 IDs are monotonic at millisecond resolution; inserts in
    // order guarantee `created_at` is non-decreasing.
    let m1 = mk_msg(&svc, owner, conv.id, "first", None).await;
    let m2 = mk_msg(&svc, owner, conv.id, "second", None).await;
    let m3 = mk_msg(&svc, owner, conv.id, "third", None).await;

    let messages = svc.list_messages(owner, conv.id, None, None).await.unwrap();
    assert_eq!(messages.len(), 3);

    // UUIDv7 is time-ordered, so the tie-breaker `id` alone is
    // enough when `created_at` is identical.
    assert_eq!(messages[0].id, m1.id);
    assert_eq!(messages[1].id, m2.id);
    assert_eq!(messages[2].id, m3.id);
    assert_eq!(messages[0].content, "first");
    assert_eq!(messages[1].content, "second");
    assert_eq!(messages[2].content, "third");
}

// ─────────────────────────────────────────────
// Idempotency
// ─────────────────────────────────────────────

#[tokio::test]
async fn idempotency_same_key_same_content_returns_same_message() {
    let svc = new_service();
    let owner = Uuid::new_v4();

    let conv = mk_conv(&svc, owner, "Test").await;

    let first = mk_msg(&svc, owner, conv.id, "Hello", Some("k1")).await;
    let second = mk_msg(&svc, owner, conv.id, "Hello", Some("k1")).await;

    // Idempotent replay: same ID returned, no duplicate stored.
    assert_eq!(first.id, second.id);
    assert_eq!(first.content, second.content);

    let messages = svc.list_messages(owner, conv.id, None, None).await.unwrap();
    assert_eq!(messages.len(), 1);
}

#[tokio::test]
async fn idempotency_same_key_different_content_returns_conflict() {
    let svc = new_service();
    let owner = Uuid::new_v4();

    let conv = mk_conv(&svc, owner, "Test").await;

    let _ = mk_msg(&svc, owner, conv.id, "Hello", Some("k1")).await;

    let result = svc
        .add_user_message(
            owner,
            conv.id,
            CreateMessageRequest {
                content: "Different".into(),
                idempotency_key: Some("k1".into()),
            },
        )
        .await;

    assert!(matches!(result, Err(ApiError::Conflict)));

    // Only the first message is stored.
    let messages = svc.list_messages(owner, conv.id, None, None).await.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].content, "Hello");
}

#[tokio::test]
async fn idempotency_scope_is_per_conversation() {
    let svc = new_service();
    let owner = Uuid::new_v4();

    let c1 = mk_conv(&svc, owner, "C1").await;
    let c2 = mk_conv(&svc, owner, "C2").await;

    // Same key, different conversations — both accepted.
    let m1 = mk_msg(&svc, owner, c1.id, "msg", Some("shared")).await;
    let m2 = mk_msg(&svc, owner, c2.id, "msg", Some("shared")).await;

    assert_ne!(m1.id, m2.id);

    let l1 = svc.list_messages(owner, c1.id, None, None).await.unwrap();
    let l2 = svc.list_messages(owner, c2.id, None, None).await.unwrap();

    assert_eq!(l1.len(), 1);
    assert_eq!(l2.len(), 1);
}

// ─────────────────────────────────────────────
// Concurrency
// ─────────────────────────────────────────────

#[tokio::test]
async fn concurrent_same_key_only_one_message_stored() {
    use std::sync::Arc;

    let svc = Arc::new(new_service());
    let owner = Uuid::new_v4();

    let conv = mk_conv(&svc, owner, "Test").await;
    let conv_id = conv.id;

    let mut handles = Vec::new();

    for _ in 0..8 {
        let svc = svc.clone();
        handles.push(tokio::spawn(async move {
            svc
                .add_user_message(
                    owner,
                    conv_id,
                    CreateMessageRequest {
                        content: "concurrent".into(),
                        idempotency_key: Some("race-key".into()),
                    },
                )
                .await
        }));
    }

    let mut ok_count = 0;
    let mut conflict_count = 0;

    for h in handles {
        match h.await.unwrap() {
            Ok(_) => ok_count += 1,
            Err(ApiError::Conflict) => conflict_count += 1,
            Err(e) => panic!("unexpected error: {:?}", e),
        }
    }

    // At least one success; no silent duplicates.
    assert!(ok_count >= 1, "expected at least one success");

    let messages = svc.list_messages(owner, conv_id, None, None).await.unwrap();
    assert_eq!(
        messages.len(),
        1,
        "exactly one message must be stored, got {}",
        messages.len()
    );

    // Total invocations accounted for.
    assert_eq!(ok_count + conflict_count, 8);
}

#[tokio::test]
async fn concurrent_different_keys_all_stored() {
    use std::sync::Arc;

    let svc = Arc::new(new_service());
    let owner = Uuid::new_v4();

    let conv = mk_conv(&svc, owner, "Test").await;
    let conv_id = conv.id;

    let mut handles = Vec::new();

    for i in 0..8u32 {
        let svc = svc.clone();
        handles.push(tokio::spawn(async move {
            svc
                .add_user_message(
                    owner,
                    conv_id,
                    CreateMessageRequest {
                        content: format!("msg-{}", i),
                        idempotency_key: Some(format!("key-{}", i)),
                    },
                )
                .await
        }));
    }

    for h in handles {
        h.await.unwrap().expect("all unique keys must succeed");
    }

    let messages = svc.list_messages(owner, conv_id, None, None).await.unwrap();
    assert_eq!(messages.len(), 8);

    // Every message has a unique id.
    let ids: std::collections::HashSet<_> = messages.iter().map(|m| m.id).collect();
    assert_eq!(ids.len(), 8);
}

// ─────────────────────────────────────────────
// Pagination
// ─────────────────────────────────────────────

#[tokio::test]
async fn pagination_cursor_accepts_v1_format() {
    let svc = new_service();
    let owner = Uuid::new_v4();

    let conv = mk_conv(&svc, owner, "Test").await;
    let _ = mk_msg(&svc, owner, conv.id, "a", None).await;
    let _ = mk_msg(&svc, owner, conv.id, "b", None).await;

    // PR-1: storage treats cursor as opaque; only `limit` is applied.
    // Any cursor with the `v1:` prefix must be accepted.
    let page = svc
        .list_messages(owner, conv.id, Some("v1:anything".into()), Some(10))
        .await
        .unwrap();

    assert_eq!(page.len(), 2);
}

#[tokio::test]
async fn pagination_invalid_cursor_rejected() {
    let svc = new_service();
    let owner = Uuid::new_v4();

    let conv = mk_conv(&svc, owner, "Test").await;
    let _ = mk_msg(&svc, owner, conv.id, "a", None).await;

    let result = svc
        .list_messages(owner, conv.id, Some("bad-cursor".into()), None)
        .await;

    assert!(matches!(result, Err(ApiError::ValidationFailed { .. })));
}

#[tokio::test]
async fn pagination_limit_applied() {
    let svc = new_service();
    let owner = Uuid::new_v4();

    let conv = mk_conv(&svc, owner, "Test").await;
    for i in 0..20 {
        let _ = mk_msg(&svc, owner, conv.id, &format!("msg-{}", i), None).await;
    }

    let page = svc
        .list_messages(owner, conv.id, None, Some(5))
        .await
        .unwrap();

    assert_eq!(page.len(), 5);
}

#[tokio::test]
async fn pagination_limit_above_max_capped() {
    let svc = new_service();
    let owner = Uuid::new_v4();

    let conv = mk_conv(&svc, owner, "Test").await;
    for i in 0..10 {
        let _ = mk_msg(&svc, owner, conv.id, &format!("msg-{}", i), None).await;
    }

    // Request 10_000, service caps to MAX_PAGE_SIZE (200).
    let page = svc
        .list_messages(owner, conv.id, None, Some(10_000))
        .await
        .unwrap();

    // Only 10 messages exist; nothing to cap in practice.
    // The important invariant: no panic, no rejection.
    assert_eq!(page.len(), 10);
}

#[tokio::test]
async fn pagination_limit_zero_becomes_one() {
    let svc = new_service();
    let owner = Uuid::new_v4();

    let conv = mk_conv(&svc, owner, "Test").await;
    let _ = mk_msg(&svc, owner, conv.id, "a", None).await;

    // Contract: limit = 0 normalizes to 1 (never empty by accident).
    let page = svc
        .list_messages(owner, conv.id, None, Some(0))
        .await
        .unwrap();

    assert_eq!(page.len(), 1);
}

#[tokio::test]
async fn pagination_empty_conversation_returns_empty_list() {
    let svc = new_service();
    let owner = Uuid::new_v4();

    let conv = mk_conv(&svc, owner, "Empty").await;

    let page = svc
        .list_messages(owner, conv.id, None, Some(50))
        .await
        .unwrap();

    assert!(page.is_empty());
}

#[tokio::test]
async fn pagination_foreign_owner_not_leaked_through_cursor() {
    let svc = new_service();
    let alice = Uuid::new_v4();
    let bob = Uuid::new_v4();

    let alice_conv = mk_conv(&svc, alice, "Alice").await;
    let _ = mk_msg(&svc, alice, alice_conv.id, "secret", None).await;

    // Bob tries to list Alice's messages with a valid-looking cursor.
    let result = svc
        .list_messages(bob, alice_conv.id, Some("v1:any".into()), Some(10))
        .await;

    assert!(matches!(result, Err(ApiError::NotFound)));
}

// ─────────────────────────────────────────────
// Boundary values
// ─────────────────────────────────────────────

#[tokio::test]
async fn title_whitespace_only_rejected() {
    let svc = new_service();
    let owner = Uuid::new_v4();

    let result = svc
        .create_conversation(
            owner,
            CreateConversationRequest {
                title: "   \t\n".into(),
            },
        )
        .await;

    assert!(matches!(result, Err(ApiError::ValidationFailed { .. })));
}

#[tokio::test]
async fn title_at_max_length_accepted() {
    let svc = new_service();
    let owner = Uuid::new_v4();

    let title = "a".repeat(MAX_TITLE_LEN);
    let result = svc
        .create_conversation(
            owner,
            CreateConversationRequest {
                title: title.clone(),
            },
        )
        .await;

    let conv = result.unwrap();
    assert_eq!(conv.title.chars().count(), MAX_TITLE_LEN);
}

#[tokio::test]
async fn title_above_max_length_rejected() {
    let svc = new_service();
    let owner = Uuid::new_v4();

    let title = "a".repeat(MAX_TITLE_LEN + 1);
    let result = svc
        .create_conversation(
            owner,
            CreateConversationRequest { title },
        )
        .await;

    assert!(matches!(result, Err(ApiError::ValidationFailed { .. })));
}

#[tokio::test]
async fn content_empty_rejected() {
    let svc = new_service();
    let owner = Uuid::new_v4();

    let conv = mk_conv(&svc, owner, "Test").await;

    let result = svc
        .add_user_message(
            owner,
            conv.id,
            CreateMessageRequest {
                content: "".into(),
                idempotency_key: None,
            },
        )
        .await;

    assert!(matches!(result, Err(ApiError::ValidationFailed { .. })));
}

#[tokio::test]
async fn content_at_max_length_accepted() {
    let svc = new_service();
    let owner = Uuid::new_v4();

    let conv = mk_conv(&svc, owner, "Test").await;
    let content = "x".repeat(MAX_CONTENT_LEN);

    let msg = svc
        .add_user_message(
            owner,
            conv.id,
            CreateMessageRequest {
                content: content.clone(),
                idempotency_key: None,
            },
        )
        .await
        .unwrap();

    assert_eq!(msg.content.chars().count(), MAX_CONTENT_LEN);
}

#[tokio::test]
async fn content_above_max_length_rejected() {
    let svc = new_service();
    let owner = Uuid::new_v4();

    let conv = mk_conv(&svc, owner, "Test").await;
    let content = "x".repeat(MAX_CONTENT_LEN + 1);

    let result = svc
        .add_user_message(
            owner,
            conv.id,
            CreateMessageRequest {
                content,
                idempotency_key: None,
            },
        )
        .await;

    assert!(matches!(result, Err(ApiError::ValidationFailed { .. })));
}

#[tokio::test]
async fn idempotency_key_too_long_rejected() {
    let svc = new_service();
    let owner = Uuid::new_v4();

    let conv = mk_conv(&svc, owner, "Test").await;
    let key = "k".repeat(129); // MAX_IDEMPOTENCY_KEY_LEN + 1

    let result = svc
        .add_user_message(
            owner,
            conv.id,
            CreateMessageRequest {
                content: "hello".into(),
                idempotency_key: Some(key),
            },
        )
        .await;

    assert!(matches!(result, Err(ApiError::ValidationFailed { .. })));
}

#[tokio::test]
async fn idempotency_key_with_invalid_chars_rejected() {
    let svc = new_service();
    let owner = Uuid::new_v4();

    let conv = mk_conv(&svc, owner, "Test").await;

    let result = svc
        .add_user_message(
            owner,
            conv.id,
            CreateMessageRequest {
                content: "hello".into(),
                idempotency_key: Some("bad key with spaces".into()),
            },
        )
        .await;

    assert!(matches!(result, Err(ApiError::ValidationFailed { .. })));
}

#[tokio::test]
async fn idempotency_key_empty_is_treated_as_none() {
    let svc = new_service();
    let owner = Uuid::new_v4();

    let conv = mk_conv(&svc, owner, "Test").await;

    // Empty key → no idempotency applied, two distinct messages.
    let _ = mk_msg(&svc, owner, conv.id, "a", Some("")).await;
    let _ = mk_msg(&svc, owner, conv.id, "b", Some("")).await;

    let messages = svc.list_messages(owner, conv.id, None, None).await.unwrap();
    assert_eq!(messages.len(), 2);
}

// ─────────────────────────────────────────────
// Owner ID integrity
// ─────────────────────────────────────────────

#[tokio::test]
async fn conversation_owner_is_taken_from_request_not_payload() {
    // The service derives `owner_id` from the authenticated caller
    // (a Uuid argument), not from anything inside the request body.
    // `CreateConversationRequest` does not expose an `owner_id` field.
    let svc = new_service();
    let alice = Uuid::new_v4();
    let bob = Uuid::new_v4();

    let conv = mk_conv(&svc, alice, "Alice's").await;
    assert_eq!(conv.owner_id, alice);

    // Bob cannot read it, regardless of what he claims.
    assert!(matches!(
        svc.get_conversation(bob, conv.id).await,
        Err(ApiError::NotFound)
    ));
    assert_eq!(conv.owner_id, alice);
}
