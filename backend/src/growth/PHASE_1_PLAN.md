# Aevum Growth — Phase 1 Plan (final)

## Цель
Доказать, что цепочка работает на реальных данных:
Source → Publication → Topic → Trend → Opportunity

## Сущности (Phase 1)
- Source        (registry: RSS/Atom-фиды)
- Publication   (нормализованные записи из фидов)
- Topic         (6 предопределённых)
- Trend         (today/7d/30d counters)
- Opportunity   (сигнал обнаруженный на трендах)

## Вертикали (Phase 1)
- post_quantum
- distributed_systems
- rust
- blockchain_architecture
- gpu_compute
- storage_systems

## Что НЕ делает Phase 1
- Scheduler           (запуск вручную)
- GitHub              (Phase 1.5)
- Telegram            (Phase 2)
- Claim extraction    (Phase 2+)
- feed-rs / reqwest   (сначала разведка зависимостей)
- ML / LLM / embeddings
- Content generation
- User tracking

## Source Registry (фундамент)
Каждая тема = список проверенных фидов:

post_quantum:
  - https://www.nist.gov/news-events/news/rss.xml
  - https://blog.cloudflare.com/rss/
  - https://pqshield.com/feed/
  - https://openquantumsafe.org/feed.xml
  - https://eprint.iacr.org/rss

distributed_systems:
  - https://blog.tokio.rs/atom.xml
  - https://brooker.co.za/blog/rss.xml
  - https://muratbuffalo.blogspot.com/feeds/posts/default

rust:
  - https://blog.rust-lang.org/feed.xml
  - https://rust-lang.github.io/rfcs/feed.xml

blockchain_architecture:
  - https://blog.ethereum.org/feed.xml
  - https://vitalik.ca/feed.xml

gpu_compute:
  - https://developer.nvidia.com/blog/feed
  - https://huggingface.co/blog/feed.xml

storage_systems:
  - https://blog.min.io/rss.xml
  - https://ceph.io/en/news/blog/rss.xml

## AevumDB namespace
growth:source:{source_id}
growth:source:by_handle:{platform}:{handle}
growth:source:by_topic:{topic}:{source_id}

growth:publication:{publication_id}
growth:publication:by_source:{source_id}:{ts}:{publication_id}
growth:publication:by_topic:{topic}:{ts}:{publication_id}
growth:publication:by_time:{ts}:{publication_id}

growth:topic:{topic}
growth:topic:state:{topic}

growth:opportunity:{opportunity_id}
growth:opportunity:by_topic:{topic}:{ts}:{opportunity_id}
growth:opportunity:by_score:{score}:{opportunity_id}

growth:event:event:{event_id}
growth:event:timeline:{ts}:{event_id}
growth:event:by_kind:{kind}:{ts}:{event_id}

growth:metric:topic:{topic}:{date}

## API (Phase 1)
GET  /growth/topics
GET  /growth/topics/{topic}
GET  /growth/topics/{topic}/trend
GET  /growth/sources
GET  /growth/sources?topic=...
GET  /growth/publications?topic=...&since=...&limit=...
GET  /growth/opportunities?min_score=...
GET  /growth/metrics
GET  /growth/events?since=...&kind=...
POST /growth/ingest/rss         (ручной триггер)
POST /growth/registry/seed      (заполнить Source Registry)

## Trend Engine (без ML)
counter:today       = N публикаций за 24h
counter:7d          = N за 7 дней
counter:30d         = N за 30 дней
trend_7d_vs_30d     = (7d/7) / (30d/30)
trend > 2.0         → Opportunity: тема ускоряется

## Структура модуля
growth/
├── mod.rs
├── models.rs             # Source, Publication, Topic, Trend, Opportunity
├── contracts.rs          # DTO + коды ошибок
├── validation.rs
├── storage.rs            # trait GrowthStorage
├── aevumdb.rs            # impl над Arc<AevumDb>
├── service.rs
├── api.rs
├── registry/
│   ├── mod.rs
│   └── seed.rs           # стартовый список фидов
├── ingestion/
│   ├── mod.rs
│   ├── rss.rs            # собственный минимальный RSS/Atom-парсер
│   └── fetcher.rs        # HTTP через std::net или ureq (после разведки)
├── analysis/
│   ├── mod.rs
│   ├── classifier.rs     # keyword-based
│   ├── trends.rs         # counters
│   └── opportunities.rs  # детектор сигналов
└── events/
    ├── mod.rs
    ├── storage.rs
    └── aevumdb.rs

## Зависимости (TBD после разведки)
- HTTP: сначала посмотреть, есть ли что-то в проекте; иначе — минимальный клиент
- RSS: сначала попробовать свой парсер на stdlib (quick-xml? или ручной?)
- НЕ добавляем: feed-rs, reqwest (пока)

## Порядок реализации
1. models.rs + validation.rs
2. contracts.rs
3. storage.rs + aevumdb.rs
4. events/
5. registry/seed.rs
6. ingestion/rss.rs (свой парсер)
7. analysis/classifier.rs
8. analysis/trends.rs
9. analysis/opportunities.rs
10. service.rs + api.rs
11. Регистрация в main.rs + lib.rs
12. CLI-триггер ingest
13. Тесты + прогон на реальных фидах

## Критерий успеха Phase 1
Через 2 недели ручных прогонов:
- Source Registry заполнен (30+ фидов)
- Publications накоплены (1000+)
- Topics классифицированы
- Trends считаются
- Opportunities детектятся
- API отдаёт данные
- Мы видим РЕАЛЬНЫЕ сигналы роста в 6 вертикалях

---

## Key Encoding Rules (mandatory)

Per STORAGE STANDARD v1 sections 9 and 10.

Timestamps in ordered keys:

- Unit: microseconds (UTC).
- ASC ordering: `{ts_micros:020}` — 20-digit zero-padded.
- DESC ordering: `{inv_ts:020}`, where `inv_ts = u64::MAX - ts_micros`.
- Lexicographic ordering MUST match logical ordering.

Score in ordered keys:

- Encoded as fixed-width zero-padded decimal strings.
- Canonical width: 20 digits.
- Required for `growth:opportunity:by_score:{score:020}:{id}`.

All ordered indexes MUST include a stable tie-breaker (UUID)
appended after the ordering component.

Rationale: without fixed-width encoding, lexicographic order
diverges from numeric order (e.g. "9" > "10").

---

## Retention Policy

Declared classes per STORAGE STANDARD v1 section 15.
Phase 1 does not implement pruning; the class is declared so
that future pruning logic has an explicit contract.

- Source         → indefinite
- Publication    → indefinite
- Topic          → indefinite
- Opportunity    → indefinite
- Growth Event   → indefinite (Phase 1)
- Metrics        → indefinite (Phase 1)

---

## Deletion Semantics

Declared per STORAGE STANDARD v1 section 16.

- Source         → soft delete (future; keeps publications)
- Publication    → immutable (never deleted once stored)
- Topic          → immutable (fixed set in Phase 1)
- Opportunity    → hard delete allowed
- Growth Event   → append-only
- Metrics        → replace / update

---

## Conformance and Key Layout Contract

Per STORAGE STANDARD v1 section 20.

Growth MUST provide:

1. `InMemoryGrowthStorage` and `AevumDbGrowthStorage` implementing
   the same `GrowthStorage` trait.
2. A conformance test suite that runs identical scenarios against
   both implementations and asserts identical observable behavior.
3. A Persistent Key Layout Contract test that verifies:
   - exact bytes of every primary key builder output;
   - exact bytes of every secondary index key builder output;
   - exact bytes of every prefix builder output;
   - lexicographic ordering of composite keys;
   - prefix boundaries;
   - timestamp encoding (width, unit, ordering);
   - score encoding (width, ordering).

Any change to the key builders is a breaking change and MUST be
accompanied by a migration plan.
