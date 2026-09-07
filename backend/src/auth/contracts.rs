//! Aevum Platform — Authentication API contracts.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::models::UserStatus;

// ============================================================
// REQUEST DTOs
// ============================================================

#[derive(Debug, Clone, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PasswordChangeRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PasswordResetRequest {
    pub email: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PasswordResetConfirm {
    pub token: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmailVerificationRequest {}

#[derive(Debug, Clone, Deserialize)]
pub struct EmailVerificationConfirm {
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailVerificationResponse {
    pub success: bool,
}

// ============================================================
// RESPONSE DTOs
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterResponse {
    pub user_id: Uuid,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub user_id: Uuid,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeResponse {
    pub user_id: Uuid,
    pub email: String,
    pub status: UserStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoutResponse {
    pub success: bool,
}

// ============================================================
// AUTH CONTRACT CONSTANTS
// ============================================================

pub const SESSION_COOKIE_NAME: &str = "__Host-aevum_session";
pub const SESSION_DURATION_DAYS: i64 = 7;
pub const SESSION_DURATION_SECONDS: i64 = SESSION_DURATION_DAYS * 24 * 60 * 60;

// ============================================================
// VALIDATION LIMITS
// ============================================================

pub const MAX_EMAIL_LENGTH: usize = 254;
pub const MIN_PASSWORD_LENGTH: usize = 12;
pub const MAX_PASSWORD_LENGTH: usize = 128;
pub const MAX_USER_AGENT_LENGTH: usize = 512;
pub const MAX_IP_ADDRESS_LENGTH: usize = 64;
pub const PASSWORD_RESET_TOKEN_BYTES: usize = 32;
pub const PASSWORD_RESET_TOKEN_TTL_MINUTES: i64 = 30;

// ============================================================
// CONTRACT VALIDATION HELPERS
// ============================================================

impl RegisterRequest {
    pub fn validate(&self) -> Result<(), ContractValidationError> {
        validate_email(&self.email)?;
        validate_password(&self.password)
    }
}

impl LoginRequest {
    pub fn validate(&self) -> Result<(), ContractValidationError> {
        validate_email(&self.email)?;
        if self.password.is_empty() {
            return Err(ContractValidationError::EmptyPassword);
        }
        if self.password.len() > MAX_PASSWORD_LENGTH {
            return Err(ContractValidationError::PasswordTooLong);
        }
        Ok(())
    }
}

impl PasswordChangeRequest {
    pub fn validate(&self) -> Result<(), ContractValidationError> {
        if self.current_password.is_empty() {
            return Err(ContractValidationError::EmptyPassword);
        }
        validate_password(&self.new_password)?;
        Ok(())
    }
}

impl PasswordResetRequest {
    pub fn validate(&self) -> Result<(), ContractValidationError> {
        validate_email(&self.email)?;
        Ok(())
    }
}

impl PasswordResetConfirm {
    pub fn validate(&self) -> Result<(), ContractValidationError> {
        if self.token.is_empty() {
            return Err(ContractValidationError::EmptyResetToken);
        }
        validate_password(&self.new_password)?;
        Ok(())
    }
}

impl EmailVerificationRequest {
    pub fn validate(&self) -> Result<(), ContractValidationError> {
        Ok(())
    }
}

impl EmailVerificationConfirm {
    pub fn validate(&self) -> Result<(), ContractValidationError> {
        if self.token.is_empty() {
            return Err(ContractValidationError::EmptyVerificationToken);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractValidationError {
    EmptyEmail,
    EmailTooLong,
    EmptyPassword,
    PasswordTooShort,
    PasswordTooLong,
    EmptyResetToken,
    EmptyVerificationToken,
}

fn validate_email(email: &str) -> Result<(), ContractValidationError> {
    if email.is_empty() {
        return Err(ContractValidationError::EmptyEmail);
    }
    if email.len() > MAX_EMAIL_LENGTH {
        return Err(ContractValidationError::EmailTooLong);
    }
    Ok(())
}

fn validate_password(password: &str) -> Result<(), ContractValidationError> {
    if password.is_empty() {
        return Err(ContractValidationError::EmptyPassword);
    }
    if password.len() < MIN_PASSWORD_LENGTH {
        return Err(ContractValidationError::PasswordTooShort);
    }
    if password.len() > MAX_PASSWORD_LENGTH {
        return Err(ContractValidationError::PasswordTooLong);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_registration_request_passes() {
        let request = RegisterRequest {
            email: "user@example.com".to_owned(),
            password: "correct-horse-battery-staple".to_owned(),
        };
        assert!(request.validate().is_ok());
    }

    #[test]
    fn empty_email_is_rejected() {
        let request = RegisterRequest {
            email: String::new(),
            password: "correct-horse-battery-staple".to_owned(),
        };
        assert_eq!(request.validate(), Err(ContractValidationError::EmptyEmail));
    }

    #[test]
    fn short_registration_password_is_rejected() {
        let request = RegisterRequest {
            email: "user@example.com".to_owned(),
            password: "short".to_owned(),
        };
        assert_eq!(
            request.validate(),
            Err(ContractValidationError::PasswordTooShort)
        );
    }
}
