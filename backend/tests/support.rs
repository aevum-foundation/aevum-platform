//! Shared test support utilities for Aevum Platform integration tests.

use std::sync::Arc;

use aevum_platform_api::{
    auth::{
        email::MockEmailProvider,
        service::AuthService,
        storage::InMemoryAuthStorage,
    },
    config::Config,
    state::AppState,
    storage::MockStorage,
};

pub struct TestContext {
    pub email_provider: Arc<MockEmailProvider>,
    pub auth_service: Arc<AuthService<InMemoryAuthStorage>>,
    pub app_state: AppState,
}

pub async fn create_test_context() -> TestContext {
    let storage = Arc::new(MockStorage::new());
    let config = Config::from_env();
    let app_state = AppState::new(config, storage);

    let auth_storage = InMemoryAuthStorage::new();
    let email_provider = Arc::new(MockEmailProvider::new());
    let auth_service = Arc::new(
        AuthService::new_with_email_provider(
            auth_storage,
            email_provider.clone(),
        )
    );

    TestContext {
        email_provider,
        auth_service,
        app_state,
    }
}
