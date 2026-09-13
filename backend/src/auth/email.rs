//! Email provider abstraction.
//!
//! AUTH-21 — Password Reset
//!
//! Production will use a real email service.
//! Development and tests use the mock provider.

use async_trait::async_trait;

use crate::error::ApiError;

/// Sends transactional emails required by the authentication system.
///
/// Implementations:
/// - MockEmailProvider (dev/test)
/// - ProductionEmailProvider (future)
#[async_trait]
pub trait EmailProvider: Send + Sync {
    async fn send_password_reset(&self, email: &str, reset_token: &str) -> Result<(), ApiError>;

    async fn send_email_verification(
        &self,
        email: &str,
        verification_token: &str,
    ) -> Result<(), ApiError>;
}

/// Mock email provider for development and tests.
#[derive(Default)]
pub struct MockEmailProvider {
    pub sent_emails: std::sync::Mutex<Vec<(String, String)>>,
}

impl MockEmailProvider {
    pub fn new() -> Self {
        Self {
            sent_emails: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn sent_count(&self) -> usize {
        self.sent_emails.lock().unwrap().len()
    }
}

#[async_trait]
impl EmailProvider for MockEmailProvider {
    async fn send_email_verification(
        &self,
        email: &str,
        verification_token: &str,
    ) -> Result<(), ApiError> {
        let mut sent = self.sent_emails.lock().unwrap();
        sent.push((email.to_string(), verification_token.to_string()));
        tracing::info!("mock email verification sent to {}", email);
        Ok(())
    }

    async fn send_password_reset(&self, email: &str, reset_token: &str) -> Result<(), ApiError> {
        let mut sent = self.sent_emails.lock().unwrap();
        sent.push((email.to_string(), reset_token.to_string()));
        tracing::info!("mock password reset email sent to {}", email);
        Ok(())
    }
}
