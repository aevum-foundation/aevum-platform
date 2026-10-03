//! RSS 2.0 / Atom parser.
//!
//! Contract: `growth-feed-audit-v1.md`.
//!
//! Boundary:
//!
//! ```text
//! XML bytes  →  [rss.rs]  →  Vec<ParsedPublication>
//! ```
//!
//! This module does NOT:
//! - perform HTTP;
//! - write to storage;
//! - classify topics;
//! - detect opportunities;
//! - retry or back off;
//! - schedule polls.
//!
//! It is a pure function of bytes: same input → same output.
//!
//! # Error policy
//!
//! Feed-level structural errors (malformed XML, wrong root element,
//! undecodable text) return `Err(ApiError)`.
//!
//! Entry-level errors (missing title, missing identity, unparseable
//! date) do NOT fail the feed: the affected entry is skipped or
//! degraded, and the reason is logged at `warn` level.
//!
//! # Event::Empty vs Event::Start
//!
//! `quick-xml` distinguishes `<tag/>` (`Event::Empty`) from
//! `<tag>...</tag>` (`Event::Start` + ... + `Event::End`). The
//! parser handles both: an `Empty` event is treated as a start
//! immediately followed by the matching end.

use std::borrow::Cow;

use chrono::{DateTime, Utc};
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use crate::error::ApiError;

use super::{FeedType, ParsedPublication};

// ---------------------------------------------------------------------------
// Error constants
// ---------------------------------------------------------------------------

const FEED_MALFORMED_CODE: &str = "GROWTH_FEED_MALFORMED";
const FEED_MALFORMED_MSG: &str = "Feed XML is malformed";

const FEED_WRONG_ROOT_CODE: &str = "GROWTH_FEED_WRONG_ROOT";
const FEED_WRONG_ROOT_MSG: &str = "Feed root element is neither <rss> nor <feed>";

const FEED_ENCODING_CODE: &str = "GROWTH_FEED_ENCODING";
const FEED_ENCODING_MSG: &str = "Feed contains undecodable text";

const FEED_ATTRIBUTE_CODE: &str = "GROWTH_FEED_ATTRIBUTE";
const FEED_ATTRIBUTE_MSG: &str = "Feed contains a malformed attribute";

#[derive(Debug, Clone, Copy)]
enum FeedError {
    Malformed,
    WrongRoot,
    Encoding,
    Attribute,
}

impl FeedError {
    fn into_api_error(self) -> ApiError {
        match self {
            FeedError::Malformed => ApiError::ValidationFailed {
                code: FEED_MALFORMED_CODE,
                message: FEED_MALFORMED_MSG,
            },
            FeedError::WrongRoot => ApiError::ValidationFailed {
                code: FEED_WRONG_ROOT_CODE,
                message: FEED_WRONG_ROOT_MSG,
            },
            FeedError::Encoding => ApiError::ValidationFailed {
                code: FEED_ENCODING_CODE,
                message: FEED_ENCODING_MSG,
            },
            FeedError::Attribute => ApiError::ValidationFailed {
                code: FEED_ATTRIBUTE_CODE,
                message: FEED_ATTRIBUTE_MSG,
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Field tracking
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Title,
    Link,
    Guid,
    Id,
    Description,
    Summary,
    PubDate,
    Published,
    Updated,
    /// Atom `<author><name>`.
    AuthorName,
    /// RSS `<dc:creator>`.
    Creator,
    /// Atom `<content>` — accumulates all text nodes, including nested.
    Content,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

#[derive(Default)]
struct ParseState {
    feed_type: Option<FeedType>,
    root_seen: bool,

    /// Depth inside the current `<item>` / `<entry>`. Zero means we
    /// are inside the entry but not in any nested element.
    entry_depth: usize,
    /// True if we are inside an `<item>` / `<entry>`.
    in_entry: bool,
    /// True if we are inside an Atom `<author>`.
    in_author: bool,
    /// Depth inside an Atom `<content>` (1 = directly inside content).
    content_depth: usize,

    current_field: Option<Field>,
    /// Buffer for the current simple field. Not used for `<content>`
    /// — that field accumulates directly into the builder.
    buf: String,

    /// Depth of currently open elements. Incremented on every
    /// `Event::Start`, decremented on every `Event::End`.
    /// MUST be 0 at EOF for well-formed XML.
    open_depth: usize,

    parsed: Vec<ParsedPublication>,
}

#[derive(Default)]
struct EntryBuilder {
    title: String,
    link: String,
    link_alternate: String,
    guid: String,
    id: String,
    description: String,
    content: String,
    summary: String,
    pub_date: String,
    published: String,
    updated: String,
    author: String,
}

impl EntryBuilder {
    fn finish(self, feed_type: Option<FeedType>) -> Option<ParsedPublication> {
        let feed_type = feed_type?;

        let title = self.title.trim().to_owned();
        if title.is_empty() {
            log::warn!("Growth feed: entry skipped, missing or empty title");
            return None;
        }

        let (external_id, url) = match feed_type {
            FeedType::Atom => {
                let id = self.id.trim();
                if !id.is_empty() {
                    (
                        id.to_owned(),
                        first_non_empty(&self.link_alternate, &self.link),
                    )
                } else if !self.link_alternate.trim().is_empty() {
                    (
                        self.link_alternate.trim().to_owned(),
                        Some(self.link_alternate.clone()),
                    )
                } else if !self.link.trim().is_empty() {
                    (self.link.trim().to_owned(), Some(self.link.clone()))
                } else {
                    log::warn!("Growth feed: atom entry skipped, no id and no link");
                    return None;
                }
            }
            FeedType::Rss2 => {
                let guid = self.guid.trim();
                if !guid.is_empty() {
                    (guid.to_owned(), first_non_empty("", &self.link))
                } else if !self.link.trim().is_empty() {
                    (self.link.trim().to_owned(), Some(self.link.clone()))
                } else {
                    log::warn!("Growth feed: rss item skipped, no guid and no link");
                    return None;
                }
            }
        };

        let external_id = external_id.trim().to_owned();
        if external_id.is_empty() {
            log::warn!("Growth feed: entry skipped, empty external_id");
            return None;
        }

        // summary: Atom prefers content, falls back to summary.
        //          RSS uses description.
        let summary_text = match feed_type {
            FeedType::Atom => first_non_empty(&self.content, &self.summary),
            FeedType::Rss2 => first_non_empty(&self.description, ""),
        };

        let published_at = match feed_type {
            FeedType::Atom => {
                parse_atom_date(first_non_empty(&self.published, &self.updated).as_deref())
            }
            FeedType::Rss2 => parse_rss_date(&self.pub_date),
        };

        let author = {
            let a = self.author.trim();
            if a.is_empty() {
                None
            } else {
                Some(a.to_owned())
            }
        };

        Some(ParsedPublication {
            external_id,
            url: url.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty()),
            title,
            summary: summary_text
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty()),
            author,
            published_at,
        })
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub fn parse(xml: &str) -> Result<Vec<ParsedPublication>, ApiError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    reader.config_mut().check_end_names = true;

    let mut state = ParseState::default();
    let mut current: Option<EntryBuilder> = None;

    loop {
        let event = reader
            .read_event()
            .map_err(|e| log_and(FeedError::Malformed, format!("read_event: {}", e)))?;

        match event {
            Event::Eof => break,
            Event::Start(e) => {
                handle_start(&e, &mut state, &mut current, &mut None)?;
            }
            Event::Empty(e) => {
                handle_start(&e, &mut state, &mut current, &mut None)?;
                let name_owned: Vec<u8> = e.local_name().as_ref().to_vec();
                handle_end(&name_owned, &mut state, &mut current)?;
            }
            Event::End(e) => {
                let name_owned: Vec<u8> = e.local_name().as_ref().to_vec();
                handle_end(&name_owned, &mut state, &mut current)?;
            }
            Event::Text(t) => {
                let text = t
                    .xml_content()
                    .map_err(|e| log_and(FeedError::Encoding, format!("text: {}", e)))?;
                handle_text(&text, &mut state, &mut current);
            }
            Event::CData(c) => {
                let bytes = c.into_inner();
                let text = std::str::from_utf8(&bytes)
                    .map_err(|e| log_and(FeedError::Encoding, format!("cdata: {}", e)))?;
                handle_text(text, &mut state, &mut current);
            }
            _ => {}
        }
    }

    if !state.root_seen {
        return Err(log_and(
            FeedError::WrongRoot,
            "no <rss> / <feed> seen".into(),
        ));
    }

    // Refuse to return a partially-parsed feed: any element that
    // was opened but never closed indicates malformed XML.
    if state.open_depth != 0 {
        return Err(log_and(
            FeedError::Malformed,
            format!("{} unclosed element(s) at EOF", state.open_depth),
        ));
    }

    if state.in_entry {
        return Err(log_and(
            FeedError::Malformed,
            "unexpected EOF inside <item>/<entry>".into(),
        ));
    }

    // Flush any dangling entry (should not happen for well-formed feeds).
    if let Some(builder) = current.take() {
        if let Some(p) = builder.finish(state.feed_type) {
            state.parsed.push(p);
        }
    }

    Ok(state.parsed)
}

// ---------------------------------------------------------------------------
// Event handlers
// ---------------------------------------------------------------------------

fn handle_start(
    e: &BytesStart,
    state: &mut ParseState,
    current: &mut Option<EntryBuilder>,
    // Unused; kept for symmetry with handle_end signature evolution.
    _unused: &mut Option<()>,
) -> Result<(), ApiError> {
    let name = e.local_name();
    let name: &[u8] = name.as_ref();

    // Root element: only the first element we see is considered.
    if !state.root_seen {
        match name {
            b"rss" => {
                state.feed_type = Some(FeedType::Rss2);
                state.root_seen = true;
                state.open_depth = 1;
                return Ok(());
            }
            b"feed" => {
                state.feed_type = Some(FeedType::Atom);
                state.root_seen = true;
                state.open_depth = 1;
                return Ok(());
            }
            _ => {
                return Err(log_and(
                    FeedError::WrongRoot,
                    format!(
                        "first element is <{}>, expected <rss> or <feed>",
                        String::from_utf8_lossy(name)
                    ),
                ));
            }
        }
    }

    // Entry boundary.
    match name {
        b"item" | b"entry" => {
            state.in_entry = true;
            state.entry_depth = 0;
            state.current_field = None;
            state.buf.clear();
            state.in_author = false;
            state.content_depth = 0;
            state.open_depth += 1;
            *current = Some(EntryBuilder::default());
            return Ok(());
        }
        _ => {}
    }

    if !state.in_entry {
        return Ok(());
    }

    // Fields inside an entry.
    match name {
        b"title" => {
            state.entry_depth += 1;
            start_simple_field(state, Field::Title);
        }
        b"link" => {
            state.entry_depth += 1;
            // Atom: href is on the element; RSS: text content.
            if state.feed_type == Some(FeedType::Atom) {
                if let Some(builder) = current.as_mut() {
                    read_atom_link(e, builder)?;
                }
                // No text capture for Atom <link/>.
                state.current_field = None;
                state.buf.clear();
            } else {
                start_simple_field(state, Field::Link);
            }
        }
        b"guid" => {
            state.entry_depth += 1;
            start_simple_field(state, Field::Guid);
        }
        b"id" => {
            state.entry_depth += 1;
            start_simple_field(state, Field::Id);
        }
        b"description" => {
            state.entry_depth += 1;
            start_simple_field(state, Field::Description);
        }
        b"summary" => {
            state.entry_depth += 1;
            start_simple_field(state, Field::Summary);
        }
        b"pubDate" => {
            state.entry_depth += 1;
            start_simple_field(state, Field::PubDate);
        }
        b"published" => {
            state.entry_depth += 1;
            start_simple_field(state, Field::Published);
        }
        b"updated" => {
            state.entry_depth += 1;
            start_simple_field(state, Field::Updated);
        }
        b"creator" => {
            // dc:creator — local_name is "creator".
            state.entry_depth += 1;
            start_simple_field(state, Field::Creator);
        }
        b"author" => {
            state.entry_depth += 1;
            state.in_author = true;
        }
        b"name" => {
            state.entry_depth += 1;
            if state.in_author {
                start_simple_field(state, Field::AuthorName);
            }
        }
        b"content" => {
            state.entry_depth += 1;
            state.content_depth = 1;
            state.current_field = Some(Field::Content);
            // Do not clear content on entry — it accumulates across
            // nested text nodes.
        }
        _ => {
            state.entry_depth += 1;
            if state.content_depth > 0 {
                state.content_depth += 1;
            }
        }
    }

    state.open_depth += 1;

    Ok(())
}

fn handle_end(
    name: &[u8],
    state: &mut ParseState,
    current: &mut Option<EntryBuilder>,
) -> Result<(), ApiError> {
    state.open_depth = state.open_depth.saturating_sub(1);

    // Entry boundary close: only when depth reaches 0.
    if (name == b"item" || name == b"entry") && state.in_entry {
        if let Some(builder) = current.take() {
            if let Some(p) = builder.finish(state.feed_type) {
                state.parsed.push(p);
            }
        }
        state.in_entry = false;
        state.entry_depth = 0;
        state.current_field = None;
        state.in_author = false;
        state.content_depth = 0;
        state.buf.clear();
        return Ok(());
    }

    if !state.in_entry {
        return Ok(());
    }

    // Manage content_depth around nested elements.
    if name == b"content" && state.content_depth > 0 {
        state.content_depth = 0;
        state.current_field = None;
        state.entry_depth = state.entry_depth.saturating_sub(1);
        return Ok(());
    }

    if state.content_depth > 0 {
        // Nested element inside <content>: text accumulation continues.
        state.content_depth = state.content_depth.saturating_sub(1);
        state.entry_depth = state.entry_depth.saturating_sub(1);
        return Ok(());
    }

    // Simple field close.
    if let Some(field) = state.current_field.take() {
        let value = std::mem::take(&mut state.buf);
        let trimmed = value.trim();

        if let Some(builder) = current.as_mut() {
            match field {
                Field::Title => builder.title = trimmed.to_owned(),
                Field::Link => {
                    if builder.link.is_empty() {
                        builder.link = trimmed.to_owned();
                    }
                }
                Field::Guid => builder.guid = trimmed.to_owned(),
                Field::Id => builder.id = trimmed.to_owned(),
                Field::Description => builder.description = value,
                Field::Summary => builder.summary = value,
                Field::PubDate => builder.pub_date = trimmed.to_owned(),
                Field::Published => builder.published = trimmed.to_owned(),
                Field::Updated => builder.updated = trimmed.to_owned(),
                Field::Creator => {
                    if builder.author.is_empty() && !trimmed.is_empty() {
                        builder.author = trimmed.to_owned();
                    }
                }
                Field::AuthorName => {
                    if builder.author.is_empty() && !trimmed.is_empty() {
                        builder.author = trimmed.to_owned();
                    }
                }
                Field::Content => {
                    // Should not reach here: content closes above.
                }
            }
        }
    }

    if name == b"author" {
        state.in_author = false;
    }

    state.entry_depth = state.entry_depth.saturating_sub(1);

    Ok(())
}

fn handle_text(text: &str, state: &mut ParseState, current: &mut Option<EntryBuilder>) {
    if !state.in_entry {
        return;
    }

    // Accumulate into <content> if we are inside it.
    if state.content_depth > 0 {
        if let Some(builder) = current.as_mut() {
            builder.content.push_str(text);
        }
        return;
    }

    if state.current_field.is_some() {
        state.buf.push_str(text);
    }
}

// ---------------------------------------------------------------------------
// Field helpers
// ---------------------------------------------------------------------------

fn start_simple_field(state: &mut ParseState, field: Field) {
    state.current_field = Some(field);
    state.buf.clear();
}

fn read_atom_link(e: &BytesStart, builder: &mut EntryBuilder) -> Result<(), ApiError> {
    // Atom: rel defaults to "alternate".
    let mut rel: String = "alternate".to_owned();
    let mut href: Option<String> = None;

    for attr in e.attributes().flatten() {
        let key = attr.key.local_name();
        let key: &[u8] = key.as_ref();
        match key {
            b"rel" => {
                let v = attr
                    .unescape_value()
                    .map_err(|e| log_and(FeedError::Attribute, format!("rel: {}", e)))?;
                rel = v.into_owned();
            }
            b"href" => {
                let v = attr
                    .unescape_value()
                    .map_err(|e| log_and(FeedError::Attribute, format!("href: {}", e)))?;
                href = Some(v.into_owned());
            }
            _ => {}
        }
    }

    if rel == "alternate" {
        if let Some(href) = href {
            if builder.link_alternate.is_empty() {
                builder.link_alternate = href;
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Utility helpers
// ---------------------------------------------------------------------------

fn log_and(err: FeedError, detail: String) -> ApiError {
    log::warn!("Growth feed: {} ({})", err.into_api_error(), detail);
    err.into_api_error()
}

fn first_non_empty<'a>(a: &'a str, b: &'a str) -> Option<String> {
    let a = a.trim();
    if !a.is_empty() {
        return Some(a.to_owned());
    }
    let b = b.trim();
    if !b.is_empty() {
        return Some(b.to_owned());
    }
    None
}

fn parse_atom_date(s: Option<&str>) -> Option<DateTime<Utc>> {
    let s = s?.trim();
    if s.is_empty() {
        return None;
    }
    match DateTime::parse_from_rfc3339(s) {
        Ok(dt) => Some(dt.with_timezone(&Utc)),
        Err(e) => {
            log::warn!("Growth feed: atom date parse failed: {} ({})", s, e);
            None
        }
    }
}

fn parse_rss_date(s: &str) -> Option<DateTime<Utc>> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    match DateTime::parse_from_rfc2822(s) {
        Ok(dt) => Some(dt.with_timezone(&Utc)),
        Err(e) => {
            log::warn!("Growth feed: rss date parse failed: {} ({})", s, e);
            None
        }
    }
}

// Silence unused import warnings when features are disabled.
#[allow(dead_code)]
fn _unused_cow<'a>(c: Cow<'a, str>) -> Cow<'a, str> {
    c
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const ATOM_NOMINAL: &str = r#"<?xml version="1.0"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Test</title>
  <updated>2026-10-01T12:00:00+00:00</updated>
  <entry>
    <title>Hello</title>
    <link rel="alternate" href="https://example.com/1" type="text/html"/>
    <id>https://example.com/1</id>
    <published>2026-10-01T00:00:00+00:00</published>
    <updated>2026-10-01T00:00:00+00:00</updated>
    <content type="html">&lt;p&gt;Body&lt;/p&gt;</content>
  </entry>
</feed>"#;

    const RSS_NOMINAL: &str = r#"<?xml version="1.0"?>
<rss version="2.0">
  <channel>
    <title>Test</title>
    <item>
      <title>Post 1</title>
      <link>https://example.com/1</link>
      <guid isPermaLink="true">https://example.com/1</guid>
      <description>Body</description>
      <pubDate>Sat, 12 Sep 2026 19:01:45 +0000</pubDate>
      <dc:creator>Author A</dc:creator>
    </item>
  </channel>
</rss>"#;

    #[test]
    fn atom_nominal_parses() {
        let v = parse(ATOM_NOMINAL).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].external_id, "https://example.com/1");
        assert_eq!(v[0].title, "Hello");
        assert_eq!(v[0].url.as_deref(), Some("https://example.com/1"));
        assert!(v[0].published_at.is_some());
    }

    #[test]
    fn rss_nominal_parses() {
        let v = parse(RSS_NOMINAL).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].external_id, "https://example.com/1");
        assert_eq!(v[0].title, "Post 1");
        assert_eq!(v[0].author.as_deref(), Some("Author A"));
        assert!(v[0].published_at.is_some());
    }

    #[test]
    fn atom_link_without_rel_is_alternate() {
        let xml = r#"<?xml version="1.0"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <entry>
    <title>Hello</title>
    <link href="https://example.com/1"/>
  </entry>
</feed>"#;
        let v = parse(xml).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].external_id, "https://example.com/1");
    }

    #[test]
    fn atom_no_id_falls_back_to_link() {
        let xml = r#"<?xml version="1.0"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <entry>
    <title>Hello</title>
    <link rel="alternate" href="https://example.com/1"/>
  </entry>
</feed>"#;
        let v = parse(xml).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].external_id, "https://example.com/1");
    }

    #[test]
    fn rss_no_guid_falls_back_to_link() {
        let xml = r#"<?xml version="1.0"?>
<rss version="2.0"><channel>
  <item>
    <title>Post</title>
    <link>https://example.com/1</link>
  </item>
</channel></rss>"#;
        let v = parse(xml).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].external_id, "https://example.com/1");
    }

    #[test]
    fn entry_without_identity_is_skipped() {
        let xml = r#"<?xml version="1.0"?>
<rss version="2.0"><channel>
  <item><title>No id, no link</title></item>
  <item>
    <title>OK</title>
    <link>https://example.com/ok</link>
  </item>
</channel></rss>"#;
        let v = parse(xml).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].title, "OK");
    }

    #[test]
    fn entry_without_title_is_skipped() {
        let xml = r#"<?xml version="1.0"?>
<rss version="2.0"><channel>
  <item>
    <link>https://example.com/no-title</link>
  </item>
</channel></rss>"#;
        assert!(parse(xml).unwrap().is_empty());
    }

    #[test]
    fn entry_with_whitespace_title_is_skipped() {
        let xml = r#"<?xml version="1.0"?>
<rss version="2.0"><channel>
  <item>
    <title>   </title>
    <link>https://example.com/x</link>
  </item>
</channel></rss>"#;
        assert!(parse(xml).unwrap().is_empty());
    }

    #[test]
    fn atom_published_missing_falls_back_to_updated() {
        let xml = r#"<?xml version="1.0"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <entry>
    <title>T</title>
    <id>https://example.com/1</id>
    <updated>2026-10-01T00:00:00+00:00</updated>
  </entry>
</feed>"#;
        let v = parse(xml).unwrap();
        assert!(v[0].published_at.is_some());
    }

    #[test]
    fn rss_invalid_date_yields_none() {
        let xml = r#"<?xml version="1.0"?>
<rss version="2.0"><channel>
  <item>
    <title>T</title>
    <link>https://example.com/1</link>
    <pubDate>not-a-date</pubDate>
  </item>
</channel></rss>"#;
        let v = parse(xml).unwrap();
        assert_eq!(v.len(), 1);
        assert!(v[0].published_at.is_none());
    }

    #[test]
    fn malformed_xml_returns_error() {
        assert!(parse("<rss><unclosed>").is_err());
    }

    #[test]
    fn wrong_root_returns_error() {
        assert!(parse("<html></html>").is_err());
    }

    #[test]
    fn root_must_be_first_element() {
        let xml = "<foo><rss version=\"2.0\"><channel/></rss></foo>";
        assert!(parse(xml).is_err());
    }

    #[test]
    fn multiple_dc_creators_use_first() {
        let xml = r#"<?xml version="1.0"?>
<rss version="2.0"><channel>
  <item>
    <title>T</title>
    <link>https://example.com/1</link>
    <dc:creator>A</dc:creator>
    <dc:creator>B</dc:creator>
  </item>
</channel></rss>"#;
        let v = parse(xml).unwrap();
        assert_eq!(v[0].author.as_deref(), Some("A"));
    }

    #[test]
    fn atom_author_name_is_captured() {
        let xml = r#"<?xml version="1.0"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <entry>
    <title>T</title>
    <id>https://example.com/1</id>
    <author><name>John Doe</name></author>
  </entry>
</feed>"#;
        let v = parse(xml).unwrap();
        assert_eq!(v[0].author.as_deref(), Some("John Doe"));
    }

    #[test]
    fn determinism_same_input_same_output() {
        let a = parse(RSS_NOMINAL).unwrap();
        let b = parse(RSS_NOMINAL).unwrap();
        assert_eq!(a.len(), b.len());
        assert_eq!(a[0].external_id, b[0].external_id);
        assert_eq!(a[0].title, b[0].title);
    }
}
