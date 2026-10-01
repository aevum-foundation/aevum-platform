//! Integration tests for `growth::ingestion::rss::parse`.
//!
//! Fixtures live in `tests/fixtures/feeds/`.
//!
//! Real feeds (rust_blog_feed.xml, eprint_feed.xml) are checked
//! for structural properties, not exact content — their content
//! changes over time.
//!
//! Synthetic fixtures are checked for exact behavior.

use std::path::PathBuf;

use aevum_platform_api::growth::ingestion::rss::parse;

// ---------------------------------------------------------------------------
// Fixture loader
// ---------------------------------------------------------------------------

fn fixture_path(name: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests");
    p.push("fixtures");
    p.push("feeds");
    p.push(name);
    p
}

fn fixture(name: &str) -> String {
    std::fs::read_to_string(fixture_path(name))
        .unwrap_or_else(|e| panic!("fixture {}: {}", name, e))
}

// ---------------------------------------------------------------------------
// Real feeds — structural assertions only
// ---------------------------------------------------------------------------

#[test]
fn rust_blog_atom_parses() {
    let xml = fixture("rust_blog_feed.xml");
    let v = parse(&xml).expect("rust blog atom must parse");
    assert!(
        v.len() >= 5,
        "expected at least 5 entries, got {}",
        v.len()
    );
    for p in &v {
        assert!(!p.title.is_empty(), "title must not be empty");
        assert!(!p.external_id.is_empty(), "external_id must not be empty");
        assert!(
            p.url.is_some(),
            "url must be present (link[rel=alternate] or id)",
        );
    }
    assert!(
        v.iter().any(|p| p.published_at.is_some()),
        "at least one entry must have published_at",
    );
}

#[test]
fn eprint_rss_parses() {
    let xml = fixture("eprint_feed.xml");
    let v = parse(&xml).expect("eprint rss must parse");
    assert!(
        v.len() >= 20,
        "expected at least 20 items, got {}",
        v.len()
    );
    for p in &v {
        assert!(!p.title.is_empty());
        assert!(!p.external_id.is_empty());
        assert!(
            p.external_id.starts_with("https://eprint.iacr.org/"),
            "eprint external_id must be a permalink, got {}",
            p.external_id,
        );
    }
    // eprint uses dc:creator, so most entries should have an author.
    let with_author = v.iter().filter(|p| p.author.is_some()).count();
    assert!(with_author > 0, "expected at least one entry with author");
    // All items have pubDate.
    let with_date = v.iter().filter(|p| p.published_at.is_some()).count();
    assert_eq!(with_date, v.len(), "every eprint item should have a date");
}

// ---------------------------------------------------------------------------
// Synthetic fixtures — exact behavior
// ---------------------------------------------------------------------------

#[test]
fn atom_minimal_uses_updated_when_published_missing() {
    let v = parse(&fixture("atom_minimal.xml")).unwrap();
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].title, "Minimal Atom");
    assert_eq!(v[0].external_id, "https://example.com/1");
    assert!(v[0].published_at.is_some(), "must use <updated> as fallback");
}

#[test]
fn rss_minimal_has_no_date_and_uses_link_as_id() {
    let v = parse(&fixture("rss_minimal.xml")).unwrap();
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].title, "Minimal RSS");
    assert_eq!(v[0].external_id, "https://example.com/1");
    assert!(v[0].published_at.is_none());
}

#[test]
fn atom_no_id_falls_back_to_link() {
    let v = parse(&fixture("atom_no_id.xml")).unwrap();
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].external_id, "https://example.com/1");
}

#[test]
fn rss_no_guid_falls_back_to_link() {
    let v = parse(&fixture("rss_no_guid.xml")).unwrap();
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].external_id, "https://example.com/1");
}

#[test]
fn rss_no_identity_skips_broken_entry() {
    let v = parse(&fixture("rss_no_identity.xml")).unwrap();
    assert_eq!(v.len(), 1, "one entry must be skipped");
    assert_eq!(v[0].title, "OK");
}

#[test]
fn atom_link_without_rel_is_alternate() {
    let v = parse(&fixture("atom_link_no_rel.xml")).unwrap();
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].external_id, "https://example.com/1");
    assert_eq!(v[0].url.as_deref(), Some("https://example.com/1"));
}

#[test]
fn atom_cdata_content_is_captured() {
    let v = parse(&fixture("atom_cdata_content.xml")).unwrap();
    assert_eq!(v.len(), 1);
    let summary = v[0].summary.as_deref().unwrap_or("");
    assert!(
        summary.contains("Hello"),
        "CDATA content must be captured, got {:?}",
        summary,
    );
    assert!(
        summary.contains("CDATA"),
        "CDATA content must include the bold text",
    );
}

#[test]
fn rss_multiple_authors_uses_first() {
    let v = parse(&fixture("rss_multiple_authors.xml")).unwrap();
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].author.as_deref(), Some("First"));
}

#[test]
fn malformed_fixture_returns_error() {
    let xml = fixture("malformed.xml");
    assert!(parse(&xml).is_err(), "malformed XML must be rejected");
}

#[test]
fn wrong_root_fixture_returns_error() {
    let xml = fixture("wrong_root.xml");
    assert!(parse(&xml).is_err(), "HTML root must be rejected");
}
