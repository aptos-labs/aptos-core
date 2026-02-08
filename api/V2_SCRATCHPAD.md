# V2 API Scratchpad

Working notes, decisions log, and progress tracker for the API v2 implementation.

## Decisions Log

| Date | Decision | Rationale |
|---|---|---|
| 2026-02-07 | Framework: Axum + Tower + Hyper | Best Tonic/gRPC integration path, Tower middleware ecosystem, most popular Rust web framework |
| 2026-02-07 | gRPC: Deferred to Phase 2 | Focus on REST + WebSocket first; Tonic shares Tower ecosystem so integration will be smooth |
| 2026-02-07 | WebSocket: tx status, blocks, events | Replaces polling pattern, most requested real-time data |
| 2026-02-07 | Batching: JSON-RPC 2.0 | Widely adopted standard with excellent client library support |
| 2026-02-07 | BCS input: Versioned envelope | ULEB128 enum discriminant allows backward-compatible schema evolution |
| 2026-02-07 | BCS output: Deprecated | JSON-only output simplifies clients; BCS output was rarely used except for tx submission |
| 2026-02-07 | TX submission: BCS-only input | No JSON tx submission in v2; BCS is canonical and avoids ambiguity |
| 2026-02-07 | Error model: New V2Error with ErrorCode enum | Machine-readable, framework-agnostic, includes request_id |
| 2026-02-07 | OpenAPI: utoipa + utoipa-axum | Best Axum integration for OpenAPI spec generation |
| 2026-02-07 | HTTP/2: h2c default via hyper-util | Auto-detect HTTP/1.1 vs h2c; TLS optional via config |
| 2026-02-07 | Port: Configurable same or separate | Maximum deployment flexibility |
| 2026-02-07 | Context: V2Context wraps v1 Context | Shared DB/mempool/caches, decoupled from Poem error traits |
| 2026-02-07 | v1 deprecation: 3-6 month coexistence | Give ecosystem time to migrate |
| 2026-02-07 | Response metadata: body-only, no headers | v1's `X-Aptos-*` headers are awkward for many clients; putting ledger metadata in the JSON body alongside the data simplifies parsing and makes the API self-contained. Errors do NOT include ledger metadata (use `/v2/info` if needed). The only custom header is `X-Request-Id`. |
| 2026-02-07 | Pagination: unified opaque cursor on all list endpoints | v1 uses mixed styles (cursor for resources, offset for txns/events). v2 uses a single opaque cursor pattern everywhere. Server controls page size (no client `limit` param). Cursor is in the response body, not a header. Internal encoding is versioned (`version_byte + bcs(CursorInner)`) so format can evolve. |
| 2026-02-07 | HTTP/2 (h2c): already supported by axum::serve | `axum::serve` uses `hyper_util::server::conn::auto::Builder` internally, which auto-negotiates HTTP/1.1 and HTTP/2 (h2c prior knowledge). No additional configuration needed. |
| 2026-02-07 | Same-port co-hosting: Axum external + Poem internal proxy | Poem v1 starts on internal random port; Axum serves as the external-facing server with v2 routes and a reverse proxy fallback for v1. Both Poem 3.x and Axum 0.7 use hyper 1.x / http 1.x so types are compatible. Config: `api_v2.address = None` = same port, `api_v2.address = Some(addr)` = separate port. |

| 2026-02-08 | WebSocket: broadcast + per-connection filter | Background block poller writes to `broadcast::Sender<WsEvent>` (capacity 4096); each connection subscribes and filters events against its active subscriptions. tx_status uses dedicated per-subscription poller task. `tokio-tungstenite 0.21.0` (matches axum's internal tungstenite) for test client. |
| 2026-02-08 | TLS: rustls 0.21 + tokio-rustls 0.24 + hyper-util auto-builder | Manual accept loop with `TlsAcceptor` → `hyper_util::server::conn::auto::Builder` for ALPN-based h2/http1.1 negotiation. Uses `hyper1` (renamed hyper 1.x) for `service_fn`. Config: `api_v2.tls_cert_path` + `api_v2.tls_key_path`. PEM support for PKCS8, RSA, and EC keys. |
| 2026-02-08 | OpenAPI: utoipa 5.x + utoipa-axum 0.1.0 | `utoipa-axum 0.1.0` is compatible with axum 0.7 (0.2.0 requires axum 0.8). Manual `#[utoipa::path]` macros on handlers, `#[derive(ToSchema)]` on types, `Object` placeholder for external aptos_api_types. Spec served at `/v2/spec.json` and `/v2/spec.yaml`. |

|| 2026-02-08 | WebSocket event filtering: compiled EventFilter | Events subscriptions support: exact match, module wildcard (`0x1::coin::*`), address wildcard (`0x1::*`), multiple types (OR logic via `event_types[]`), sender address filtering, and start_version floor. Filters are compiled on subscribe for zero-alloc per-event matching. Backward compat: `event_type` + `event_types` are merged. All matching events are delivered (not just the first). Broadcaster now includes sender address from user txns and converts BCS event data to JSON via MoveConverter. |

|| 2026-02-08 | Batch: shared snapshot | All requests in a JSON-RPC batch share a pinned `LedgerInfo` + version (`BatchSnapshot`). Guarantees read consistency and eliminates N redundant `ledger_info()` calls per batch. Per-request `ledger_version` override still supported for explicit version pinning. |
|| 2026-02-08 | Ledger info: 50ms TTL cache | `V2Context::ledger_info()` caches the result for 50ms using `tokio::sync::RwLock` with `try_read`/`try_write` (non-blocking). Under high QPS, hundreds of requests share a single DB read. `ledger_info_uncached()` available for cases needing absolute freshness (e.g., WS broadcaster). Cache hit/miss tracked via Prometheus counters. |
|| 2026-02-08 | WS broadcaster: adaptive poll | Poll interval auto-adjusts: starts at 100ms, shrinks by 1.5x to floor 20ms when new blocks arrive, grows by 1.5x to ceiling 500ms when idle. Resets to max when no WS clients connected. Reduces CPU usage on slow/idle chains. |
|| 2026-02-08 | Prometheus metrics | New `v2/metrics.rs` with `aptos_api_v2_` prefix. Tracks: request duration histogram (method/path/status), request count, in-flight gauge, WS connections, WS messages, batch sizes, ledger cache hit/miss. Path normalization in middleware prevents cardinality explosion (dynamic segments → `:address`, `:hash`, etc.). |

## Open Questions

- [ ] Should v2 response envelope include gas_used for view functions?
- [ ] Should WebSocket events include full transaction data or just summaries?
- [ ] Should batch requests share a single DbStateView for consistency?
- [ ] What's the right broadcast channel capacity for WebSocket events?
- [ ] Should we support Server-Sent Events (SSE) as an alternative to WebSocket?
- [ ] Do we need API key / authentication support in v2?

## Phase Breakdown

### Phase 1: Foundation (current)
- Config, dependencies, module structure
- V2Context, V2Error, router integration
- Core endpoints: health, info, resources, view, transactions, blocks
- JSON-RPC batch
- WebSocket (new_blocks, tx_status, events)
- Tower middleware, HTTP/2, OpenAPI

### Phase 2: gRPC + Advanced Features
- Tonic gRPC service alongside REST
- Protobuf definitions for core types
- gRPC streaming (alternative to WebSocket for server-to-server)
- ~~Advanced event filtering in WebSocket~~ (done in Phase 1)

### Phase 3: Optimization (current)
- ~~Shared state views for batch requests~~ (done)
- ~~TTL-cached ledger info~~ (done)
- ~~Adaptive polling for WebSocket broadcaster~~ (done)
- ~~Prometheus metrics~~ (done)
- ~~Performance benchmarking vs v1~~ (done)

## Progress

- [x] Design documents written
- [x] Design docs updated: body-only metadata, cursor pagination, new endpoints (modules, events, account txns)
- [x] ApiV2Config struct added to NodeConfig
- [x] Axum/Tower/tower-http/utoipa/base64/uuid dependencies added
- [x] v2 module structure created (api/src/v2/)
- [x] V2Context implemented (wraps v1 Context, adds pagination helpers)
- [x] V2Error + ErrorCode implemented (40+ error codes, IntoResponse)
- [x] Opaque cursor-based pagination (StateKey, Version, SequenceNumber variants)
- [x] Health/info endpoints (GET /v2/health, GET /v2/info)
- [x] Resource endpoints (paginated list + single get)
- [x] Module endpoints (paginated list + single get)
- [x] View function endpoint (POST /v2/view, JSON input)
- [x] Transaction endpoints (paginated list, get by hash, BCS submit, wait by hash)
- [x] Account transaction summaries endpoint (paginated)
- [x] Events endpoint (paginated by creation number)
- [x] Block endpoints (by height, latest)
- [x] JSON-RPC 2.0 batch endpoint (POST /v2/batch, 8 methods supported)
- [x] Middleware (request-id, logging, CORS, compression, size limit)
- [x] Router integration (separate port via axum::serve in runtime.rs)
- [x] JsonOrBcs + BcsOnly content negotiation extractors
- [x] V2Response envelope with LedgerMetadata in body
- [x] Cursor unit tests (4 passing)
- [x] Integration tests (55 tests: 24 endpoint + 6 co-hosting + 17 WebSocket + 4 TLS + 4 OpenAPI)
- [x] Performance benchmarks (9 criterion benchmarks incl. parameterized batch)
- [x] HTTP/2 (h2c) — already supported by axum::serve (uses hyper_util auto::Builder)
- [x] Same-port Poem+Axum co-hosting via reverse proxy (api_v2.address=None → same port)
- [x] V1Proxy module for reverse-proxying v1 requests to internal Poem server
- [x] WebSocket support (`/v2/ws` with subscribe/unsubscribe/ping protocol)
  - [x] `websocket/types.rs`: WsClientMessage, WsServerMessage, SubscriptionType, WsEvent
  - [x] `websocket/broadcaster.rs`: background block poller with broadcast channel (capacity 4096)
  - [x] `websocket/mod.rs`: ws_handler, per-connection read/write/broadcast loops, match_event, tx_status_tracker
  - [x] V2Context extended with broadcast::Sender<WsEvent> and AtomicUsize active connection counter
  - [x] Route `/v2/ws` added to router
  - [x] Block poller started in runtime.rs when websocket_enabled=true
  - [x] Subscription types: new_blocks, transaction_status (hash-specific poller), events (advanced filtering)
  - [x] Guards: max_connections, max_subscriptions_per_conn, configurable via ApiV2Config
  - [x] 9 integration tests (ping/pong, subscribe, unsubscribe, error cases, tx status lifecycle)
- [x] Advanced WebSocket event filtering
  - [x] `EventFilter` compiled filter struct with zero-alloc per-event matching
  - [x] Exact match: `"0x1::coin::DepositEvent"`
  - [x] Module wildcard: `"0x1::coin::*"` (matches all events from module)
  - [x] Address wildcard: `"0x1::*"` (matches all events from address)
  - [x] Multiple type filters: `event_types: [...]` with OR logic
  - [x] Backward compat: `event_type` and `event_types` merged
  - [x] Sender address filtering: `sender` field (hex, case-insensitive)
  - [x] Version floor filtering: `start_version` field
  - [x] All matching events delivered (not just first match)
  - [x] Broadcaster includes sender address from user transactions
  - [x] Broadcaster converts BCS event data to JSON via MoveConverter
  - [x] 11 unit tests for EventFilter (exact, wildcard, sender, version, combined)
  - [x] 8 integration tests (multiple types, wildcards, sender, version, combined, merged, no-filter)
- [x] TLS support for v2 server (rustls + ALPN h2/http1.1 negotiation)
  - [x] `tls.rs`: `build_tls_acceptor` (PEM cert/key loader, PKCS8+RSA+EC), `serve_tls` (manual accept loop with hyper_util auto-builder)
  - [x] `ApiV2Config` extended with `tls_cert_path` and `tls_key_path`
  - [x] `runtime.rs` updated to use TLS when configured (both separate-port and co-hosting modes)
  - [x] Dependencies: `rustls 0.21`, `tokio-rustls 0.24`, `rustls-pemfile 1.0`, `hyper1` (renamed hyper 1.x)
  - [x] 4 integration tests (health, info, resources over TLS; invalid cert error)
- [x] OpenAPI spec generation (utoipa 5.x + utoipa-axum 0.1.0)
  - [x] `openapi.rs`: `V2ApiDoc` struct with `#[derive(OpenApi)]`, spec JSON/YAML handlers
  - [x] `#[utoipa::path]` macros on all 15 endpoint handlers
  - [x] `#[derive(ToSchema)]` on V2Error, ErrorCode, LedgerMetadata, HealthResponse, NodeInfo, SubmitResult, TransactionSummary
  - [x] `#[derive(IntoParams)]` on PaginatedLedgerParams, CursorOnlyParams, LedgerVersionParam, BlockParams
  - [x] Routes: `/v2/spec.json` and `/v2/spec.yaml`
  - [x] Tags: Health, Accounts, Transactions, Events, View, Blocks
  - [x] 4 integration tests (JSON spec, YAML spec, schemas, tags)
- [x] Phase 3: Optimization
  - [x] Batch shared snapshot: all requests in a batch share pinned `LedgerInfo` + version
  - [x] Ledger info 50ms TTL cache: `tokio::sync::RwLock` with `try_read`/`try_write`
  - [x] `ledger_info_uncached()` for absolute freshness (WS broadcaster, tx wait loops)
  - [x] Adaptive WS broadcaster polling: 20ms–500ms range, 1.5x factor, idle backoff
  - [x] Prometheus metrics module (`v2/metrics.rs`): 7 metric families with `aptos_api_v2_` prefix
  - [x] Path normalization in middleware: dynamic segments → `:address`, `:hash`, etc.
  - [x] Metrics instrumentation in middleware, context (cache), and batch handler
  - [x] Head-to-head v1 vs v2 benchmarks: health, ledger_info, resources, single_resource, transactions
  - [x] 8 unit tests for path normalization
  - [x] Total: 74 tests passing (55 integration + 11 EventFilter + 8 path normalization)
