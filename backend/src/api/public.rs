//! Public Intelligence renderer.
//!
//! Deterministic HTML generated from the Growth API.
//! No LLM, no static files, no second source of truth.
//!
//! # Scope
//!
//! This module renders the public surface of the Growth subsystem:
//!
//! - `/growth`               — index
//! - `/growth/topics`        — list of topics
//! - `/growth/topics/{topic}` — topic page (the main entry point)
//! - `/growth/opportunities` — list of current signals
//! - `/sitemap.xml`          — only URLs that actually exist
//! - `/robots.txt`           — crawler policy
//!
//! It does NOT own:
//!
//! - knowledge pages (`/knowledge/*`)
//! - publication pages (`/growth/publications/{id}`)
//! - RSS feeds
//! - social distribution
//!
//! Those are separate layers (Phase 1.5+).
//!
//! # Canonical URL
//!
//! All absolute URLs (canonical, OpenGraph, JSON-LD, sitemap,
//! internal links) are built from a single resolver:
//!
//! ```text
//! AEVUM_PUBLIC_URL  (env)  →  default https://aevumchain.com
//! ```
//!
//! Do not hardcode the public host anywhere else.
//!
//! # CTA
//!
//! Rendered by `render_cta`, which reads `PUBLIC_IDENTITY`. Adding
//! a new channel is a one-line change in `PUBLIC_IDENTITY`.

use actix_web::{get, web, HttpResponse};
use chrono::{DateTime, Utc};

use crate::error::ApiError;
use crate::growth::api::AppGrowthService;
use crate::growth::contracts::{
    OpportunityResponse, TopicReportResponse, TopicSummaryResponse,
};
use crate::growth::models::Topic;

// ---------------------------------------------------------------------------
// Public URL resolver
// ---------------------------------------------------------------------------

/// Default canonical host when `AEVUM_PUBLIC_URL` is unset.
const DEFAULT_PUBLIC_URL: &str = "https://aevumchain.com";

/// Resolve the public base URL.
///
/// Precedence: `AEVUM_PUBLIC_URL` → `DEFAULT_PUBLIC_URL`.
/// The result is trimmed of trailing slashes so that
/// `format!("{}/growth/topics/{}", base, slug)` is always correct.
pub fn public_url() -> String {
    let raw = std::env::var("AEVUM_PUBLIC_URL")
        .ok()
        .filter(|u| !u.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_PUBLIC_URL.to_owned());
    raw.trim_end_matches('/').to_owned()
}

// ---------------------------------------------------------------------------
// Public identity (social / distribution)
// ---------------------------------------------------------------------------

/// Social destinations surfaced by the CTA component.
///
/// This is identity metadata, not URL resolution. Canonical URLs
/// still come from `public_url()`.
pub struct PublicIdentity {
    pub github: &'static str,
    /// `None` renders as "Coming soon" in the CTA.
    pub telegram: Option<&'static str>,
    pub x: Option<&'static str>,
}

pub const PUBLIC_IDENTITY: PublicIdentity = PublicIdentity {
    github: "https://github.com/aevum-foundation",
    telegram: None,
    x: None,
};

// ---------------------------------------------------------------------------
// Defaults / bounds
// ---------------------------------------------------------------------------

const DEFAULT_OPPORTUNITY_LIMIT: usize = 50;
const MAX_OPPORTUNITY_LIMIT: usize = 500;

// ---------------------------------------------------------------------------
// Query
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize)]
pub struct LimitQuery {
    pub limit: Option<usize>,
}

impl LimitQuery {
    fn resolve(self, default: usize, max: usize) -> usize {
        self.limit.unwrap_or(default).min(max)
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /growth — index page.
#[get("/growth")]
pub async fn growth_index(
    service: web::Data<AppGrowthService>,
) -> Result<HttpResponse, ApiError> {
    let topics = service.list_topics().await?;
    let base = public_url();
    let body = render_index_page(&base, &topics);
    Ok(html_response(body))
}

/// GET /growth/topics — list of topics.
#[get("/growth/topics")]
pub async fn topics_index(
    service: web::Data<AppGrowthService>,
) -> Result<HttpResponse, ApiError> {
    let topics = service.list_topics().await?;
    let base = public_url();
    let body = render_topics_page(&base, &topics);
    Ok(html_response(body))
}

/// GET /growth/topics/{topic} — topic page.
#[get("/growth/topics/{topic}")]
pub async fn topic_page(
    service: web::Data<AppGrowthService>,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let slug = path.into_inner();
    let topic = parse_topic(&slug)?;
    let report = service.topic_report(topic).await?;
    let base = public_url();
    let body = render_topic_page(&base, &report);
    Ok(html_response(body))
}

/// GET /growth/opportunities — list of signals.
#[get("/growth/opportunities")]
pub async fn opportunities_page(
    service: web::Data<AppGrowthService>,
    query: web::Query<LimitQuery>,
) -> Result<HttpResponse, ApiError> {
    let limit = query
        .into_inner()
        .resolve(DEFAULT_OPPORTUNITY_LIMIT, MAX_OPPORTUNITY_LIMIT);
    let opps = service.list_opportunities(limit).await?;
    let base = public_url();
    let body = render_opportunities_page(&base, &opps);
    Ok(html_response(body))
}

/// GET /sitemap.xml — only URLs that actually exist.
#[get("/sitemap.xml")]
pub async fn sitemap(
    service: web::Data<AppGrowthService>,
) -> Result<HttpResponse, ApiError> {
    let topics = service.list_topics().await?;
    let base = public_url();
    let body = render_sitemap(&base, &topics);
    Ok(HttpResponse::Ok()
        .content_type("application/xml; charset=utf-8")
        .body(body))
}

/// GET /robots.txt — crawler policy.
#[get("/robots.txt")]
pub async fn robots() -> HttpResponse {
    let body = render_robots();
    HttpResponse::Ok()
        .content_type("text/plain; charset=utf-8")
        .body(body)
}

// ---------------------------------------------------------------------------
// Configure
// ---------------------------------------------------------------------------

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(growth_index)
        .service(topics_index)
        .service(topic_page)
        .service(opportunities_page)
        .service(sitemap)
        .service(robots);
}

// ---------------------------------------------------------------------------
// Topic parsing
// ---------------------------------------------------------------------------

/// Parse a URL slug into a `Topic`.
///
/// Mirrors the parsing rule used by the JSON API. Unknown slugs
/// produce a `ValidationFailed` error that HTTP surfaces as 400.
fn parse_topic(slug: &str) -> Result<Topic, ApiError> {
    Topic::from_str(slug).ok_or(ApiError::ValidationFailed {
        code: "GROWTH_TOPIC_UNKNOWN",
        message: "Unknown topic slug",
    })
}

// ---------------------------------------------------------------------------
// HTML rendering — helpers
// ---------------------------------------------------------------------------

fn html_response(body: String) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(body)
}

/// Escape text for safe inclusion in HTML body or attributes.
fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Format a basis-point ratio as `X.XX×`.
///
/// 32589 → "3.26×"
/// 20000 → "2.00×"
/// 10000 → "1.00×"
///
/// Display-only. No analytics are performed here.
fn format_ratio_bp(bp: u32) -> String {
    let whole = bp / 10_000;
    let frac = (bp % 10_000) / 100; // 2 decimal places
    format!("{}.{:02}×", whole, frac)
}

fn format_timestamp(ts: DateTime<Utc>) -> String {
    ts.format("%Y-%m-%d %H:%M UTC").to_string()
}

// ---------------------------------------------------------------------------
// HTML rendering — layout
// ---------------------------------------------------------------------------

fn html_head(title: &str, description: &str, canonical: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<meta name="description" content="{desc}">
<link rel="canonical" href="{canonical}">
<meta property="og:type" content="website">
<meta property="og:title" content="{title}">
<meta property="og:description" content="{desc}">
<meta property="og:url" content="{canonical}">
<meta name="twitter:card" content="summary">
<style>
body {{ font-family: system-ui, -apple-system, sans-serif; max-width: 760px; margin: 2rem auto; padding: 0 1rem; line-height: 1.55; color: #1a1a1a; }}
h1 {{ font-size: 1.8rem; margin-top: 0; }}
h2 {{ font-size: 1.2rem; margin-top: 2rem; border-bottom: 1px solid #eee; padding-bottom: 0.3rem; }}
a {{ color: #0645ad; text-decoration: none; }}
a:hover {{ text-decoration: underline; }}
dl {{ display: grid; grid-template-columns: max-content max-content; gap: 0.3rem 1.5rem; }}
dt {{ color: #666; }}
dd {{ margin: 0; font-variant-numeric: tabular-nums; }}
ol, ul {{ padding-left: 1.2rem; }}
li {{ margin-bottom: 0.4rem; }}
small {{ color: #666; }}
.accelerating {{ color: #b00020; font-weight: 600; }}
.normal {{ color: #333; }}
.cta {{ margin-top: 3rem; padding: 1.2rem; border: 1px solid #ddd; border-radius: 6px; background: #fafafa; }}
.cta h3 {{ margin-top: 0; font-size: 1rem; }}
.cta ul {{ list-style: none; padding: 0; }}
.cta li {{ margin: 0.4rem 0; }}
.cta .soon {{ color: #999; }}
footer {{ margin-top: 3rem; color: #888; font-size: 0.9rem; }}
</style>
</head>
<body>
"#,
        title = html_escape(title),
        desc = html_escape(description),
        canonical = html_escape(canonical),
    )
}

fn html_footer(base: &str, generated_at: DateTime<Utc>) -> String {
    format!(
        r#"<footer>
<p>Generated by Aevum Growth at {ts}.</p>
<p><a href="{base}/growth">← All topics</a></p>
</footer>
</body>
</html>"#,
        ts = html_escape(&format_timestamp(generated_at)),
        base = html_escape(base),
    )
}

// ---------------------------------------------------------------------------
// HTML rendering — CTA
// ---------------------------------------------------------------------------

fn render_cta() -> String {
    let mut items = String::new();
    items.push_str(&format!(
        r#"<li><a href="{}">GitHub</a> — explore the project</li>"#,
        html_escape(PUBLIC_IDENTITY.github),
    ));
    match PUBLIC_IDENTITY.telegram {
        Some(url) => items.push_str(&format!(
            r#"<li><a href="{}">Telegram</a></li>"#,
            html_escape(url),
        )),
        None => items.push_str(r#"<li><span class="soon">Telegram — coming soon</span></li>"#),
    }
    match PUBLIC_IDENTITY.x {
        Some(url) => items.push_str(&format!(
            r#"<li><a href="{}">X</a></li>"#,
            html_escape(url),
        )),
        None => items.push_str(r#"<li><span class="soon">X — coming soon</span></li>"#),
    }
    format!(
        r#"<section class="cta">
<h3>Follow Aevum Intelligence</h3>
<ul>{items}</ul>
</section>"#,
        items = items,
    )
}

// ---------------------------------------------------------------------------
// HTML rendering — pages
// ---------------------------------------------------------------------------

fn render_index_page(base: &str, topics: &[TopicSummaryResponse]) -> String {
    let now = Utc::now();
    let canonical = format!("{}/growth", base);

    let mut items = String::new();
    for t in topics {
        let class = if t.accelerating { "accelerating" } else { "normal" };
        items.push_str(&format!(
            r#"<li><a href="{base}/growth/topics/{slug}">{slug}</a> — 7d: {c7}, 30d: {c30}, ratio <span class="{class}">{ratio}</span></li>"#,
            base = html_escape(base),
            slug = html_escape(&t.topic),
            c7 = t.count_7d,
            c30 = t.count_30d,
            ratio = html_escape(&format_ratio_bp(t.ratio_7d_vs_30d_bp)),
            class = class,
        ));
    }

    format!(
        r#"{head}
<h1>Aevum Growth</h1>
<p>Public intelligence on technology trends observed across open sources.</p>

<h2>Topics</h2>
<ul>{items}</ul>

<p><a href="{base}/growth/opportunities">View active signals →</a></p>
{cta}
{footer}"#,
        head = html_head(
            "Aevum Growth",
            "Aevum observes public RSS/Atom sources and publishes topic trends and signals.",
            &canonical,
        ),
        items = items,
        base = html_escape(base),
        cta = render_cta(),
        footer = html_footer(base, now),
    )
}

fn render_topics_page(base: &str, topics: &[TopicSummaryResponse]) -> String {
    let now = Utc::now();
    let canonical = format!("{}/growth/topics", base);

    let mut rows = String::new();
    for t in topics {
        let class = if t.accelerating { "accelerating" } else { "normal" };
        rows.push_str(&format!(
            r#"<li><a href="{base}/growth/topics/{slug}">{slug}</a> — 24h: {c24}, 7d: {c7}, 30d: {c30}, ratio <span class="{class}">{ratio}</span></li>"#,
            base = html_escape(base),
            slug = html_escape(&t.topic),
            c24 = t.count_24h,
            c7 = t.count_7d,
            c30 = t.count_30d,
            ratio = html_escape(&format_ratio_bp(t.ratio_7d_vs_30d_bp)),
            class = class,
        ));
    }

    format!(
        r#"{head}
<h1>Topics</h1>
<p>Topics observed by Aevum Growth. Each topic page contains the current trend, recent publications, and active signals.</p>
<ul>{rows}</ul>
{cta}
{footer}"#,
        head = html_head(
            "Topics — Aevum Growth",
            "Topics observed by Aevum Growth.",
            &canonical,
        ),
        rows = rows,
        cta = render_cta(),
        footer = html_footer(base, now),
    )
}

fn render_topic_page(base: &str, report: &TopicReportResponse) -> String {
    let canonical = format!("{}/growth/topics/{}", base, report.topic);
    let class = if report.trend.accelerating {
        "accelerating"
    } else {
        "normal"
    };

    let title = format!("{} — Aevum Growth", report.topic);
    let description = format!(
        "Aevum-observed activity on {}: 7d={}, 30d={}, ratio {}.",
        report.topic,
        report.trend.count_7d,
        report.trend.count_30d,
        format_ratio_bp(report.trend.ratio_7d_vs_30d_bp),
    );

    // Trend
    let trend_html = format!(
        r#"<dl>
<dt>Last 24h</dt><dd>{c24}</dd>
<dt>Last 7d</dt><dd>{c7}</dd>
<dt>Last 30d</dt><dd>{c30}</dd>
<dt>7d / 30d ratio</dt><dd><span class="{class}">{ratio}</span></dd>
</dl>"#,
        c24 = report.trend.count_24h,
        c7 = report.trend.count_7d,
        c30 = report.trend.count_30d,
        ratio = html_escape(&format_ratio_bp(report.trend.ratio_7d_vs_30d_bp)),
        class = class,
    );

    // Signals
    let mut signals = String::new();
    if report.opportunities.is_empty() {
        signals.push_str("<p>No active signals for this topic.</p>");
    } else {
        signals.push_str("<ul>");
        for opp in &report.opportunities {
            signals.push_str(&format!(
                r#"<li><strong>{kind}</strong> — score {score} bp, evidence: {n} publication(s)</li>"#,
                kind = html_escape(&format!("{:?}", opp.kind)),
                score = opp.score_bp,
                n = opp.evidence.len(),
            ));
        }
        signals.push_str("</ul>");
    }

    // Recent publications
    let mut pubs = String::new();
    if report.recent_publications.is_empty() {
        pubs.push_str("<p>No recent publications.</p>");
    } else {
        pubs.push_str("<ol>");
        for p in report.recent_publications.iter().take(20) {
            let link = p
                .url
                .as_deref()
                .map(|u| format!(r#"<a href="{}">{}</a>"#, html_escape(u), html_escape(&p.title)))
                .unwrap_or_else(|| html_escape(&p.title));
            let date = p
                .published_at
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "—".to_owned());
            let author = p
                .author
                .as_deref()
                .map(|a| format!(" · {}", html_escape(a)))
                .unwrap_or_default();
            pubs.push_str(&format!(
                r#"<li>{link} <small>— {date}{author}</small></li>"#,
                link = link,
                date = date,
                author = author,
            ));
        }
        pubs.push_str("</ol>");
    }

    // Methodology
    let methodology = r#"<p>Aevum Growth observes public RSS and Atom feeds. For each
publication, Aevum records a deterministic identifier, classifies it
against a small set of technology verticals, and counts activity over
24-hour, 7-day, and 30-day windows. Signals are emitted when recent
activity diverges from the 30-day baseline.</p>
<p>This page is generated deterministically from the Aevum Growth
snapshot. It contains no editorial content and does not reproduce
the source material — it only observes, counts, and links to
original sources.</p>"#;

    format!(
        r#"{head}
<h1>{topic}</h1>
<p><small>Updated: {updated}</small></p>

<h2>Activity</h2>
{trend}

<h2>Active signals</h2>
{signals}

<h2>Recent publications</h2>
{pubs}

<h2>Methodology</h2>
{methodology}

{cta}
{footer}"#,
        head = html_head(&title, &description, &canonical),
        topic = html_escape(&report.topic),
        updated = html_escape(&format_timestamp(report.generated_at)),
        trend = trend_html,
        signals = signals,
        pubs = pubs,
        methodology = methodology,
        cta = render_cta(),
        footer = html_footer(base, report.generated_at),
    )
}

fn render_opportunities_page(base: &str, opps: &[OpportunityResponse]) -> String {
    let now = Utc::now();
    let canonical = format!("{}/growth/opportunities", base);

    let mut items = String::new();
    if opps.is_empty() {
        items.push_str("<p>No active signals.</p>");
    } else {
        items.push_str("<ul>");
        for o in opps {
            items.push_str(&format!(
                r#"<li><strong>{topic}</strong> — {kind}, score {score} bp, evidence: {n} publication(s)</li>"#,
                topic = html_escape(&o.topic),
                kind = html_escape(&format!("{:?}", o.kind)),
                score = o.score_bp,
                n = o.evidence.len(),
            ));
        }
        items.push_str("</ul>");
    }

    format!(
        r#"{head}
<h1>Signals</h1>
<p>Active signals detected by Aevum Growth.</p>
{items}
{cta}
{footer}"#,
        head = html_head(
            "Signals — Aevum Growth",
            "Active signals detected by Aevum Growth.",
            &canonical,
        ),
        items = items,
        cta = render_cta(),
        footer = html_footer(base, now),
    )
}

// ---------------------------------------------------------------------------
// Sitemap
// ---------------------------------------------------------------------------

fn render_sitemap(base: &str, topics: &[TopicSummaryResponse]) -> String {
    let mut urls = String::new();
    for path in &["/growth", "/growth/topics", "/growth/opportunities"] {
        urls.push_str(&format!(
            "<url><loc>{}{}</loc><changefreq>hourly</changefreq></url>\n",
            html_escape(base),
            path,
        ));
    }
    for t in topics {
        urls.push_str(&format!(
            "<url><loc>{}/growth/topics/{}</loc><changefreq>hourly</changefreq></url>\n",
            html_escape(base),
            html_escape(&t.topic),
        ));
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
{urls}</urlset>"#,
        urls = urls,
    )
}

// ---------------------------------------------------------------------------
// Robots
// ---------------------------------------------------------------------------

fn render_robots() -> String {
    let base = public_url();
    format!(
        r#"User-agent: Googlebot
Allow: /

User-agent: Bingbot
Allow: /

User-agent: OAI-SearchBot
Allow: /

User-agent: GPTBot
Allow: /

User-agent: *
Allow: /

Sitemap: {base}/sitemap.xml
"#,
        base = base,
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_escape_handles_specials() {
        assert_eq!(html_escape("<a&b>"), "&lt;a&amp;b&gt;");
        assert_eq!(html_escape("\"x\" 'y'"), "&quot;x&quot; &#39;y&#39;");
        assert_eq!(html_escape("plain"), "plain");
    }

    #[test]
    fn ratio_format_two_decimals() {
        assert_eq!(format_ratio_bp(32_589), "3.25×");
        assert_eq!(format_ratio_bp(20_000), "2.00×");
        assert_eq!(format_ratio_bp(10_000), "1.00×");
        assert_eq!(format_ratio_bp(0), "0.00×");
        assert_eq!(format_ratio_bp(7_142), "0.71×");
    }

    #[test]
    fn topic_page_contains_canonical_and_topic() {
        let report = TopicReportResponse {
            topic: "post_quantum".to_owned(),
            generated_at: Utc::now(),
            trend: crate::growth::contracts::TopicTrendResponse {
                topic: "post_quantum".to_owned(),
                count_24h: 9,
                count_7d: 73,
                count_30d: 96,
                ratio_7d_vs_30d_bp: 32_589,
                accelerating: true,
                computed_at: Utc::now(),
            },
            recent_publications: vec![],
            opportunities: vec![],
        };
        let html = render_topic_page("https://aevumchain.com", &report);
        assert!(html.contains("https://aevumchain.com/growth/topics/post_quantum"));
        assert!(html.contains("<h1>post_quantum</h1>"));
        assert!(html.contains("3.25×"));
        assert!(html.contains("Follow Aevum Intelligence"));
        assert!(html.contains("github.com/aevum-foundation"));
    }

    #[test]
    fn sitemap_contains_only_existing_urls() {
        let topics = vec![TopicSummaryResponse {
            topic: "post_quantum".to_owned(),
            count_24h: 0,
            count_7d: 0,
            count_30d: 0,
            ratio_7d_vs_30d_bp: 0,
            accelerating: false,
        }];
        let xml = render_sitemap("https://aevumchain.com", &topics);
        assert!(xml.contains("<loc>https://aevumchain.com/growth</loc>"));
        assert!(xml.contains("<loc>https://aevumchain.com/growth/topics</loc>"));
        assert!(xml.contains("<loc>https://aevumchain.com/growth/opportunities</loc>"));
        assert!(xml.contains("<loc>https://aevumchain.com/growth/topics/post_quantum</loc>"));
        assert!(!xml.contains("/knowledge/"));
        assert!(!xml.contains("/publications/"));
    }

    #[test]
    fn robots_lists_search_and_ai_bots() {
        let body = render_robots();
        assert!(body.contains("User-agent: Googlebot"));
        assert!(body.contains("User-agent: Bingbot"));
        assert!(body.contains("User-agent: OAI-SearchBot"));
        assert!(body.contains("User-agent: GPTBot"));
        assert!(body.contains("Sitemap:"));
    }

    #[test]
    fn cta_renders_github_and_coming_soon() {
        let cta = render_cta();
        assert!(cta.contains("github.com/aevum-foundation"));
        assert!(cta.contains("Telegram — coming soon"));
        assert!(cta.contains("X — coming soon"));
    }
}
