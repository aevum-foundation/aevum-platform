use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: ErrorBody,
}

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub code: &'static str,
    pub message: &'static str,
}

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
        HttpResponse::build(self.status_code()).json(body)
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
