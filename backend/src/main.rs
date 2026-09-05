//! Aevum Platform API — entry point.

use actix_web::{middleware::Logger, App, HttpServer};
use env_logger::Env;

use aevum_platform_api::{
    api::{self, health},
    auth::{middleware::AuthMiddleware, service::AuthService, storage::InMemoryAuthStorage},
    config::Config,
    state::AppState,
    storage::MockStorage,
};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let config = Config::from_env();
    let host = config.host.clone();
    let port = config.port;

    log::info!("Starting Aevum Platform API v{}", env!("CARGO_PKG_VERSION"));
    log::info!("Environment: {}", config.environment);
    log::info!("Listening on {}:{}", host, port);

    let storage = std::sync::Arc::new(MockStorage::new());
    let app_state = AppState::new(config, storage);

    // Auth service for middleware
    let auth_storage = InMemoryAuthStorage::new();
    let auth_service = AuthService::new(auth_storage);
    let auth_service_data = actix_web::web::Data::new(auth_service);

    HttpServer::new(move || {
        App::new()
            .app_data(actix_web::web::Data::new(app_state.clone()))
            .app_data(auth_service_data.clone())
            .wrap(Logger::default())
            .wrap(AuthMiddleware::new(auth_service_data.clone()))
            .configure(health::configure)
            .configure(api::auth::configure)
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}
