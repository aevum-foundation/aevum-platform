//! Session authentication middleware.
//!
//! The middleware is intentionally generic over AuthStorage.
//! The concrete storage type is selected at the composition root
//! (production: AevumDbAuthStorage, tests: InMemoryAuthStorage).

use std::{
    rc::Rc,
    task::{Context, Poll},
};

use actix_web::{
    body::MessageBody,
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    web, Error, HttpMessage,
};
use futures_util::future::{ready, LocalBoxFuture, Ready};

use crate::auth::{
    contracts::SESSION_COOKIE_NAME,
    models::User,
    service::{AuthService, AuthStorage},
};

#[derive(Clone)]
pub struct AuthMiddleware<S>
where
    S: AuthStorage + 'static,
{
    auth_service: web::Data<AuthService<S>>,
}

impl<S> AuthMiddleware<S>
where
    S: AuthStorage + 'static,
{
    pub fn new(auth_service: web::Data<AuthService<S>>) -> Self {
        Self { auth_service }
    }
}

pub struct AuthMiddlewareService<S, T>
where
    S: AuthStorage + 'static,
{
    auth_service: web::Data<AuthService<S>>,
    service: Rc<T>,
}

impl<S, T, B> Transform<T, ServiceRequest> for AuthMiddleware<S>
where
    S: AuthStorage + 'static,
    T: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    T::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthMiddlewareService<S, T>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: T) -> Self::Future {
        ready(Ok(AuthMiddlewareService {
            auth_service: self.auth_service.clone(),
            service: Rc::new(service),
        }))
    }
}

impl<S, T, B> Service<ServiceRequest> for AuthMiddlewareService<S, T>
where
    S: AuthStorage + 'static,
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
        let auth_service = self.auth_service.clone();
        let service = Rc::clone(&self.service);

        Box::pin(async move {
            let mut req = req;

            let token = req
                .cookie(SESSION_COOKIE_NAME)
                .map(|cookie| cookie.value().to_owned());

            if let Some(token) = token {
                match auth_service.authenticate(&token).await {
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
