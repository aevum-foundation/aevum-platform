//! HTTP HEAD response normalization middleware.
//!
//! This middleware enforces the transport-level invariant that HEAD
//! responses contain no response body and do not advertise a
//! Content-Length value that cannot be transmitted.
//!
//! # Problem
//!
//! Actix-web allows a single handler to serve both GET and HEAD via
//! `#[route("path", method = "GET", method = "HEAD")]`. When a HEAD
//! request is dispatched to such a handler, the handler runs in full
//! and returns a `ServiceResponse` whose body contains the same
//! payload that would have been sent to a GET client.
//!
//! Actix does not send the body on the wire for HEAD (per RFC 9110
//! Section 9.3.2), but it leaves the `Content-Length` header on the
//! response, computed from the body that will not be sent. This
//! produces a mismatched response that some crawlers, health-checkers,
//! and HTTP clients treat as a failed fetch:
//!
//! ```text
//! HEAD /growth HTTP/1.1
//! < HTTP/1.1 200 OK
//! < content-length: 3274
//! ... connection closes without 3274 bytes ...
//! curl: (18) transfer closed with 3274 bytes remaining to read
//! ```
//!
//! It is a real transport defect, not a cosmetic issue.
//!
//! # Fix
//!
//! This middleware applies the following transformation to every
//! HEAD response:
//!
//! 1. Execute the downstream handler normally.
//! 2. Remove `Content-Length` from the response headers.
//! 3. Replace the response body with an explicit empty body.
//! 4. Preserve the status code and all other headers.
//!
//! Non-HEAD requests pass through unchanged (apart from the required
//! conversion to `BoxBody`).
//!
//! The middleware does not rely on any implicit behaviour of the body
//! type or the Actix runtime. The `Content-Length` header is removed
//! explicitly, because it cannot be honoured once the body is dropped.
//!
//! # Scope
//!
//! This middleware is transport policy. It does not know about any
//! specific route, domain, or business concept. It applies to every
//! request whose method is `HEAD`.
//!
//! See `docs/architecture/intelligence-integration-v1.md` Section 5.8.

use actix_http::body::None as NoneBody;
use actix_web::body::{BoxBody, MessageBody};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::http::header::CONTENT_LENGTH;
use actix_web::http::Method;
use actix_web::middleware::Next;
use actix_web::Error;

/// Normalize a HEAD response.
///
/// For non-HEAD requests, the response is passed through unchanged,
/// apart from the required conversion to `BoxBody`.
///
/// For HEAD requests:
///
/// 1. The downstream handler executes normally.
/// 2. `Content-Length` is removed from the response headers.
/// 3. The response body is replaced with an explicit empty body.
/// 4. The response status and all other headers are preserved.
///
/// The middleware prevents a response from advertising bytes that will
/// not be transmitted on the wire.
///
/// # Usage
///
/// ```ignore
/// App::new()
///     .wrap(actix_web::middleware::from_fn(
///         aevum_platform_api::http::head::normalize_head_response,
///     ))
///     // ...
/// ```
pub async fn normalize_head_response<B>(
    req: ServiceRequest,
    next: Next<B>,
) -> Result<ServiceResponse<BoxBody>, Error>
where
    B: MessageBody + 'static,
{
    let is_head = req.method() == Method::HEAD;

    let mut res = next.call(req).await?;

    if !is_head {
        return Ok(res.map_into_boxed_body());
    }

    res.response_mut().headers_mut().remove(CONTENT_LENGTH);

    Ok(res.map_body(|_, _| BoxBody::new(NoneBody::new())))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    use actix_web::body::to_bytes;
    use actix_web::http::{header, StatusCode};
    use actix_web::{test, web, App, HttpResponse, Responder};

    async fn get_handler() -> impl Responder {
        HttpResponse::Ok()
            .content_type("text/plain; charset=utf-8")
            .insert_header(("X-Custom", "yes"))
            .body("hello, world")
    }

    fn build_app() -> App<
        impl actix_web::dev::ServiceFactory<
            actix_web::dev::ServiceRequest,
            Config = (),
            Response = actix_web::dev::ServiceResponse,
            Error = actix_web::Error,
            InitError = (),
        >,
    > {
        App::new()
            .wrap(actix_web::middleware::from_fn(normalize_head_response))
            .service(
                web::resource("/x")
                    .route(web::get().to(get_handler))
                    .route(web::head().to(get_handler)),
            )
    }

    // -----------------------------------------------------------------------
    // GET — must be unchanged by the middleware.
    // -----------------------------------------------------------------------

    #[actix_web::test]
    async fn get_body_is_preserved() {
        let app = test::init_service(build_app()).await;
        let req = test::TestRequest::get().uri("/x").to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::OK);
        let body = to_bytes(resp.into_body()).await.unwrap();
        assert_eq!(&body[..], b"hello, world");
    }

    #[actix_web::test]
    async fn get_content_type_is_preserved() {
        let app = test::init_service(build_app()).await;
        let req = test::TestRequest::get().uri("/x").to_request();
        let resp = test::call_service(&app, req).await;

        let ct = resp
            .headers()
            .get(header::CONTENT_TYPE)
            .expect("GET must have Content-Type");
        assert_eq!(ct, "text/plain; charset=utf-8");
    }

    // -----------------------------------------------------------------------
    // HEAD — must be normalized.
    // -----------------------------------------------------------------------

    #[actix_web::test]
    async fn head_status_is_preserved() {
        let app = test::init_service(build_app()).await;
        let req = test::TestRequest::default()
            .method(Method::HEAD)
            .uri("/x")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn head_content_length_is_removed() {
        let app = test::init_service(build_app()).await;
        let req = test::TestRequest::default()
            .method(Method::HEAD)
            .uri("/x")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert!(
            resp.headers().get(header::CONTENT_LENGTH).is_none(),
            "HEAD must not advertise Content-Length"
        );
    }

    #[actix_web::test]
    async fn head_body_is_empty() {
        let app = test::init_service(build_app()).await;
        let req = test::TestRequest::default()
            .method(Method::HEAD)
            .uri("/x")
            .to_request();
        let resp = test::call_service(&app, req).await;

        let body = to_bytes(resp.into_body()).await.unwrap();
        assert!(body.is_empty(), "HEAD body must be empty");
    }

    #[actix_web::test]
    async fn head_content_type_is_preserved() {
        let app = test::init_service(build_app()).await;
        let req = test::TestRequest::default()
            .method(Method::HEAD)
            .uri("/x")
            .to_request();
        let resp = test::call_service(&app, req).await;

        let ct = resp
            .headers()
            .get(header::CONTENT_TYPE)
            .expect("HEAD must preserve Content-Type");
        assert_eq!(ct, "text/plain; charset=utf-8");
    }

    #[actix_web::test]
    async fn head_other_headers_are_preserved() {
        let app = test::init_service(build_app()).await;
        let req = test::TestRequest::default()
            .method(Method::HEAD)
            .uri("/x")
            .to_request();
        let resp = test::call_service(&app, req).await;

        let custom = resp
            .headers()
            .get("x-custom")
            .expect("HEAD must preserve other headers");
        assert_eq!(custom, "yes");
    }

    // -----------------------------------------------------------------------
    // Non-GET, non-HEAD — must be unchanged.
    // -----------------------------------------------------------------------

    #[actix_web::test]
    async fn post_body_is_preserved() {
        // This test uses a route that responds to POST by mirroring GET.
        // It confirms that methods other than HEAD are not normalized.
        use actix_web::http::Method;

        async fn post_handler() -> impl Responder {
            HttpResponse::Ok().body("posted")
        }

        let app = test::init_service(
            App::new()
                .wrap(actix_web::middleware::from_fn(normalize_head_response))
                .route("/p", web::post().to(post_handler)),
        )
        .await;

        let req = test::TestRequest::default()
            .method(Method::POST)
            .uri("/p")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::OK);
        let body = to_bytes(resp.into_body()).await.unwrap();
        assert_eq!(&body[..], b"posted");
    }
}
