//! Growth operational CLI.
//!
//! One domain — several entrypoints. The HTTP server and this CLI
//! both dispatch to the same `GrowthService`; neither re-implements
//! the pipeline.
//!
//! This binary contains NO business logic. It only:
//!
//! - parses arguments (clap);
//! - constructs an `AevumDbGrowthStorage` at the selected DB path;
//! - dispatches to `GrowthService` / `registry::seed` / `analysis::*`;
//! - renders human-readable output;
//! - sets an exit code.
//!
//! All Growth behaviour lives in `aevum_platform_api::growth`.
//!
//! # DB path resolution
//!
//! Precedence (highest first):
//!
//! 1. `--db-path <path>`
//! 2. `AEVUM_GROWTH_DB` environment variable
//! 3. `./data/growth` (default)
//!
//! There is no separate "growth database". Growth uses the same
//! AevumDB engine as the rest of the platform, under the
//! `growth:*` key namespace.
//!
//! # Phase 1 configuration policy
//!
//! The CLI opens AevumDB in plaintext mode. This is intended for
//! local development and single-host operations. Production
//! encryption wiring (envelope, key epoch, suite id) will follow
//! the platform database configuration once that configuration is
//! exposed as a shared component.
//!
//! Until then, `growth` MUST NOT be treated as a production
//! encrypted deployment tool. It is an operational CLI for Phase 1
//! data collection.
//!
//! # Exit codes
//!
//! - `0` — success.
//! - `1` — general failure, including one or more source failures
//!   during `ingest`. Individual source failures do NOT stop the
//!   run, but they DO make the process fail, so that `systemd`,
//!   `cron`, `CI`, and `Docker` health checks can detect them.
//!
//! # Phase 1 limitations
//!
//! - `trends` and `opportunities` load at most
//!   `PHASE_1_MAX_PUBLICATIONS` most recent publications. This is
//!   correctness-neutral for small datasets but incomplete at
//!   scale. The correct long-term approach is a time-bounded scan
//!   `[now - 30d, now]`, which requires a new AevumDB primitive
//!   (`range_scan` / `list_publications_since`). That primitive is
//!   tracked as Tier-1 backlog.
//! - `opportunities` emits records without evidence IDs. Evidence
//!   wiring belongs to `GrowthService::analyze_opportunities`
//!   (Tier-1).
//! - `sources` and `ingest` fetch at most 1000 sources. Acceptable
//!   for the Phase 1 registry size; pagination is a future
//!   extension.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use aevum_platform_api::growth::aevumdb::AevumDbGrowthStorage;
use aevum_platform_api::growth::analysis::trends::{self, TrendInput};
use aevum_platform_api::growth::models::SourceStatus;
use aevum_platform_api::growth::registry::seed;
use aevum_platform_api::growth::service::GrowthService;
use aevum_platform_api::growth::storage::{PublicationStorage, SourceStorage};

use aevum_db::{DbConfig, DbRuntime};

/// Maximum number of publications loaded for trend computation.
///
/// See module docs ("Phase 1 limitations").
const PHASE_1_MAX_PUBLICATIONS: usize = 10_000;

/// Maximum number of sources listed or ingested per run.
const PHASE_1_MAX_SOURCES: usize = 1_000;

// ---------------------------------------------------------------------------
// CLI definition
// ---------------------------------------------------------------------------

#[derive(Debug, Parser)]
#[command(
    name = "growth",
    version,
    about = "Aevum Growth operational CLI",
    long_about = None,
)]
struct Cli {
    /// AevumDB path. Overrides AEVUM_GROWTH_DB.
    #[arg(long, value_name = "PATH")]
    db_path: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Populate the source registry from the built-in seed list.
    Seed,

    /// List sources currently in the registry.
    Sources,

    /// Ingest every active source in the registry.
    Ingest,

    /// Print the current topic trends.
    Trends,

    /// Print current opportunities.
    Opportunities,
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_target(false)
        .init();

    let cli = Cli::parse();
    let db_path = resolve_db_path(cli.db_path);

    if let Err(err) = run(cli.command, &db_path).await {
        eprintln!("error: {}", err);
        std::process::exit(1);
    }
}

async fn run(
    command: Command,
    db_path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = DbConfig::plaintext(db_path.to_path_buf());
    let runtime = DbRuntime::plaintext();

    let storage = AevumDbGrowthStorage::open(config, runtime)
        .map_err(|e| format!("failed to open AevumDB at {:?}: {:?}", db_path, e))?;

    let service = GrowthService::new(storage)
        .map_err(|e| format!("failed to build GrowthService: {:?}", e))?;

    match command {
        Command::Seed => cmd_seed(&service).await?,
        Command::Sources => cmd_sources(&service).await?,
        Command::Ingest => cmd_ingest(&service).await?,
        Command::Trends => cmd_trends(&service).await?,
        Command::Opportunities => cmd_opportunities(&service).await?,
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

async fn cmd_seed(
    service: &GrowthService<AevumDbGrowthStorage>,
) -> Result<(), Box<dyn std::error::Error>> {
    let n = seed::seed_sources(service.storage())
        .await
        .map_err(|e| format!("seed failed: {:?}", e))?;
    println!("seeded {} sources", n);
    Ok(())
}

async fn cmd_sources(
    service: &GrowthService<AevumDbGrowthStorage>,
) -> Result<(), Box<dyn std::error::Error>> {
    let sources = service
        .storage()
        .list_sources(PHASE_1_MAX_SOURCES)
        .await
        .map_err(|e| format!("list_sources failed: {:?}", e))?;

    println!(
        "{:<44}  {:<6}  {:<10}  {}",
        "feed_url", "kind", "status", "topics"
    );
    println!("{}", "-".repeat(120));
    for s in &sources {
        let topics: Vec<&str> = s.topics.iter().map(|t| t.as_str()).collect();
        println!(
            "{:<44}  {:<6}  {:<10}  {}",
            truncate(s.feed_url.as_str(), 44),
            s.platform.as_str(),
            s.status.as_str(),
            topics.join(","),
        );
    }
    println!("\ntotal: {}", sources.len());
    Ok(())
}

async fn cmd_ingest(
    service: &GrowthService<AevumDbGrowthStorage>,
) -> Result<(), Box<dyn std::error::Error>> {
    let all_sources = service
        .storage()
        .list_sources(PHASE_1_MAX_SOURCES)
        .await
        .map_err(|e| format!("list_sources failed: {:?}", e))?;

    // Lifecycle filtering belongs to the CLI, not to the service.
    // The service ingests whatever source it is given.
    let sources: Vec<_> = all_sources
        .into_iter()
        .filter(|s| s.status == SourceStatus::Active)
        .collect();

    if sources.is_empty() {
        eprintln!("no active sources; run `growth seed` first");
        return Ok(());
    }

    let mut total_parsed = 0usize;
    let mut total_inserted = 0usize;
    let mut total_duplicate = 0usize;
    let mut failures = 0usize;

    for source in &sources {
        match service.ingest_source(source).await {
            Ok(stats) => {
                println!(
                    "{:<44}  parsed={:<4}  inserted={:<4}  dup={:<4}",
                    truncate(source.feed_url.as_str(), 44),
                    stats.parsed,
                    stats.inserted,
                    stats.skipped_duplicate,
                );
                total_parsed += stats.parsed;
                total_inserted += stats.inserted;
                total_duplicate += stats.skipped_duplicate;
            }
            Err(e) => {
                failures += 1;
                eprintln!(
                    "{:<44}  ERROR: {:?}",
                    truncate(source.feed_url.as_str(), 44),
                    e,
                );
            }
        }
    }

    println!(
        "\nsummary: parsed={} inserted={} duplicates={} failures={}",
        total_parsed, total_inserted, total_duplicate, failures,
    );

    if failures > 0 {
        // Exit non-zero for automation (systemd, cron, CI, Docker
        // health). Individual source failures do NOT stop the run,
        // but they DO make the process fail.
        return Err(format!(
            "ingestion completed with {} source failure(s)",
            failures
        )
        .into());
    }
    Ok(())
}

async fn cmd_trends(
    service: &GrowthService<AevumDbGrowthStorage>,
) -> Result<(), Box<dyn std::error::Error>> {
    let inputs = load_trend_inputs(service).await?;
    let now = chrono::Utc::now();
    let trends = trends::compute_all(&inputs, now);

    println!(
        "{:<24}  {:>6}  {:>6}  {:>6}  {:>10}",
        "topic", "24h", "7d", "30d", "ratio_bp"
    );
    println!("{}", "-".repeat(64));
    for t in &trends {
        println!(
            "{:<24}  {:>6}  {:>6}  {:>6}  {:>10}",
            t.topic.as_str(),
            t.count_24h,
            t.count_7d,
            t.count_30d,
            t.ratio_7d_vs_30d_bp,
        );
    }
    Ok(())
}

async fn cmd_opportunities(
    service: &GrowthService<AevumDbGrowthStorage>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Single analysis entrypoint. Loads trends + detects
    // opportunities with real evidence.
    let result = service
        .analyze_opportunities()
        .await
        .map_err(|e| format!("analyze_opportunities failed: {:?}", e))?;

    if result.opportunities.is_empty() {
        println!("no opportunities detected");
        return Ok(());
    }

    println!(
        "{:<24}  {:<22}  {:>8}  {:>8}  {}",
        "topic", "kind", "score_bp", "evidence", "detected_at"
    );
    println!("{}", "-".repeat(96));
    for o in &result.opportunities {
        println!(
            "{:<24}  {:<22}  {:>8}  {:>8}  {}",
            o.topic.as_str(),
            format!("{:?}", o.kind),
            o.score_bp,
            o.evidence.len(),
            o.detected_at.to_rfc3339(),
        );
    }
    println!("\ntotal: {}", result.opportunities.len());
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Load trend inputs from storage.
///
/// Phase 1 limitation: fetches up to `PHASE_1_MAX_PUBLICATIONS`
/// most recent publications. This is a correctness-neutral
/// approximation for small datasets, but it becomes incomplete at
/// scale.
///
/// The correct long-term approach is a time-bounded scan
/// `[now - 30d, now]`, which requires a new AevumDB primitive
/// (`range_scan` / `list_publications_since`). That primitive is
/// tracked as Tier-1 backlog; this loader will switch to it
/// without changing its signature.
async fn load_trend_inputs(
    service: &GrowthService<AevumDbGrowthStorage>,
) -> Result<Vec<TrendInput>, Box<dyn std::error::Error>> {
    let publications = service
        .storage()
        .list_recent_publications(PHASE_1_MAX_PUBLICATIONS)
        .await
        .map_err(|e| format!("list_recent_publications failed: {:?}", e))?;

    let inputs: Vec<TrendInput> = publications
        .into_iter()
        .map(|p| TrendInput {
            topics: p.topics.clone(),
            effective_ts: p.published_at.unwrap_or(p.ingested_at),
        })
        .collect();

    Ok(inputs)
}

fn resolve_db_path(cli: Option<PathBuf>) -> PathBuf {
    if let Some(p) = cli {
        return p;
    }
    if let Ok(env) = std::env::var("AEVUM_GROWTH_DB") {
        if !env.trim().is_empty() {
            return PathBuf::from(env);
        }
    }
    PathBuf::from("./data/growth")
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        return s.to_owned();
    }
    let mut out: String = s.chars().take(n.saturating_sub(1)).collect();
    out.push('…');
    out
}
