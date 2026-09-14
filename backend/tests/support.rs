//! Shared test support utilities for Aevum Platform integration tests.

use std::sync::Arc;

use actix_web::cookie::Cookie;

use aevum_platform_api::{
    auth::{email::MockEmailProvider, service::AuthService, storage::InMemoryAuthStorage},
    community::{
        notifications::{service::NotificationService, storage::InMemoryNotificationStorage},
        service::CommunityService,
        storage::InMemoryCommunityStorage,
    },
    config::Config,
    state::AppState,
    storage::MockStorage,
};

pub struct TestContext {
    pub email_provider: Arc<MockEmailProvider>,
    pub auth_service: Arc<AuthService<InMemoryAuthStorage>>,
    pub community_service: Arc<CommunityService<InMemoryCommunityStorage>>,
    pub notification_service: Arc<NotificationService<InMemoryNotificationStorage>>,
    pub app_state: AppState,
}

pub async fn create_test_context() -> TestContext {
    let storage = Arc::new(MockStorage::new());
    let config = Config::from_env();
    let app_state = AppState::new(config, storage);

    let auth_storage = InMemoryAuthStorage::new();
    let email_provider = Arc::new(MockEmailProvider::new());
    let auth_service = Arc::new(AuthService::new_with_email_provider(
        auth_storage,
        email_provider.clone(),
    ));

    let community_storage = InMemoryCommunityStorage::new();
    let community_service = Arc::new(CommunityService::new(community_storage));

    let notification_storage = InMemoryNotificationStorage::new();
    let notification_service = Arc::new(NotificationService::with_noop_sink(notification_storage));

    TestContext {
        email_provider,
        auth_service,
        community_service,
        notification_service,
        app_state,
    }
}

/// Extract a cookie value from a `Set-Cookie` header by name.
///
/// Shared across all integration tests so we have one canonical parser.
pub fn extract_cookie(
    response: &actix_web::dev::ServiceResponse<
        actix_web::body::EitherBody<actix_web::body::BoxBody>,
    >,
    name: &str,
) -> Option<String> {
    response
        .headers()
        .get_all("set-cookie")
        .into_iter()
        .filter_map(|value| value.to_str().ok())
        .find(|cookie_str| cookie_str.starts_with(&format!("{}=", name)))
        .and_then(|cookie_str| {
            let cookie = Cookie::parse(cookie_str).ok()?;
            Some(cookie.value().to_string())
        })
}

/// Build a `Cookie` header for a request.
pub fn cookie_header(name: &'static str, value: &str) -> Cookie<'static> {
    Cookie::build(name, value.to_owned()).finish()
}
