//! HTTP fetcher for Growth ingestion.
//!
//! Responsibility:
//!
//! ```text
//! URL  →  [fetcher.rs]  →  XML bytes (as String)
//! ```
//!
//! Contract:
//!
//! - one GET per URL
//! - 10 second timeout
//! - explicit User-Agent
//! - one retry on transport error (network, timeout, DNS)
//! - NO retry on HTTP error status (4xx / 5xx) — that is the
//!   server's answer, not a transport failure
//! - bounded body read: at most MAX_BODY_BYTES reach memory, even
//!   for chunked responses without Content-Length
//! - returns the response body as `String` (UTF-8)
//! - this module does NOT parse XML; that is `rss.rs`
//! - this module does NOT schedule polls; that is the service layer
//!
//! See `growth-feed-audit-v1.md` section 2.

use std::time::Duration;

use reqwest::Client;

use crate::error::ApiError;

// ---------------------------------------------------------------------------
// Error constants
// ---------------------------------------------------------------------------

const FETCH_TRANSPORT_CODE: &str = "GROWTH_FETCH_TRANSPORT";
const FETCH_TRANSPORT_MSG: &str = "Feed fetch failed (transport)";

const FETCH_STATUS_CODE: &str = "GROWTH_FETCH_STATUS";
const FETCH_STATUS_MSG: &str = "Feed fetch failed (non-success status)";

const FETCH_BODY_CODE: &str = "GROWTH_FETCH_BODY";
const FETCH_BODY_MSG: &str = "Feed response body is not valid or too large";

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// User-Agent sent with every Growth fetch.
///
/// Public Aevum identifier so that feed operators can recognize our
/// traffic in their logs.
const GROWTH_USER_AGENT: &str = concat!(
    "AevumGrowth/0.1 (+https://aevum.foundation; ",
    "growth ingestion bot)"
);

const DEFAULT_TIMEOUT_SECS: u64 = 10;
const MAX_BODY_BYTES: usize = 8 * 1024 * 1024; // 8 MiB

// ---------------------------------------------------------------------------
// Fetcher
// ---------------------------------------------------------------------------

/// HTTP client for Growth feed fetching.
///
/// `Clone` is cheap (`reqwest::Client` is internally `Arc`).
#[derive(Clone)]
pub struct Fetcher {
    client: Client,
}

impl std::fmt::Debug for Fetcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Fetcher").finish_non_exhaustive()
    }
}

impl Fetcher {
    /// Create a new fetcher with the default timeout.
    pub fn new() -> Result<Self, ApiError> {
        Self::with_timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
    }

    /// Create a new fetcher with an explicit timeout.
    ///
    /// Intended for tests and for callers that need a stricter bound.
    pub fn with_timeout(timeout: Duration) -> Result<Self, ApiError> {
        let client = Client::builder()
            .timeout(timeout)
            .user_agent(GROWTH_USER_AGENT)
            .build()
            .map_err(|e| {
                log::error!("Growth fetcher: client build failed: {}", e);
                ApiError::Internal
            })?;
        Ok(Self { client })
    }

    /// Fetch `url` and return the response body as a `String`.
    ///
    /// # Errors
    ///
    /// - `FETCH_TRANSPORT` — network failure after one retry.
    /// - `FETCH_STATUS` — HTTP status is not 2xx (no retry).
    /// - `FETCH_BODY` — body is not valid UTF-8 or exceeds the size cap.
    pub async fn fetch(&self, url: &str) -> Result<String, ApiError> {
        match self.try_once(url).await {
            Ok(body) => Ok(body),
            Err(FetchAttempt::Transport(e)) => {
                log::warn!(
                    "Growth fetcher: transport error on first attempt: {} ({})",
                    url,
                    e
                );
                // One retry on transport error only.
                match self.try_once(url).await {
                    Ok(body) => Ok(body),
                    Err(FetchAttempt::Transport(e2)) => {
                        log::error!("Growth fetcher: transport error on retry: {} ({})", url, e2);
                        Err(api_error(FETCH_TRANSPORT_CODE, FETCH_TRANSPORT_MSG))
                    }
                    Err(FetchAttempt::Status(status)) => {
                        log::warn!(
                            "Growth fetcher: non-success status on retry: {} ({})",
                            url,
                            status
                        );
                        Err(api_error(FETCH_STATUS_CODE, FETCH_STATUS_MSG))
                    }
                    Err(FetchAttempt::Body(msg)) => {
                        log::warn!("Growth fetcher: bad body on retry: {} ({})", url, msg);
                        Err(api_error(FETCH_BODY_CODE, FETCH_BODY_MSG))
                    }
                }
            }
            Err(FetchAttempt::Status(status)) => {
                log::warn!("Growth fetcher: non-success status: {} ({})", url, status);
                Err(api_error(FETCH_STATUS_CODE, FETCH_STATUS_MSG))
            }
            Err(FetchAttempt::Body(msg)) => {
                log::warn!("Growth fetcher: bad body: {} ({})", url, msg);
                Err(api_error(FETCH_BODY_CODE, FETCH_BODY_MSG))
            }
        }
    }

    async fn try_once(&self, url: &str) -> Result<String, FetchAttempt> {
        let mut response = self
            .client
            .get(url)
            .header(
                "Accept",
                "application/atom+xml, application/rss+xml, application/xml, text/xml",
            )
            .send()
            .await
            .map_err(|e| FetchAttempt::Transport(format!("{}", e)))?;

        let status = response.status();
        if !status.is_success() {
            return Err(FetchAttempt::Status(status.as_u16()));
        }

        // Early check when the server declares a size.
        if let Some(len) = response.content_length() {
            if len as usize > MAX_BODY_BYTES {
                return Err(FetchAttempt::Body(format!(
                    "content-length {} exceeds cap {}",
                    len, MAX_BODY_BYTES
                )));
            }
        }

        // Bounded read: never let more than MAX_BODY_BYTES reach memory,
        // even for chunked responses without Content-Length.
        let mut buf: Vec<u8> = Vec::with_capacity(8 * 1024);
        let mut total: usize = 0;
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| FetchAttempt::Transport(format!("chunk: {}", e)))?
        {
            total += chunk.len();
            if total > MAX_BODY_BYTES {
                return Err(FetchAttempt::Body(format!(
                    "body size exceeded cap {}",
                    MAX_BODY_BYTES
                )));
            }
            buf.extend_from_slice(&chunk);
        }

        String::from_utf8(buf).map_err(|e| FetchAttempt::Body(format!("utf8: {}", e)))
    }
}

// ---------------------------------------------------------------------------
// Internal attempt error
// ---------------------------------------------------------------------------

enum FetchAttempt {
    Transport(String),
    Status(u16),
    Body(String),
}

fn api_error(code: &'static str, message: &'static str) -> ApiError {
    ApiError::ValidationFailed { code, message }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fetcher_new_succeeds() {
        let f = Fetcher::new().unwrap();
        let _ = format!("{:?}", f);
    }

    #[test]
    fn fetcher_with_timeout_succeeds() {
        let _ = Fetcher::with_timeout(Duration::from_millis(200)).unwrap();
    }

    #[test]
    fn fetcher_is_clone() {
        let f = Fetcher::new().unwrap();
        let _g = f.clone();
    }

    #[test]
    fn body_cap_is_reasonable() {
        // Sanity: the cap is between 1 MiB and 32 MiB.
        assert!(MAX_BODY_BYTES >= 1 * 1024 * 1024);
        assert!(MAX_BODY_BYTES <= 32 * 1024 * 1024);
    }

    /// Real network test. Ignored by default.
    ///
    /// Run manually with:
    ///
    /// ```text
    /// cargo test --lib growth::ingestion::fetcher -- --ignored --nocapture
    /// ```
    #[tokio::test]
    #[ignore]
    async fn fetch_real_feed() {
        let f = Fetcher::new().unwrap();
        let body = f
            .fetch("https://blog.rust-lang.org/feed.xml")
            .await
            .expect("fetch rust blog feed");
        assert!(body.contains("<feed"), "must contain <feed>");
        assert!(body.len() > 1000, "body should be > 1 KB");
    }

    /// Real network test: 404 must return FETCH_STATUS, not transport.
    #[tokio::test]
    #[ignore]
    async fn fetch_404_returns_status_error() {
        let f = Fetcher::new().unwrap();
        let err = f
            .fetch("https://blog.rust-lang.org/this-does-not-exist-42")
            .await
            .expect_err("expected an error");
        match err {
            ApiError::ValidationFailed { code, .. } => {
                assert_eq!(code, FETCH_STATUS_CODE);
            }
            other => panic!("unexpected error: {:?}", other),
        }
    }
}
