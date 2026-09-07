//! Aevum Platform API — entry point.

use actix_web::{middleware::Logger, App, HttpServer};
use std::time::Duration;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::fmt;

use aevum_platform_api::{
    api::{self, health},
    auth::api::AuthApi,
    auth::csrf::CsrfConfig,
    auth::csrf_middleware::CsrfMiddleware,
    auth::{middleware::AuthMiddleware, service::AuthService, storage::InMemoryAuthStorage},
    config::Config,
    state::AppState,
    storage::MockStorage,
};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    let config = Config::from_env();
    let host = config.host.clone();
    let port = config.port;

    tracing::info!("Starting Aevum Platform API v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("Environment: {}", config.environment);
    tracing::info!("Listening on {}:{}", host, port);

    let storage = std::sync::Arc::new(MockStorage::new());
    let app_state = AppState::new(config, storage);

    // Auth service
    let auth_storage = InMemoryAuthStorage::new();
    let auth_service = std::sync::Arc::new(AuthService::new(auth_storage));

    // For HTTP handlers
    let auth_api: std::sync::Arc<dyn AuthApi> = auth_service.clone();
    let auth_api_data = actix_web::web::Data::new(auth_api);

    // For middleware
    let authenticator: std::sync::Arc<dyn aevum_platform_api::auth::authenticator::Authenticator> =
        auth_service.clone();
    let auth_middleware_data = actix_web::web::Data::new(authenticator);

    let server = HttpServer::new(move || {
        App::new()
            .app_data(actix_web::web::Data::new(app_state.clone()))
            .app_data(auth_api_data.clone())
            .wrap(Logger::default())
            .wrap(AuthMiddleware::new(auth_middleware_data.clone()))
            .wrap(CsrfMiddleware::new(actix_web::web::Data::new(
                CsrfConfig::default(),
            )))
            .configure(health::configure)
            .configure(api::auth::configure)
    })
    .bind((host.as_str(), port))?
    .disable_signals()
    .run();

    let server_handle = server.handle();

    let shutdown_task = tokio::spawn(async move {
        shutdown_signal().await;
        tracing::info!("shutdown signal received");

        // Stop accepting new requests
        server_handle.stop(true).await;
    });

    // Wait for either shutdown signal or server exit
    tokio::select! {
        result = server => {
            if let Err(error) = result {
                tracing::error!("server error: {}", error);
                return Err(error);
            }
        }
        _ = shutdown_task => {
            // Signal received, server already stopped
            tracing::info!("server stopped");
        }
    }

    Ok(())
}

#[cfg(unix)]
async fn shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};

    let mut sigterm = signal(SignalKind::terminate()).expect("failed to register SIGTERM handler");
    let mut sigint = signal(SignalKind::interrupt()).expect("failed to register SIGINT handler");

    tokio::select! {
        _ = sigterm.recv() => {
            tracing::info!("SIGTERM received");
        }
        _ = sigint.recv() => {
            tracing::info!("SIGINT received");
        }
    }
}

#[cfg(not(unix))]
async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("SIGINT received");
}
