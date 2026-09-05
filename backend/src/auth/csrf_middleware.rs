//! CSRF enforcement middleware.
//!
//! AUTH-13 — CSRF Protection
//!
//! This middleware validates CSRF tokens for state-changing requests
//! (POST, PUT, PATCH, DELETE). Exempt paths are public authentication
//! endpoints where no CSRF cookie exists yet.

use std::{
    rc::Rc,
    task::{Context, Poll},
};

use actix_web::{
    body::MessageBody,
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    http::Method,
    web, Error, HttpMessage,
};
use futures_util::future::{ready, LocalBoxFuture, Ready};

use crate::auth::csrf::{validate_csrf_tokens, CSRF_COOKIE_NAME, CSRF_HEADER_NAME};
use crate::auth::models::User;
use crate::error::ApiError;

/// Paths exempt from CSRF validation (pre-authentication endpoints).
const CSRF_EXEMPT_PATHS: &[&str] = &[
    "/api/v1/auth/login",
    "/api/v1/auth/register",
    "/api/v1/auth/logout",
];

#[derive(Clone)]
pub struct CsrfMiddleware {
    config: web::Data<crate::auth::csrf::CsrfConfig>,
}

impl CsrfMiddleware {
    pub fn new(config: web::Data<crate::auth::csrf::CsrfConfig>) -> Self {
        Self { config }
    }

    fn is_exempt(path: &str) -> bool {
        CSRF_EXEMPT_PATHS.iter().any(|exempt| *exempt == path)
    }
}

pub struct CsrfMiddlewareService<T> {
    service: Rc<T>,
}

impl<T, B> Transform<T, ServiceRequest> for CsrfMiddleware
where
    T: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    T::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = CsrfMiddlewareService<T>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: T) -> Self::Future {
        ready(Ok(CsrfMiddlewareService {
            service: Rc::new(service),
        }))
    }
}

impl<T, B> Service<ServiceRequest> for CsrfMiddlewareService<T>
where
    T: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    T::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = Rc::clone(&self.service);

        Box::pin(async move {
            if requires_csrf(req.method()) && !CsrfMiddleware::is_exempt(req.path()) {
                validate_csrf(&req)?;
            }

            service.call(req).await
        })
    }
}

fn requires_csrf(method: &Method) -> bool {
    matches!(
        method,
        &Method::POST | &Method::PUT | &Method::PATCH | &Method::DELETE
    )
}

fn validate_csrf(req: &ServiceRequest) -> Result<(), Error> {
    let cookie_token = req
        .cookie(CSRF_COOKIE_NAME)
        .map(|cookie| cookie.value().to_owned());

    let header_token = req
        .headers()
        .get(CSRF_HEADER_NAME)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_owned());

    match (cookie_token, header_token) {
        (Some(cookie), Some(header)) if validate_csrf_tokens(&cookie, &header) => Ok(()),
        _ => Err(ApiError::Forbidden.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csrf_required_only_for_mutating_methods() {
        assert!(requires_csrf(&Method::POST));
        assert!(requires_csrf(&Method::PUT));
        assert!(requires_csrf(&Method::PATCH));
        assert!(requires_csrf(&Method::DELETE));
        assert!(!requires_csrf(&Method::GET));
        assert!(!requires_csrf(&Method::HEAD));
        assert!(!requires_csrf(&Method::OPTIONS));
    }

    #[test]
    fn login_is_exempt_from_csrf() {
        assert!(CsrfMiddleware::is_exempt("/api/v1/auth/login"));
        assert!(CsrfMiddleware::is_exempt("/api/v1/auth/register"));
        assert!(CsrfMiddleware::is_exempt("/api/v1/auth/logout"));
        assert!(!CsrfMiddleware::is_exempt("/api/v1/wallet/transfer"));
    }
}
