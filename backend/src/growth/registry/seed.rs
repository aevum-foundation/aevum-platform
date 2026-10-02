//! Growth source registry — initial seed.
//!
//! Fixed list of validated feeds from `PHASE_1_PLAN.md`.
//!
//! All URLs in `SEED_FEEDS` were verified against their live
//! endpoints before being added here. Feeds that returned
//! non-200, timeout, or 404 are not included.
//!
//! `seed_sources` is idempotent: `SourceId` is deterministic over
//! `(platform, feed_url)`, so repeated seeding produces the same
//! primary keys and overwrites the same records.

use chrono::Utc;
use url::Url;

use crate::error::ApiError;
use crate::growth::models::{Platform, Source, Topic};
use crate::growth::storage::SourceStorage;

// ---------------------------------------------------------------------------
// Seed data
// ---------------------------------------------------------------------------

/// One entry in the seed list.
pub struct SeedFeed {
    pub platform: Platform,
    pub feed_url: &'static str,
    pub homepage: Option<&'static str>,
    pub topics: &'static [Topic],
}

/// Initial seed of validated feeds.
///
/// Verified live on 2026-10-02. Feeds that returned non-200,
/// timeout, or 404 were excluded.
pub const SEED_FEEDS: &[SeedFeed] = &[
    // ─── post_quantum ─────────────────────────────────
    SeedFeed {
        platform: Platform::Rss,
        feed_url: "https://www.nist.gov/news-events/news/rss.xml",
        homepage: Some("https://www.nist.gov/news-events"),
        topics: &[Topic::PostQuantum],
    },
    SeedFeed {
        platform: Platform::Rss,
        feed_url: "https://blog.cloudflare.com/rss/",
        homepage: Some("https://blog.cloudflare.com"),
        topics: &[Topic::PostQuantum],
    },
    SeedFeed {
        platform: Platform::Rss,
        feed_url: "https://pqshield.com/feed/",
        homepage: Some("https://pqshield.com"),
        topics: &[Topic::PostQuantum],
    },
    SeedFeed {
        platform: Platform::Atom,
        feed_url: "https://openquantumsafe.org/feed.xml",
        homepage: Some("https://openquantumsafe.org"),
        topics: &[Topic::PostQuantum],
    },
    SeedFeed {
        platform: Platform::Rss,
        feed_url: "https://eprint.iacr.org/rss/rss.xml",
        homepage: Some("https://eprint.iacr.org"),
        topics: &[Topic::PostQuantum],
    },

    // ─── distributed_systems ──────────────────────────
    SeedFeed {
        platform: Platform::Rss,
        feed_url: "https://brooker.co.za/blog/rss.xml",
        homepage: Some("https://brooker.co.za/blog/"),
        topics: &[Topic::DistributedSystems],
    },
    SeedFeed {
        platform: Platform::Atom,
        feed_url: "https://muratbuffalo.blogspot.com/feeds/posts/default",
        homepage: Some("https://muratbuffalo.blogspot.com/"),
        topics: &[Topic::DistributedSystems],
    },

    // ─── rust ─────────────────────────────────────────
    SeedFeed {
        platform: Platform::Atom,
        feed_url: "https://blog.rust-lang.org/feed.xml",
        homepage: Some("https://blog.rust-lang.org/"),
        topics: &[Topic::Rust],
    },

    // ─── blockchain_architecture ──────────────────────
    SeedFeed {
        platform: Platform::Rss,
        feed_url: "https://blog.ethereum.org/en/feed.xml",
        homepage: Some("https://blog.ethereum.org/"),
        topics: &[Topic::BlockchainArchitecture],
    },

    // ─── gpu_compute ──────────────────────────────────
    SeedFeed {
        platform: Platform::Atom,
        feed_url: "https://developer.nvidia.com/blog/feed",
        homepage: Some("https://developer.nvidia.com/blog"),
        topics: &[Topic::GpuCompute],
    },
    SeedFeed {
        platform: Platform::Rss,
        feed_url: "https://huggingface.co/blog/feed.xml",
        homepage: Some("https://huggingface.co/blog"),
        topics: &[Topic::GpuCompute],
    },

    // ─── storage_systems ──────────────────────────────
    SeedFeed {
        platform: Platform::Rss,
        feed_url: "https://www.pingcap.com/blog/rss/",
        homepage: Some("https://www.pingcap.com/blog/"),
        topics: &[Topic::StorageSystems, Topic::DistributedSystems],
    },
    SeedFeed {
        platform: Platform::Rss,
        feed_url: "https://redpanda.com/blog/rss.xml",
        homepage: Some("https://redpanda.com/blog"),
        topics: &[Topic::StorageSystems, Topic::DistributedSystems],
    },
];

// ---------------------------------------------------------------------------
// Seeding
// ---------------------------------------------------------------------------

/// Populate the source registry from `SEED_FEEDS`.
///
/// Idempotent: deterministic `SourceId` means repeated calls
/// overwrite the same records rather than creating duplicates.
///
/// Returns the number of sources written (equal to the number of
/// valid entries in `SEED_FEEDS`).
pub async fn seed_sources<S>(storage: &S) -> Result<usize, ApiError>
where
    S: SourceStorage + Sync,
{
    let now = Utc::now();
    let mut written = 0usize;

    for feed in SEED_FEEDS {
        let Ok(url) = Url::parse(feed.feed_url) else {
            log::warn!(
                "Growth seed: skipping feed with unparseable URL: {}",
                feed.feed_url
            );
            continue;
        };

        let homepage = match feed.homepage {
            Some(h) => match Url::parse(h) {
                Ok(u) => Some(u),
                Err(e) => {
                    log::warn!(
                        "Growth seed: bad homepage {} for {}: {}",
                        h,
                        feed.feed_url,
                        e
                    );
                    None
                }
            },
            None => None,
        };

        let topics = feed.topics.to_vec();
        if topics.is_empty() {
            log::warn!(
                "Growth seed: skipping feed with no topics: {}",
                feed.feed_url
            );
            continue;
        }

        let source = Source::new(feed.platform, url, homepage, topics, now);

        storage.put_source(&source).await?;
        written += 1;
    }

    Ok(written)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    use crate::growth::in_memory::InMemoryGrowthStorage;
    use crate::growth::storage::SourceStorage;

    #[test]
    fn seed_is_not_empty() {
        assert!(!SEED_FEEDS.is_empty());
        assert!(SEED_FEEDS.len() >= 10, "expected at least 10 seed feeds");
    }

    #[test]
    fn all_seed_urls_parse() {
        for feed in SEED_FEEDS {
            assert!(
                Url::parse(feed.feed_url).is_ok(),
                "bad feed_url: {}",
                feed.feed_url,
            );
            if let Some(h) = feed.homepage {
                assert!(
                    Url::parse(h).is_ok(),
                    "bad homepage for {}: {}",
                    feed.feed_url,
                    h,
                );
            }
        }
    }

    #[test]
    fn all_seed_entries_have_topics() {
        for feed in SEED_FEEDS {
            assert!(
                !feed.topics.is_empty(),
                "feed without topics: {}",
                feed.feed_url,
            );
        }
    }

    #[test]
    fn every_topic_is_covered() {
        // Every Phase 1 vertical should have at least one seed feed.
        for topic in Topic::ALL {
            let covered = SEED_FEEDS.iter().any(|f| f.topics.contains(&topic));
            assert!(covered, "topic {:?} has no seed feed", topic);
        }
    }

    #[tokio::test]
    async fn seed_writes_all_feeds() {
        let storage = InMemoryGrowthStorage::new();
        let n = seed_sources(&storage).await.unwrap();
        assert_eq!(n, SEED_FEEDS.len());

        let all = storage.list_sources(100).await.unwrap();
        assert_eq!(all.len(), SEED_FEEDS.len());
    }

    #[tokio::test]
    async fn seed_is_idempotent() {
        let storage = InMemoryGrowthStorage::new();
        let first = seed_sources(&storage).await.unwrap();
        let second = seed_sources(&storage).await.unwrap();
        assert_eq!(first, second);

        let all = storage.list_sources(100).await.unwrap();
        assert_eq!(
            all.len(),
            SEED_FEEDS.len(),
            "second seed must not create duplicates",
        );
    }

    #[tokio::test]
    async fn seed_populates_every_topic() {
        let storage = InMemoryGrowthStorage::new();
        seed_sources(&storage).await.unwrap();

        for topic in Topic::ALL {
            let by_topic = storage.list_sources_by_topic(topic, 100).await.unwrap();
            assert!(
                !by_topic.is_empty(),
                "topic {:?} has no seeded source",
                topic,
            );
        }
    }
}
