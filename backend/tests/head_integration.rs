//! HEAD request contract for public backend routes.
//!
//! Every public GET route must also respond to HEAD with the same status
//! code and headers, and an empty body. This test opens a temporary
//! AevumDB, builds the Growth service against it, and asserts the
//! contract for both `/api/growth/*` and `/growth/*` surfaces.
//!
//! See docs/architecture/intelligence-integration-v1.md Section 5.8.

use std::sync::Arc;

use actix_web::{http::StatusCode, middleware::from_fn, test, App};

use aevum_platform_api::api::{growth as api_growth, public as api_public};
use aevum_platform_api::growth::aevumdb::AevumDbGrowthStorage;
use aevum_platform_api::growth::api_impl::into_app;
use aevum_platform_api::growth::service::GrowthService;

// ---------------------------------------------------------------------------
// Fixture
// ---------------------------------------------------------------------------

fn build_app() -> App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    let tmp = tempfile::tempdir().expect("tempdir");
    // Leak the tempdir: the process is short-lived (a single test binary),
    // and the AevumDB handle must outlive the App.
    let dir = tmp.keep();

    let storage = AevumDbGrowthStorage::open(
        aevum_db::DbConfig::plaintext(dir),
        aevum_db::DbRuntime::plaintext(),
    )
    .expect("open AevumDB");

    let service = Arc::new(GrowthService::new(storage).expect("build GrowthService"));
    let app_service = into_app(service);

    App::new()
        .wrap(from_fn(
            aevum_platform_api::http::head::normalize_head_response,
        ))
        .app_data(actix_web::web::Data::new(app_service))
        .configure(api_growth::configure)
        .configure(api_public::configure)
}

// ---------------------------------------------------------------------------
// HEAD contract — public GET routes must also answer HEAD with 200.
// ---------------------------------------------------------------------------

async fn assert_head_ok(uri: &str) {
    let app = test::init_service(build_app()).await;

    let req = test::TestRequest::default()
        .method(actix_web::http::Method::HEAD)
        .uri(uri)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "HEAD {} must return 200",
        uri
    );

    let body = test::read_body(resp).await;
    assert!(
        body.is_empty(),
        "HEAD {} must return empty body, got {} bytes",
        uri,
        body.len()
    );
}

#[actix_web::test]
async fn head_health_ok() {
    assert_head_ok("/api/growth/health").await;
}

#[actix_web::test]
async fn head_topics_ok() {
    assert_head_ok("/api/growth/topics").await;
}

#[actix_web::test]
async fn head_topic_report_ok() {
    assert_head_ok("/api/growth/topics/post_quantum").await;
}

#[actix_web::test]
async fn head_opportunities_ok() {
    assert_head_ok("/api/growth/opportunities").await;
}

#[actix_web::test]
async fn head_sources_ok() {
    assert_head_ok("/api/growth/sources").await;
}

#[actix_web::test]
async fn head_growth_index_ok() {
    assert_head_ok("/growth").await;
}

#[actix_web::test]
async fn head_growth_topics_ok() {
    assert_head_ok("/growth/topics").await;
}

#[actix_web::test]
async fn head_growth_topic_page_ok() {
    assert_head_ok("/growth/topics/post_quantum").await;
}

#[actix_web::test]
async fn head_growth_opportunities_ok() {
    assert_head_ok("/growth/opportunities").await;
}

#[actix_web::test]
async fn head_sitemap_ok() {
    assert_head_ok("/sitemap.xml").await;
}

#[actix_web::test]
async fn head_robots_ok() {
    assert_head_ok("/robots.txt").await;
}

// ---------------------------------------------------------------------------
// GET is unchanged: same routes answer GET with 200 as before.
// ---------------------------------------------------------------------------

async fn assert_get_ok(uri: &str) {
    let app = test::init_service(build_app()).await;

    let req = test::TestRequest::get().uri(uri).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "GET {} must return 200", uri);
}

#[actix_web::test]
async fn get_health_ok() {
    assert_get_ok("/api/growth/health").await;
}

#[actix_web::test]
async fn get_topics_ok() {
    assert_get_ok("/api/growth/topics").await;
}

#[actix_web::test]
async fn get_growth_index_ok() {
    assert_get_ok("/growth").await;
}
