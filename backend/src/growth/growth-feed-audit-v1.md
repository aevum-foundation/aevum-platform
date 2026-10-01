# Growth Feed Ingestion — Audit v1

Status: **Draft — for review before ingestion code**
Owner: Growth domain
Companion to: `PHASE_1_PLAN.md`, `growth-storage-design-v1.md`
Scope: contract for RSS 2.0 / Atom parsing in `ingestion/`.

This document fixes the contract that `ingestion/rss.rs` MUST
implement. It is written **before** the parser code, based on real
feed samples (see section 10). The parser MUST conform to this
document.

---

## 1. Scope and Non-goals

In scope:

- RSS 2.0 and Atom feed parsing
- external identity rules (guid / id / link)
- date parsing rules
- output contract (`ParsedPublication`)
- error handling policy for malformed entries
- fixture-based parser tests

Out of scope:

- HTTP fetching
- scheduling / polling
- storage writes
- topic classification
- opportunity detection
- payload size limits (resource policy, separate)

---

## 2. Responsibilities (rss.rs boundary)

`ingestion/rss.rs` transforms XML bytes into normalized feed
entries. It MUST:

- parse RSS 2.0 and Atom XML;
- extract entries;
- apply identity rules (section 5);
- apply date rules (section 6);
- produce `ParsedPublication` values (section 7).

It MUST NOT:

- perform HTTP (owned by `ingestion/fetcher.rs`);
- write to AevumDB (owned by `growth/aevumdb/`);
- know about `AevumDbGrowthStorage` or `InMemoryGrowthStorage`;
- classify topics (owned by `analysis/classifier.rs`);
- detect opportunities (owned by `analysis/opportunities.rs`);
- implement retry or backoff (owned by fetcher);
- schedule polls (owned by service layer).

Rationale: the parser is a pure function of bytes. This makes it
testable on fixtures without a network, without a database, and
without a scheduler.

---

## 3. RSS 2.0 Contract

Root element: `<rss version="2.0">`.

Channel: `<channel>`.

Entries: repeated `<item>`.

### 3.1 Fields used by the parser

| Element | Status | Meaning | Notes |
| --- | --- | --- | --- |
| `title` | MUST (else skip) | entry title | plain text |
| `link` | SHOULD | entry URL | used as `external_id` fallback |
| `guid` | SHOULD | stable identity | `isPermaLink` attribute MAY be present |
| `description` | SHOULD | summary / content | MAY be CDATA; MAY be HTML-escaped |
| `pubDate` | SHOULD | publication date | RFC 2822 / RFC 822 |
| `category` | MAY (multiple) | topic hints | not used for classification in Phase 1 |
| `enclosure` | MAY | attachments | ignored in Phase 1 |
| `dc:creator` | MAY (multiple) | author(s) | first one is used |
| `dc:rights` | MAY | license / rights | ignored in Phase 1 |

### 3.2 Real sample (eprint.iacr.org)

```xml
<item>
  <title>LibFWHT: From Exact Walsh Spectra to Key Dependence ...</title>
  <link>https://eprint.iacr.org/2026/1997</link>
  <description><![CDATA[...long text...]]></description>
  <guid isPermaLink="true">https://eprint.iacr.org/2026/1997</guid>
  <category>Secret-key cryptography</category>
  <enclosure url="https://eprint.iacr.org/2026/1997.pdf" length="2448454" type="application/pdf"/>
  <pubDate>Sat, 12 Sep 2026 19:01:45 +0000</pubDate>
  <dc:creator>Hosein Hadipour</dc:creator>
  <dc:creator>Saleh Khalaj Monfared</dc:creator>
  <dc:rights>https://creativecommons.org/licenses/by/4.0/</dc:rights>
</item>
Observations:

guid is a permalink and equals link.

description uses CDATA, is very long (several KB).

Multiple dc:creator elements → multiple authors.

pubDate is RFC 2822 with +0000 timezone.

4. Atom Contract
Root element: <feed xmlns="http://www.w3.org/2005/Atom">.

Entries: repeated <entry>.

4.1 Fields used by the parser
Element	Status	Meaning	Notes
title	MUST (else skip)	entry title	plain text
link[rel=alternate]	SHOULD	entry URL	href attribute
id	SHOULD	stable identity	typically a permalink
published	SHOULD	publication date	RFC 3339
updated	MAY	last edit date	RFC 3339; fallback if published missing
content	SHOULD	content	type attribute (html / text / xhtml); MAY be escaped HTML
summary	MAY	short summary	fallback if content missing
category	MAY (multiple)	topic hints	not used for classification in Phase 1
author[name]	MAY	author	at entry level; feed-level author is ignored
4.2 Real sample (blog.rust-lang.org)
xml
<entry>
  <title>Announcing Rust 1.99.0</title>
  <link rel="alternate" href="https://blog.rust-lang.org/2026/10/01/Rust-1.99.0/" type="text/html"/>
  <published>2026-10-01T00:00:00+00:00</published>
  <updated>2026-10-01T00:00:00+00:00</updated>
  <id>https://blog.rust-lang.org/2026/10/01/Rust-1.99.0/</id>
  <content type="html" xml:base="https://blog.rust-lang.org/2026/10/01/Rust-1.99.0/">&lt;p&gt;The Rust team is happy ...</content>
</entry>
Observations:

id is a permalink and equals the link[rel=alternate] href.

published and updated are RFC 3339 with +00:00.

content is HTML, escaped as XML text (&lt; / &gt;).

No <category> in this feed.

No <author> at entry level (feed-level author only).

---

## 5. External Identity Rules

`external_id` is the stable identity of a publication within a
source. It feeds `PublicationId::from_parts(source_id, external_id)`.

Rules by feed type:

Atom:
id → link[rel=alternate].href → skip

text

RSS 2.0:
guid → link → skip

text

Additional rules:

- The parser MUST NOT generate a synthetic `external_id` when none
  is present. Doing so would create duplicates on re-ingestion.
- If both `id`/`guid` and `link` are absent, the entry MUST be
  skipped and the reason MUST be logged at `warn` level.
- If `id`/`guid` is present but not a valid URL, it is still used
  as `external_id` verbatim. Identity is a string; it does not
  need to be a URL.
- Trimming: leading/trailing whitespace MUST be stripped before
  use.

---

## 6. Date Parsing Rules

Dates are parsed per field, not per feed type.

Atom:
published → updated → None

text

Rationale: `published` is the intended publication date.
`updated` may change on every edit and is only a fallback.

RSS 2.0:
pubDate → None

text

Format:

| Feed type | Field | Format | Parser |
| --- | --- | --- | --- |
| Atom | `published` / `updated` | RFC 3339 | `DateTime::parse_from_rfc3339` |
| RSS 2.0 | `pubDate` | RFC 2822 / RFC 822 | `DateTime::parse_from_rfc2822` |

Additional rules:

- All parsed dates MUST be converted to UTC.
- If a date field is present but does not parse, the entry MUST
  NOT be skipped. Instead, `published_at = None` and the failure
  MUST be logged at `warn` level.
- If no date is present, `published_at = None`. The storage layer
  will use `effective_timestamp(published_at, ingested_at)`, which
  falls back to `ingested_at`.

---

## 7. ParsedPublication Output Contract

The parser produces values of:

```rust
pub struct ParsedPublication {
    pub external_id: String,
    pub url: Option<String>,
    pub title: String,
    pub summary: Option<String>,
    pub author: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
}
Field rules:

Field	Source	Notes
external_id	section 5	MUST be non-empty
url	link / link[rel=alternate].href	None if absent
title	title	MUST be non-empty (trimmed)
summary	description (RSS) / content or summary (Atom)	full text, not truncated
author	first dc:creator (RSS) / author[name] (Atom)	None if absent
published_at	section 6	None if missing or unparseable
The parser does NOT fill source_id, id, topics, or
ingested_at. Those are the responsibility of the ingestion
service (service.rs), which composes a full Publication.

Rationale: the parser stays a pure function of bytes. It does not
know the source, cannot compute PublicationId, and does not
perform classification.

---

## 8. Error Handling Policy

The parser distinguishes three levels of failure.

### 8.1 Feed-level failure (fatal)

Returns `Err(ApiError)`:

- XML is not well-formed
- root element is neither `<rss>` nor `<feed>`
- XML namespace for Atom is missing on `<feed>`

The caller (ingestion service) MUST treat this as a failed fetch
and MUST NOT persist anything for this feed.

### 8.2 Entry-level failure (skip + log)

Entry is skipped, parser continues with the next entry:

- missing `title`
- missing identity (section 5)
- `external_id` empty after trim

Each skipped entry MUST be logged at `warn` level with:

- feed URL (passed in by caller)
- entry index (0-based position in the feed)
- reason

### 8.3 Entry-level degradation (warn + continue)

Entry is kept with a degraded field:

- unparseable date → `published_at = None`
- missing `link` → `url = None`
- missing `author` → `author = None`
- missing `summary` → `summary = None`

Each degradation MUST be logged at `warn` level with the entry
index and the field name.

### 8.4 Invariants

- The parser MUST NOT panic on any input. All XML access uses
  fallible APIs from `quick-xml` and `Option`/`Result` handling.
- The parser MUST NOT perform network calls.
- The parser MUST NOT touch storage.
- The parser MUST be deterministic: same bytes → same output.

---

## 9. Fixture-Based Test Strategy

Parser tests MUST NOT depend on network access.

Required fixtures (stored under `tests/fixtures/feeds/`):

| Fixture | Type | Purpose |
| --- | --- | --- |
| `rust_blog_feed.xml` | Atom | nominal Atom sample |
| `eprint_feed.xml` | RSS 2.0 | nominal RSS sample with CDATA + dc:creator |
| `atom_minimal.xml` | Atom | entry missing `published`, using `updated` |
| `rss_minimal.xml` | RSS 2.0 | entry missing `pubDate` |
| `atom_no_id.xml` | Atom | entry missing `id`, using `link` fallback |
| `rss_no_guid.xml` | RSS 2.0 | entry missing `guid`, using `link` fallback |
| `rss_no_identity.xml` | RSS 2.0 | entry with neither `guid` nor `link` → skip |
| `atom_cdata_content.xml` | Atom | `content` as CDATA |
| `rss_multiple_authors.xml` | RSS 2.0 | multiple `dc:creator` → first used |
| `malformed.xml` | — | not well-formed → feed-level error |
| `wrong_root.xml` | — | `<html>` root → feed-level error |

Required test cases:

1. Atom nominal → correct `ParsedPublication` set.
2. RSS nominal → correct `ParsedPublication` set.
3. Atom fallback from `published` to `updated`.
4. RSS missing `pubDate` → `published_at = None`.
5. Atom missing `id` → fallback to `link`.
6. RSS missing `guid` → fallback to `link`.
7. Entry with no identity → skipped, one entry less in output.
8. Feed-level error on malformed XML.
9. Feed-level error on wrong root element.
10. Multiple `dc:creator` → first author used.
11. Determinism: parse the same fixture twice → identical output.

Fixtures MUST be committed to the repository and MUST be
byte-stable.

---

## 10. Sample Matrix

Real feeds used to derive sections 3 and 4:

| Feed | URL | Type |
| --- | --- | --- |
| Rust Blog | https://blog.rust-lang.org/feed.xml | Atom |
| IACR ePrint | https://eprint.iacr.org/rss/rss.xml | RSS 2.0 |

Planned sources from `PHASE_1_PLAN.md` MUST be verified against
this audit before their feeds are added to the registry. Any
field not covered by this document MUST be added to this audit
first, not guessed in code.

---

## Appendix A — Changelog

- v1 (draft) — initial feed ingestion contract. Derived from real
  samples of `blog.rust-lang.org/feed.xml` (Atom) and
  `eprint.iacr.org/rss/rss.xml` (RSS 2.0), plus the existing
  storage design document.
