//! Deterministic, explainable topic classifier.
//!
//! Contract: `growth-feed-audit-v1.md` (classification is out of
//! scope there, but the style follows the same rules).
//!
//! # Model (A+)
//!
//! ```text
//! final_topics = union(base_topics, detected_topics)
//! ```
//!
//! - `base_topics` come from the seed registry (`Source.topics`).
//!   They are the guaranteed context of the source: `ePrint` is
//!   always `PostQuantum`, `Rust Blog` is always `Rust`.
//!
//! - `detected_topics` come from keyword matching over
//!   `title + summary`. They refine the base set: a Rust Blog
//!   post about GPU acceleration becomes `[Rust, GpuCompute]`.
//!
//! Neither set is dropped. A publication never loses its source
//! context, and additional relevance is always recorded.
//!
//! # Invariants
//!
//! 1. Deterministic: `same input → same output`.
//! 2. Explainable: every detected topic carries the terms that
//!    matched, so the caller can render "why this topic?".
//! 3. Multi-class: a publication can carry any number of topics.
//! 4. Base context never lost.
//! 5. Pure function: no I/O, no storage, no time, no randomness.
//!
//! # Not in scope
//!
//! - ML / embeddings
//! - stemming, lemmatization
//! - language detection
//! - ranking of topics (all detected topics are equal)
//!
//! These can be layered later without changing the public
//! signature of `classify`.

use crate::growth::models::Topic;

// ---------------------------------------------------------------------------
// Output contract
// ---------------------------------------------------------------------------

/// Where a topic assignment came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceSource {
    /// Inherited from `Source.topics` (seed registry).
    Inherited,
    /// Detected by keyword matching over the publication text.
    Detected,
}

/// Why a specific topic ended up in the final set.
///
/// For `Inherited` topics `matched_terms` is empty — there is
/// nothing to explain, the source already declares this topic.
///
/// For `Detected` topics `matched_terms` contains the actual
/// lowercase terms that matched, in the order they appear in the
/// dictionary. Duplicates are removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopicEvidence {
    pub topic: Topic,
    pub source: EvidenceSource,
    pub matched_terms: Vec<String>,
}

/// Result of classification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassificationResult {
    /// Final set of topics: union of base and detected,
    /// de-duplicated, sorted by `Topic` discriminant order.
    pub topics: Vec<Topic>,
    /// One entry per topic in `topics`, same order.
    pub evidence: Vec<TopicEvidence>,
}

// ---------------------------------------------------------------------------
// Keyword dictionaries
// ---------------------------------------------------------------------------

/// Minimum number of matched terms required to accept a topic
/// from keyword matching.
const MIN_MATCHES: usize = 2;

/// Keyword dictionary for a single topic.
///
/// All terms are lowercase ASCII. Matching is substring-based on a
/// normalized `haystack` (see `classify`).
struct TopicDict {
    topic: Topic,
    terms: &'static [&'static str],
}

/// Canonical dictionary table.
///
/// Terms were derived from real titles across the 7 seed fixtures
/// (`eprint`, `rust_blog`, `muratbuffalo`, `pingcap`, `redpanda`,
/// `ethereum`, `nvidia`) and from the Phase 1 vertical definitions.
const DICTS: &[TopicDict] = &[
    TopicDict {
        topic: Topic::PostQuantum,
        terms: &[
            "post-quantum",
            "post quantum",
            "pqc",
            "ml-kem",
            "ml-dsa",
            "kyber",
            "dilithium",
            "lattice-based",
            "lattice cryptography",
            "quantum-resistant",
            "quantum safe",
            "quantum-safe",
            "fips 203",
            "fips 204",
            "fips 205",
        ],
    },
    TopicDict {
        topic: Topic::DistributedSystems,
        terms: &[
            "consensus",
            "raft",
            "paxos",
            "replication",
            "distributed",
            "distributed sql",
            "fault tolerance",
            "fault-tolerant",
            "byzantine",
            "eventual consistency",
            "linearizability",
            "linearizable",
            "metastability",
            "metastable",
            "quorum",
            "self-stabilization",
            "self-stabilizing",
            "stabilization",
            "actor model",
        ],
    },
    TopicDict {
        topic: Topic::Rust,
        terms: &[
            "rust",
            "cargo",
            "rustc",
            "crates.io",
            "tokio",
            "async rust",
            "rust-lang",
            "rustacean",
            "rustaceans",
            "memory safety",
            "borrow checker",
            "miri",
            "rustup",
            "trait solver",
            "rustconf",
            "rust blog",
        ],
    },
    TopicDict {
        topic: Topic::BlockchainArchitecture,
        terms: &[
            "ethereum",
            "blockchain",
            "smart contract",
            "smart contracts",
            "rollup",
            "rollups",
            "layer 2",
            "l2",
            "evm",
            "merkle",
            "state root",
            "consensus layer",
            "validator",
            "validators",
            "staking",
            "proof-of-stake",
            "proof of stake",
        ],
    },
    TopicDict {
        topic: Topic::GpuCompute,
        terms: &[
            "gpu",
            "gpus",
            "cuda",
            "nvidia",
            "h100",
            "a100",
            "h200",
            "tensor",
            "tensors",
            "matrix multiplication",
            "kernel fusion",
            "triton",
            "cudnn",
            "tensorrt",
            "tensor core",
            "tensor cores",
        ],
    },
    TopicDict {
        topic: Topic::StorageSystems,
        terms: &[
            "lsm",
            "lsm-tree",
            "sstable",
            "sstables",
            "compaction",
            "wal",
            "write-ahead log",
            "write ahead log",
            "memtable",
            "storage engine",
            "storage engines",
            "rocksdb",
            "b-tree",
            "b+tree",
            "tiered storage",
            "raft storage",
            "titan",
            "tikv",
            "tidb",
            "redpanda",
            "event stream",
            "stream processing",
            "streamhouse",
            "vector search",
            "full-text search",
        ],
    },
];

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Classify a publication.
///
/// # Arguments
///
/// - `title` — the publication title (required by the parser).
/// - `summary` — optional summary / content.
/// - `base_topics` — topics inherited from the source registry.
///
/// # Returns
///
/// A `ClassificationResult` whose `topics` field is the
/// deterministic union of `base_topics` and any topics detected
/// by keyword matching.
pub fn classify(
    title: &str,
    summary: Option<&str>,
    base_topics: &[Topic],
) -> ClassificationResult {
    let haystack = build_haystack(title, summary);

    // Detected topics in dictionary order (deterministic).
    let mut detected: Vec<(Topic, Vec<String>)> = Vec::new();
    for dict in DICTS {
        let matched = match_terms(&haystack, dict.terms);
        if matched.len() >= MIN_MATCHES {
            detected.push((dict.topic, matched));
        }
    }

    // Merge base and detected without duplicates.
    // We build a map Topic -> TopicEvidence using Topic::ALL order
    // for the final output. This guarantees determinism.
    let mut evidence: Vec<TopicEvidence> = Vec::new();

    for topic in Topic::ALL {
        let is_base = base_topics.contains(&topic);
        let detected_entry = detected.iter().find(|(t, _)| *t == topic);

        match (is_base, detected_entry) {
            (true, Some((_, terms))) => {
                // Present in both: keep base evidence and detected
                // terms so the caller can see both.
                evidence.push(TopicEvidence {
                    topic,
                    source: EvidenceSource::Detected,
                    matched_terms: terms.clone(),
                });
            }
            (true, None) => {
                evidence.push(TopicEvidence {
                    topic,
                    source: EvidenceSource::Inherited,
                    matched_terms: Vec::new(),
                });
            }
            (false, Some((_, terms))) => {
                evidence.push(TopicEvidence {
                    topic,
                    source: EvidenceSource::Detected,
                    matched_terms: terms.clone(),
                });
            }
            (false, None) => {}
        }
    }

    let topics: Vec<Topic> = evidence.iter().map(|e| e.topic).collect();

    ClassificationResult { topics, evidence }
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

/// Build the normalized haystack: lowercase, whitespace collapsed.
fn build_haystack(title: &str, summary: Option<&str>) -> String {
    let mut s = String::with_capacity(title.len() + 64);
    s.push_str(title);
    if let Some(sum) = summary {
        s.push(' ');
        s.push_str(sum);
    }
    // Lowercase ASCII; non-ASCII bytes are left as-is (we do not
    // attempt locale-aware lowercasing in Phase 1).
    let lowered: String = s.to_ascii_lowercase();
    // Collapse whitespace runs to a single space so that
    // "post\nquantum" and "post  quantum" match "post quantum".
    let mut collapsed = String::with_capacity(lowered.len());
    let mut prev_space = false;
    for ch in lowered.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                collapsed.push(' ');
                prev_space = true;
            }
        } else {
            collapsed.push(ch);
            prev_space = false;
        }
    }
    collapsed
}

/// Return the subset of `terms` that appear in `haystack`.
///
/// Preserves the order of `terms` (so the output is deterministic)
/// and removes duplicates.
fn match_terms(haystack: &str, terms: &[&str]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for term in terms {
        let t = term.to_ascii_lowercase();
        if haystack.contains(&t) && !out.contains(&t) {
            out.push(t);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_text_returns_only_base() {
        let r = classify("", None, &[Topic::Rust]);
        assert_eq!(r.topics, vec![Topic::Rust]);
        assert_eq!(r.evidence.len(), 1);
        assert_eq!(r.evidence[0].source, EvidenceSource::Inherited);
        assert!(r.evidence[0].matched_terms.is_empty());
    }

    #[test]
    fn no_base_and_no_matches_is_empty() {
        let r = classify("unrelated text", None, &[]);
        assert!(r.topics.is_empty());
        assert!(r.evidence.is_empty());
    }

    #[test]
    fn detected_only_topic() {
        // "consensus" + "raft" → DistributedSystems, no base.
        let r = classify("Raft consensus in practice", None, &[]);
        assert!(r.topics.contains(&Topic::DistributedSystems));
        assert_eq!(r.evidence.len(), 1);
        assert_eq!(r.evidence[0].source, EvidenceSource::Detected);
        assert!(r.evidence[0]
            .matched_terms
            .contains(&"consensus".to_owned()));
        assert!(r.evidence[0].matched_terms.contains(&"raft".to_owned()));
    }

    #[test]
    fn base_plus_detected_union() {
        // Rust Blog publishes a GPU post.
        let r = classify(
            "GPU acceleration in Rust",
            Some("How to use CUDA from Rust"),
            &[Topic::Rust],
        );
        assert!(r.topics.contains(&Topic::Rust));
        assert!(r.topics.contains(&Topic::GpuCompute));
    }

    #[test]
    fn base_survives_when_nothing_detected() {
        // ePrint cryptography paper: keywords do not fire, but base
        // is PostQuantum, so it is preserved.
        let r = classify(
            "Efficient commitments over binary fields",
            None,
            &[Topic::PostQuantum],
        );
        assert_eq!(r.topics, vec![Topic::PostQuantum]);
        assert_eq!(r.evidence[0].source, EvidenceSource::Inherited);
    }

    #[test]
    fn evidence_records_matched_terms() {
        let r = classify(
            "LSM compaction and the WAL",
            None,
            &[],
        );
        let entry = r
            .evidence
            .iter()
            .find(|e| e.topic == Topic::StorageSystems)
            .expect("StorageSystems must be detected");
        assert!(entry.matched_terms.contains(&"lsm".to_owned()));
        assert!(entry.matched_terms.contains(&"compaction".to_owned()));
        assert!(entry.matched_terms.contains(&"wal".to_owned()));
    }

    #[test]
    fn deterministic_same_input_same_output() {
        let a = classify("Raft consensus and LSM compaction", None, &[Topic::Rust]);
        let b = classify("Raft consensus and LSM compaction", None, &[Topic::Rust]);
        assert_eq!(a, b);
    }

    #[test]
    fn output_topics_are_in_enum_order() {
        let r = classify(
            "Rust on GPU with Raft consensus",
            None,
            &[],
        );
        // Must be sorted by Topic::ALL order, not by detection order.
        let expected: Vec<Topic> = Topic::ALL
            .iter()
            .copied()
            .filter(|t| r.topics.contains(t))
            .collect();
        assert_eq!(r.topics, expected);
    }

    #[test]
    fn whitespace_and_case_are_normalized() {
        let r = classify("RAFT\n\nConsensus", None, &[]);
        assert!(r.topics.contains(&Topic::DistributedSystems));
    }

    #[test]
    fn multi_class_detection() {
        // A PingCAP article can touch both storage and distributed.
        let r = classify(
            "Distributed SQL with LSM storage engine",
            None,
            &[Topic::StorageSystems],
        );
        assert!(r.topics.contains(&Topic::StorageSystems));
        assert!(r.topics.contains(&Topic::DistributedSystems));
    }

    #[test]
    fn threshold_prevents_single_keyword_noise() {
        // Only one match → no topic.
        let r = classify("A single mention of raft", None, &[]);
        assert!(!r.topics.contains(&Topic::DistributedSystems));
    }

    #[test]
    fn all_topics_have_at_least_one_dictionary() {
        let topics_in_dicts: Vec<Topic> = DICTS.iter().map(|d| d.topic).collect();
        for topic in Topic::ALL {
            assert!(
                topics_in_dicts.contains(&topic),
                "topic {:?} has no dictionary",
                topic,
            );
        }
    }
}
