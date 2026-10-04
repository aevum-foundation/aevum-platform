//! Public Intelligence renderer.
//!
//! Deterministic HTML generated from the Growth API.
//! No LLM, no static files, no second source of truth.
//!
//! # Scope
//!
//! Public surface of the Growth subsystem:
//!
//! - `/growth`               — index
//! - `/growth/topics`        — list of topics
//! - `/growth/topics/{topic}` — topic page (main entry point)
//! - `/growth/opportunities` — list of current signals
//! - `/sitemap.xml`          — only URLs that actually exist
//! - `/robots.txt`           — crawler policy
//!
//! # Design integration
//!
//! All pages use the Aevum website design system:
//!
//! - `/css/theme.css`       — colors, tokens
//! - `/css/app-shell.css`   — header, footer, layout
//! - `/css/components.css`  — buttons, cards
//! - `/js/theme.js`         — theme toggle
//! - `/js/components.js`    — loads header.html and footer.html
//! - `/js/navigation.js`    — side nav
//!
//! The header and footer are injected by `boot()` from
//! `/js/components.js`, matching the pattern used by every other
//! page on aevumchain.com.
//!
//! # Canonical URL
//!
//! All absolute URLs are built from a single resolver:
//!
//! ```text
//! AEVUM_PUBLIC_URL (env) → default https://aevumchain.com
//! ```
//!
//! Do not hardcode the public host anywhere else.
//!
//! # CTA
//!
//! Rendered by `render_cta()`, which reads `PUBLIC_IDENTITY`.
//! Adding a new channel is a one-line change in `PUBLIC_IDENTITY`.

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

const DEFAULT_PUBLIC_URL: &str = "https://aevumchain.com";

pub fn public_url() -> String {
    let raw = std::env::var("AEVUM_PUBLIC_URL")
        .ok()
        .filter(|u| !u.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_PUBLIC_URL.to_owned());
    raw.trim_end_matches('/').to_owned()
}

// ---------------------------------------------------------------------------
// Public identity
// ---------------------------------------------------------------------------

pub struct PublicIdentity {
    pub github: &'static str,
    pub telegram: Option<&'static str>,
    pub x: Option<&'static str>,
}

pub const PUBLIC_IDENTITY: PublicIdentity = PublicIdentity {
    github: "https://github.com/aevum-foundation",
    telegram: None,
    x: None,
};

// ---------------------------------------------------------------------------
// Defaults
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

#[get("/growth")]
pub async fn growth_index(
    service: web::Data<AppGrowthService>,
) -> Result<HttpResponse, ApiError> {
    let topics = service.list_topics().await?;
    let base = public_url();
    let body = render_index_page(&base, &topics);
    Ok(html_response(body))
}

#[get("/growth/topics")]
pub async fn topics_index(
    service: web::Data<AppGrowthService>,
) -> Result<HttpResponse, ApiError> {
    let topics = service.list_topics().await?;
    let base = public_url();
    let body = render_topics_page(&base, &topics);
    Ok(html_response(body))
}

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

fn parse_topic(slug: &str) -> Result<Topic, ApiError> {
    Topic::from_str(slug).ok_or(ApiError::ValidationFailed {
        code: "GROWTH_TOPIC_UNKNOWN",
        message: "Unknown topic slug",
    })
}

// ---------------------------------------------------------------------------
// HTML helpers
// ---------------------------------------------------------------------------

fn html_response(body: String) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(body)
}

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
/// 32589 → "3.25×"
/// 20000 → "2.00×"
/// 10000 → "1.00×"
fn format_ratio_bp(bp: u32) -> String {
    let whole = bp / 10_000;
    let frac = (bp % 10_000) / 100;
    format!("{}.{:02}×", whole, frac)
}

fn format_timestamp(ts: DateTime<Utc>) -> String {
    ts.format("%Y-%m-%d %H:%M UTC").to_string()
}

fn format_date(ts: DateTime<Utc>) -> String {
    ts.format("%Y-%m-%d").to_string()
}

// ---------------------------------------------------------------------------
// Layout shell (design system)
// ---------------------------------------------------------------------------

/// Emit the shared `<head>` + site header + opening container.
///
/// Every public page in this module begins with this and ends with
/// `html_footer()`. The two are a matched pair.
fn html_head(title: &str, description: &str, canonical: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en" data-theme="dark">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<meta name="description" content="{desc}">
<link rel="canonical" href="{canonical}">
<meta property="og:type" content="website">
<meta property="og:site_name" content="Aevum">
<meta property="og:title" content="{title}">
<meta property="og:description" content="{desc}">
<meta property="og:url" content="{canonical}">
<meta property="og:image" content="https://aevumchain.com/img/aevum-og.png">
<meta property="og:image:width" content="1200">
<meta property="og:image:height" content="630">
<meta name="twitter:card" content="summary_large_image">
<meta name="twitter:title" content="{title}">
<meta name="twitter:description" content="{desc}">
<meta name="twitter:image" content="https://aevumchain.com/img/aevum-og.png">
<meta name="theme-color" content="&#35;061426">
<link rel="stylesheet" href="/css/theme.css">
<link rel="stylesheet" href="/css/app-shell.css">
<link rel="stylesheet" href="/css/components.css">
<script src="/js/theme.js"></script>
</head>
<body>

<div id="header-container" aria-label="Site header"></div>

<main id="main-content">
<div class="container" style="padding: 48px 24px 64px; max-width: var(--container-max, 1280px); margin: 0 auto;">
"#,
        title = html_escape(title),
        desc = html_escape(description),
        canonical = html_escape(canonical),
    )
}

/// Emit the closing container, site footer, and bootstrap scripts.
fn html_footer() -> String {
    r#"
</div><!-- /container -->
</main>

<div id="footer-container"></div>

<script type="module">
    import { boot } from '/js/components.js';
    import { initNav } from '/js/navigation.js';

    await boot();
    await initNav();
</script>

</body>
</html>
"#
    .to_owned()
}

// ---------------------------------------------------------------------------
// Shared components
// ---------------------------------------------------------------------------

fn render_cta() -> String {
    let mut actions = String::new();
    actions.push_str(&format!(
        r#"<a class="btn btn-primary" href="{}">GitHub</a>"#,
        html_escape(PUBLIC_IDENTITY.github),
    ));
    if let Some(url) = PUBLIC_IDENTITY.telegram {
        actions.push_str(&format!(
            r#" <a class="btn btn-secondary" href="{}">Telegram</a>"#,
            html_escape(url),
        ));
    }
    if let Some(url) = PUBLIC_IDENTITY.x {
        actions.push_str(&format!(
            r#" <a class="btn btn-secondary" href="{}">X</a>"#,
            html_escape(url),
        ));
    }

    format!(
        r#"<section style="margin-top: 64px; padding: 32px; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: var(--color-bg-surface);">
<h2 style="margin-top: 0;">Follow Aevum Intelligence</h2>
<p style="color: var(--color-text-secondary);">New signals, technology trends, and original analysis. Telegram and X are coming soon.</p>
<p style="margin-bottom: 0;">{actions}</p>
</section>"#,
        actions = actions,
    )
}

fn topic_link(base: &str, slug: &str) -> String {
    format!(
        r#"<a href="{base}/growth/topics/{slug}" style="color: var(--color-accent-cyan); text-decoration: none;">{slug}</a>"#,
        base = html_escape(base),
        slug = html_escape(slug),
    )
}

fn ratio_badge(bp: u32, accelerating: bool) -> String {
    let ratio = format_ratio_bp(bp);
    if accelerating {
        format!(
            r#"<span style="color: var(--color-accent-gold); font-weight: 600;">{}</span>"#,
            html_escape(&ratio),
        )
    } else {
        format!(
            r#"<span style="color: var(--color-text-secondary);">{}</span>"#,
            html_escape(&ratio),
        )
    }
}

// ---------------------------------------------------------------------------
// Index page
// ---------------------------------------------------------------------------

fn render_index_page(base: &str, topics: &[TopicSummaryResponse]) -> String {
    let canonical = format!("{}/growth", base);

    let mut rows = String::new();
    for t in topics {
        rows.push_str(&format!(
            r#"<div style="padding: 20px; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-bg-surface);">
<div style="font-size: 20px; font-weight: 600;">{link}</div>
<div style="margin-top: 10px; color: var(--color-text-secondary);">24h: <strong>{c24}</strong> · 7d: <strong>{c7}</strong> · 30d: <strong>{c30}</strong></div>
<div style="margin-top: 6px; color: var(--color-text-muted); font-size: 14px;">7d / 30d ratio: {ratio}</div>
</div>"#,
            link = topic_link(base, &t.topic),
            c24 = t.count_24h,
            c7 = t.count_7d,
            c30 = t.count_30d,
            ratio = ratio_badge(t.ratio_7d_vs_30d_bp, t.accelerating),
        ));
    }

    format!(
        r#"{head}
<section style="padding: 48px 0;">
<div style="color: var(--color-accent-cyan); font-size: 13px; font-weight: 600; letter-spacing: 0.08em; text-transform: uppercase; margin-bottom: 12px;">Intelligence</div>
<h1 style="font-size: 48px; line-height: 1.05; margin: 0 0 16px 0;">Aevum Growth</h1>
<p style="color: var(--color-text-secondary); font-size: 18px; max-width: 720px;">Public intelligence on technology trends observed across open sources.</p>
</section>

<section style="padding: 24px 0;">
<h2>Topics</h2>
<div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(320px, 1fr)); gap: 16px; margin-top: 16px;">
{rows}
</div>
</section>

<p style="margin-top: 32px;"><a href="{base}/growth/opportunities" class="btn btn-secondary">View active signals →</a></p>

{cta}
{footer}"#,
        head = html_head(
            "Aevum Growth",
            "Aevum observes public RSS/Atom sources and publishes topic trends and signals.",
            &canonical,
        ),
        rows = rows,
        base = html_escape(base),
        cta = render_cta(),
        footer = html_footer(),
    )
}

// ---------------------------------------------------------------------------
// Topics index
// ---------------------------------------------------------------------------

fn render_topics_page(base: &str, topics: &[TopicSummaryResponse]) -> String {
    let canonical = format!("{}/growth/topics", base);

    let mut rows = String::new();
    for t in topics {
        rows.push_str(&format!(
            r#"<div style="padding: 20px; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-bg-surface);">
<div style="font-size: 20px; font-weight: 600;">{link}</div>
<div style="margin-top: 10px; color: var(--color-text-secondary);">24h: <strong>{c24}</strong> · 7d: <strong>{c7}</strong> · 30d: <strong>{c30}</strong></div>
<div style="margin-top: 6px; color: var(--color-text-muted); font-size: 14px;">7d / 30d ratio: {ratio}</div>
</div>"#,
            link = topic_link(base, &t.topic),
            c24 = t.count_24h,
            c7 = t.count_7d,
            c30 = t.count_30d,
            ratio = ratio_badge(t.ratio_7d_vs_30d_bp, t.accelerating),
        ));
    }

    format!(
        r#"{head}
<section style="padding: 48px 0;">
<h1 style="font-size: 40px; margin: 0 0 12px 0;">Topics</h1>
<p style="color: var(--color-text-secondary); font-size: 17px; max-width: 720px;">Topics observed by Aevum Growth. Each topic page contains the current trend, recent publications, and active signals.</p>
</section>

<section style="padding: 24px 0;">
<div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(320px, 1fr)); gap: 16px;">
{rows}
</div>
</section>

{cta}
{footer}"#,
        head = html_head(
            "Topics — Aevum Growth",
            "Topics observed by Aevum Growth.",
            &canonical,
        ),
        rows = rows,
        cta = render_cta(),
        footer = html_footer(),
    )
}

// ---------------------------------------------------------------------------
// Topic page
// ---------------------------------------------------------------------------

fn render_topic_page(base: &str, report: &TopicReportResponse) -> String {
    let canonical = format!("{}/growth/topics/{}", base, report.topic);

    let title = format!("{} — Aevum Growth", report.topic);
    let description = format!(
        "Aevum-observed activity on {}: 7d={}, 30d={}, ratio {}.",
        report.topic,
        report.trend.count_7d,
        report.trend.count_30d,
        format_ratio_bp(report.trend.ratio_7d_vs_30d_bp),
    );

    // ─── Hero ───
    let hero = format!(
        r#"<section style="padding: 48px 0;">
<div style="color: var(--color-accent-cyan); font-size: 13px; font-weight: 600; letter-spacing: 0.08em; text-transform: uppercase; margin-bottom: 12px;">Intelligence · Topic</div>
<h1 style="font-size: 48px; line-height: 1.05; margin: 0 0 16px 0;">{topic}</h1>
<p style="color: var(--color-text-secondary); font-size: 18px; max-width: 720px;">Aevum-observed activity across public sources. Updated {updated}.</p>
</section>"#,
        topic = html_escape(&report.topic),
        updated = html_escape(&format_timestamp(report.generated_at)),
    );

    // ─── Activity cards ───
    let ratio_html = if report.trend.accelerating {
        format!(
            r#"{} <small style="color: var(--color-accent-gold);">accelerating</small>"#,
            ratio_badge(report.trend.ratio_7d_vs_30d_bp, true),
        )
    } else {
        ratio_badge(report.trend.ratio_7d_vs_30d_bp, false)
    };

    let activity = format!(
        r#"<section style="padding: 24px 0;">
<h2>Activity</h2>
<div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(160px, 1fr)); gap: 16px; margin-top: 16px;">
<div style="padding: 20px; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-bg-surface);">
<div style="color: var(--color-text-muted); font-size: 12px; text-transform: uppercase;">Last 24h</div>
<div style="font-size: 32px; font-weight: 700; font-variant-numeric: tabular-nums; margin-top: 4px;">{c24}</div>
</div>
<div style="padding: 20px; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-bg-surface);">
<div style="color: var(--color-text-muted); font-size: 12px; text-transform: uppercase;">Last 7d</div>
<div style="font-size: 32px; font-weight: 700; font-variant-numeric: tabular-nums; margin-top: 4px;">{c7}</div>
</div>
<div style="padding: 20px; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-bg-surface);">
<div style="color: var(--color-text-muted); font-size: 12px; text-transform: uppercase;">Last 30d</div>
<div style="font-size: 32px; font-weight: 700; font-variant-numeric: tabular-nums; margin-top: 4px;">{c30}</div>
</div>
<div style="padding: 20px; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-bg-surface);">
<div style="color: var(--color-text-muted); font-size: 12px; text-transform: uppercase;">7d / 30d ratio</div>
<div style="font-size: 32px; font-weight: 700; font-variant-numeric: tabular-nums; margin-top: 4px;">{ratio}</div>
</div>
</div>
</section>"#,
        c24 = report.trend.count_24h,
        c7 = report.trend.count_7d,
        c30 = report.trend.count_30d,
        ratio = ratio_html,
    );

    // ─── Signals ───
    let mut signals = String::new();
    if report.opportunities.is_empty() {
        signals.push_str(r#"<p style="color: var(--color-text-muted);">No active signals for this topic.</p>"#);
    } else {
        signals.push_str(r#"<div style="display: grid; gap: 12px; margin-top: 16px;">"#);
        for opp in &report.opportunities {
            let kind = format!("{:?}", opp.kind).to_uppercase().replace('_', " ");
            signals.push_str(&format!(
                r#"<div style="padding: 20px; border: 1px solid var(--color-border-gold); border-radius: var(--radius-md); background: var(--color-bg-surface);">
<div style="color: var(--color-accent-gold); font-weight: 700; font-size: 14px; letter-spacing: 0.06em;">{kind}</div>
<div style="margin-top: 8px; color: var(--color-text-primary);">Score: <strong>{score} bp</strong> · Evidence: <strong>{n}</strong> publication(s)</div>
</div>"#,
                kind = html_escape(&kind),
                score = opp.score_bp,
                n = opp.evidence.len(),
            ));
        }
        signals.push_str("</div>");
    }

    // ─── Recent publications ───
    let mut pubs = String::new();
    if report.recent_publications.is_empty() {
        pubs.push_str(r#"<p style="color: var(--color-text-muted);">No recent publications.</p>"#);
    } else {
        pubs.push_str(r#"<div style="display: grid; gap: 10px; margin-top: 16px;">"#);
        for p in report.recent_publications.iter().take(20) {
            let link = p
                .url
                .as_deref()
                .map(|u| {
                    format!(
                        r#"<a href="{}" style="color: var(--color-accent-cyan); text-decoration: none;">{}</a>"#,
                        html_escape(u),
                        html_escape(&p.title),
                    )
                })
                .unwrap_or_else(|| html_escape(&p.title));
            let date = p
                .published_at
                .map(format_date)
                .unwrap_or_else(|| "—".to_owned());
            let author = p
                .author
                .as_deref()
                .map(|a| format!(" · {}", html_escape(a)))
                .unwrap_or_default();
            pubs.push_str(&format!(
                r#"<div style="padding: 14px 18px; border: 1px solid var(--color-border); border-radius: var(--radius-sm); background: var(--color-bg-surface);">
<div>{link}</div>
<div style="margin-top: 6px; color: var(--color-text-muted); font-size: 13px;">{date}{author}</div>
</div>"#,
                link = link,
                date = date,
                author = author,
            ));
        }
        pubs.push_str("</div>");
    }

    // ─── Methodology ───
    let methodology = r#"<p style="color: var(--color-text-secondary);">Aevum Growth observes public RSS and Atom feeds. For each publication, Aevum records a deterministic identifier, classifies it against a small set of technology verticals, and counts activity over 24-hour, 7-day, and 30-day windows. Signals are emitted when recent activity diverges from the 30-day baseline.</p>
<p style="color: var(--color-text-secondary);">This page is generated deterministically from the Aevum Growth snapshot. It contains no editorial content and does not reproduce the source material — it only observes, counts, and links to original sources.</p>"#;

    format!(
        r#"{head}
{hero}
{activity}
<section style="padding: 24px 0;">
<h2>Active signals</h2>
{signals}
</section>
<section style="padding: 24px 0;">
<h2>Recent publications</h2>
{pubs}
</section>
<section style="padding: 24px 0;">
<h2>Methodology</h2>
{methodology}
</section>
{cta}
{footer}"#,
        head = html_head(&title, &description, &canonical),
        hero = hero,
        activity = activity,
        signals = signals,
        pubs = pubs,
        methodology = methodology,
        cta = render_cta(),
        footer = html_footer(),
    )
}

// ---------------------------------------------------------------------------
// Opportunities page
// ---------------------------------------------------------------------------

fn render_opportunities_page(base: &str, opps: &[OpportunityResponse]) -> String {
    let canonical = format!("{}/growth/opportunities", base);

    let mut items = String::new();
    if opps.is_empty() {
        items.push_str(r#"<p style="color: var(--color-text-muted);">No active signals.</p>"#);
    } else {
        items.push_str(r#"<div style="display: grid; gap: 12px; margin-top: 16px;">"#);
        for o in opps {
            let kind = format!("{:?}", o.kind).to_uppercase().replace('_', " ");
            let topic_url = format!("{}/growth/topics/{}", base, o.topic);
            items.push_str(&format!(
                r#"<div style="padding: 20px; border: 1px solid var(--color-border-gold); border-radius: var(--radius-md); background: var(--color-bg-surface);">
<div style="display: flex; justify-content: space-between; align-items: baseline; gap: 16px; flex-wrap: wrap;">
<a href="{topic_url}" style="font-size: 20px; font-weight: 700; color: var(--color-accent-cyan); text-decoration: none;">{topic}</a>
<span style="color: var(--color-accent-gold); font-weight: 700; font-size: 13px; letter-spacing: 0.06em;">{kind}</span>
</div>
<div style="margin-top: 10px; color: var(--color-text-primary);">Score: <strong>{score} bp</strong> · Evidence: <strong>{n}</strong> publication(s)</div>
</div>"#,
                topic_url = html_escape(&topic_url),
                topic = html_escape(&o.topic),
                kind = html_escape(&kind),
                score = o.score_bp,
                n = o.evidence.len(),
            ));
        }
        items.push_str("</div>");
    }

    format!(
        r#"{head}
<section style="padding: 48px 0;">
<div style="color: var(--color-accent-cyan); font-size: 13px; font-weight: 600; letter-spacing: 0.08em; text-transform: uppercase; margin-bottom: 12px;">Intelligence · Signals</div>
<h1 style="font-size: 48px; line-height: 1.05; margin: 0 0 16px 0;">Signals</h1>
<p style="color: var(--color-text-secondary); font-size: 18px; max-width: 720px;">Active signals detected by Aevum Growth across all topics.</p>
</section>

<section style="padding: 24px 0;">
{items}
</section>

{cta}
{footer}"#,
        head = html_head(
            "Signals — Aevum Growth",
            "Active signals detected by Aevum Growth.",
            &canonical,
        ),
        items = items,
        cta = render_cta(),
        footer = html_footer(),
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
        assert!(html.contains("<h1"));
        assert!(html.contains("post_quantum"));
        assert!(html.contains("3.25×"));
        assert!(html.contains("Follow Aevum Intelligence"));
        assert!(html.contains("github.com/aevum-foundation"));
        assert!(html.contains("/css/theme.css"));
        assert!(html.contains("/js/components.js"));
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
    fn cta_renders_github() {
        let cta = render_cta();
        assert!(cta.contains("github.com/aevum-foundation"));
        assert!(cta.contains("btn btn-primary"));
    }
}
