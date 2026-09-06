//! Session authentication middleware.
//!
//! AUTH-17 — Storage Agnostic Refactor
//!
//! The middleware depends only on the `Authenticator` trait.
//! It has no knowledge of the concrete storage backend.

use std::{
    rc::Rc,
    sync::Arc,
    task::{Context, Poll},
};

use actix_web::{
    body::MessageBody,
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    web, Error, HttpMessage,
};
use futures_util::future::{ready, LocalBoxFuture, Ready};

use crate::auth::{
    authenticator::Authenticator,
    contracts::SESSION_COOKIE_NAME,
    models::User,
    password::SessionToken,
};

#[derive(Clone)]
pub struct AuthMiddleware {
    auth: web::Data<Arc<dyn Authenticator>>,
}

impl AuthMiddleware {
    pub fn new(auth: web::Data<Arc<dyn Authenticator>>) -> Self {
        Self { auth }
    }
}

pub struct AuthMiddlewareService<T> {
    auth: web::Data<Arc<dyn Authenticator>>,
    service: Rc<T>,
}

impl<T, B> Transform<T, ServiceRequest> for AuthMiddleware
where
    T: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    T::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthMiddlewareService<T>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: T) -> Self::Future {
        ready(Ok(AuthMiddlewareService {
            auth: self.auth.clone(),
            service: Rc::new(service),
        }))
    }
}

impl<T, B> Service<ServiceRequest> for AuthMiddlewareService<T>
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
        let auth = self.auth.clone();
        let service = Rc::clone(&self.service);

        Box::pin(async move {
            let mut req = req;

            let token = req
                .cookie(SESSION_COOKIE_NAME)
                .map(|cookie| cookie.value().to_owned());

            if let Some(token) = token {
                let session_token = SessionToken::from_secret(token);

                match auth.authenticate(&session_token).await {
                    Ok(Some(user)) => {
                        req.extensions_mut().insert::<User>(user);
                    }
                    Ok(None) => {
                        // Invalid, expired, revoked, or missing user:
                        // continue as anonymous.
                    }
                    Err(error) => {
                        // Storage/internal failure must NOT be converted
                        // into an anonymous request.
                        return Err(error.into());
                    }
                }
            }

            service.call(req).await
        })
    }
}
