//! Aevum Platform API — canonical error model.
//!
//! Rules:
//! - Stable machine-readable error codes.
//! - Human-readable messages are safe for clients.
//! - Internal implementation details never cross the API boundary.
//! - HTTP status is derived from the error variant.
//! - Response shape remains stable across API versions.

use actix_web::{
    http::{header, StatusCode},
    HttpResponse, ResponseError,
};
use serde::Serialize;
use thiserror::Error;

/// Canonical JSON error envelope returned by the Platform API.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: ErrorBody,
}

/// Canonical error body.
#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub code: &'static str,
    pub message: &'static str,
}

/// Platform API application errors.
///
/// Keep this enum focused on externally meaningful failures.
/// Internal errors must not expose database, filesystem, crypto,
/// backtrace, or implementation-specific details.
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Internal server error")]
    Internal,

    #[error("Resource not found")]
    NotFound,

    #[error("Bad request")]
    BadRequest,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden")]
    Forbidden,

    #[error("Conflict")]
    Conflict,

    #[error("Too many requests")]
    RateLimited,

    #[error("Service unavailable")]
    ServiceUnavailable,
}

impl ApiError {
    /// Stable machine-readable error code.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Internal => "INTERNAL_ERROR",
            Self::NotFound => "NOT_FOUND",
            Self::BadRequest => "BAD_REQUEST",
            Self::Unauthorized => "UNAUTHORIZED",
            Self::Forbidden => "FORBIDDEN",
            Self::Conflict => "CONFLICT",
            Self::RateLimited => "RATE_LIMITED",
            Self::ServiceUnavailable => "SERVICE_UNAVAILABLE",
        }
    }

    /// Safe client-facing message.
    pub const fn message(&self) -> &'static str {
        match self {
            Self::Internal => "Internal server error",
            Self::NotFound => "Resource not found",
            Self::BadRequest => "Bad request",
            Self::Unauthorized => "Authentication required",
            Self::Forbidden => "Access denied",
            Self::Conflict => "Resource conflict",
            Self::RateLimited => "Too many requests",
            Self::ServiceUnavailable => "Service temporarily unavailable",
        }
    }

    /// HTTP status associated with the error.
    pub const fn status_code(&self) -> StatusCode {
        match self {
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::BadRequest => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::Conflict => StatusCode::CONFLICT,
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            Self::ServiceUnavailable => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    /// Whether clients may retry the request.
    pub const fn is_retryable(&self) -> bool {
        matches!(self, Self::RateLimited | Self::ServiceUnavailable)
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        self.status_code()
    }

    fn error_response(&self) -> HttpResponse {
        let body = ErrorResponse {
            error: ErrorBody {
                code: self.code(),
                message: self.message(),
            },
        };

        let mut response = HttpResponse::build(self.status_code());

        match self {
            Self::Unauthorized => {
                response.insert_header((header::WWW_AUTHENTICATE, "Bearer"));
            }

            Self::RateLimited => {
                // Conservative default. A future rate-limit layer may
                // replace this with the exact retry interval.
                response.insert_header((header::RETRY_AFTER, "60"));
            }

            _ => {}
        }

        response.json(body)
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
