use actix_web::{get, web, HttpResponse, Responder};
use serde::Serialize;

use crate::state::AppState;

#[derive(Debug, Serialize)]
struct ServiceInfo {
    service: &'static str,
    version: &'static str,
    api_version: &'static str,
}

#[derive(Debug, Serialize)]
struct StorageInfo {
    name: &'static str,
    status: &'static str,
    ready: bool,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    service: ServiceInfo,
    uptime_seconds: i64,
    storage: StorageInfo,
}

#[derive(Debug, Serialize)]
struct VersionResponse {
    service: ServiceInfo,
}

#[derive(Debug, Serialize)]
struct ReadinessResponse {
    status: &'static str,
    ready: bool,
    storage: StorageInfo,
}

#[get("/health")]
async fn health(state: web::Data<AppState>) -> impl Responder {
    let storage_status = state.storage.health();

    let response = HealthResponse {
        status: "ok",
        service: ServiceInfo {
            service: "aevum-platform-api",
            version: env!("CARGO_PKG_VERSION"),
            api_version: "v1",
        },
        uptime_seconds: state.uptime().num_seconds().max(0),
        storage: StorageInfo {
            name: state.storage.name(),
            status: storage_status.as_str(),
            ready: storage_status.is_healthy(),
        },
    };

    HttpResponse::Ok().json(response)
}

#[get("/ready")]
async fn ready(state: web::Data<AppState>) -> impl Responder {
    let storage_status = state.storage.health();
    let ready = storage_status.is_healthy();

    let response = ReadinessResponse {
        status: if ready { "ready" } else { "not_ready" },
        ready,
        storage: StorageInfo {
            name: state.storage.name(),
            status: storage_status.as_str(),
            ready,
        },
    };

    if ready {
        HttpResponse::Ok().json(response)
    } else {
        HttpResponse::ServiceUnavailable().json(response)
    }
}

#[get("/version")]
async fn version() -> impl Responder {
    HttpResponse::Ok().json(VersionResponse {
        service: ServiceInfo {
            service: "aevum-platform-api",
            version: env!("CARGO_PKG_VERSION"),
            api_version: "v1",
        },
    })
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            .service(health)
            .service(ready)
            .service(version),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::StorageStatus;

    #[test]
    fn storage_status_is_stable() {
        assert_eq!(StorageStatus::Healthy.as_str(), "healthy");
        assert_eq!(StorageStatus::NotReady.as_str(), "not_ready");
        assert_eq!(StorageStatus::Unavailable.as_str(), "unavailable");
    }
}
