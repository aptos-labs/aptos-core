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
|| 2026-02-08 | Additional endpoints: 6 high-priority | Added all high-priority v1 endpoints missing from v2: `GET /v2/accounts/:address` (account info), `POST /v2/transactions/simulate` (BCS-only with gas estimation query params), `GET /v2/estimate_gas_price`, `POST /v2/tables/:handle/item`, `GET /v2/transactions/by_version/:version`, `GET /v2/blocks/by_version/:version`. All added to router, OpenAPI spec, and batch handler (5 new batch methods: get_account, get_transaction_by_version, get_block_by_version, estimate_gas_price, get_table_item). Path normalization updated for new routes. |
|| 2026-02-08 | Request timeout middleware | Configurable per-request timeout (`request_timeout_ms`, default 30s, 0=disabled). Implemented as Axum middleware between logging and CORS layers. Returns 408 + `REQUEST_TIMEOUT` error code. Prometheus counter (`aptos_api_v2_request_timeouts_total`) by method and path. |
|| 2026-02-08 | Graceful shutdown + connection draining | Configurable drain timeout (`graceful_shutdown_timeout_ms`, default 30s, 0=immediate). Uses `tokio::sync::watch` channel in V2Context for shutdown signaling. OS signal listener (Ctrl+C/SIGINT) triggers shutdown. `axum::serve().with_graceful_shutdown()` for plain mode; atomic connection counter with drain loop for TLS mode. WebSocket broadcaster exits cleanly on shutdown signal. `V2Context.trigger_shutdown()` for programmatic use (testing). |
|| 2026-02-08 | SSE (Server-Sent Events) | Two streaming endpoints: `GET /v2/sse/blocks` (new block notifications with `after_height` resumption) and `GET /v2/sse/events` (filtered events with comma-separated type patterns, sender, and start_version). Reuses WebSocket broadcast channel — block poller now starts when either WS or SSE is enabled. Background task per SSE connection reads from broadcast, filters, and forwards via mpsc channel. Avoids `tokio::select!` with broadcast (non-Send) by using `borrow()` shutdown check. `text/event-stream` content type, 15s keep-alive, `id` field for Last-Event-ID tracking. Config: `sse_enabled` (default true). |

## Resolved Questions

- [x] **Should v2 response envelope include gas_used for view functions?** — No. Gas used is tracked internally via `view_function_stats` for metrics but is not exposed in the JSON response. Keeping the response envelope uniform across all endpoints simplifies client code. For gas estimation use `GET /v2/estimate_gas_price` or `POST /v2/transactions/simulate`.
- [x] **Should WebSocket events include full transaction data or just summaries?** — Summaries only. `BlockSummary` for new blocks, `EventData` for events. Full transaction data would be too heavy for real-time streaming. Clients fetch full details via the REST endpoints when needed.
- [x] **Should batch requests share a single DbStateView for consistency?** — Yes. Implemented as `BatchSnapshot` in Phase 3: all requests in a batch share a pinned `LedgerInfo` + version for read consistency. Per-request `ledger_version` override still supported.
- [x] **What's the right broadcast channel capacity for WebSocket events?** — 4096. Provides sufficient buffering for typical block sizes while keeping memory bounded. Slow consumers receive a `LAGGED` error.
- [x] **Should we support Server-Sent Events (SSE) as an alternative to WebSocket?** — Yes. Implemented with `GET /v2/sse/blocks` (block notifications) and `GET /v2/sse/events` (filtered events). SSE is simpler for unidirectional streaming; WebSocket is better for bidirectional communication (subscribe/unsubscribe).
- [x] **Do we need API key / authentication support in v2?** — Deferred. Not in scope for initial v2 release. Operators should use reverse proxies (nginx, envoy, cloud load balancers) for authentication and rate limiting. May be revisited in a future phase.

## Phase Breakdown

### Phase 1: Foundation (current)
- Config, dependencies, module structure
- V2Context, V2Error, router integration
- Core endpoints: health, info, resources, view, transactions, blocks
- JSON-RPC batch
- WebSocket (new_blocks, tx_status, events)
- Tower middleware, HTTP/2, OpenAPI

### Phase 2: gRPC (Deferred)
- ~~Tonic gRPC service alongside REST~~ — Skipped per user decision
- ~~Protobuf definitions for core types~~ — Skipped per user decision
- ~~gRPC streaming~~ — Skipped per user decision
- ~~Advanced event filtering in WebSocket~~ (moved to Phase 1, done)

### Phase 3: Optimization (done)
- ~~Shared state views for batch requests~~ (done — `BatchSnapshot`)
- ~~TTL-cached ledger info~~ (done — 50ms TTL)
- ~~Adaptive polling for WebSocket broadcaster~~ (done — 20ms–500ms)
- ~~Prometheus metrics~~ (done — 8 metric families)
- ~~Performance benchmarking vs v1~~ (done — criterion benchmarks)

### Phase 4: Hardening & Polish (done)
- ~~Additional endpoints~~ (done — 6 high-priority + balance)
- ~~Request timeout middleware~~ (done — configurable per-request timeout)
- ~~Graceful shutdown / connection draining~~ (done — watch channel + drain timeout)
- ~~Server-Sent Events~~ (done — blocks + filtered events)
- ~~E2E integration tests~~ (done — 5 tests covering submit → commit → verify flow)
- ~~Documentation cleanup~~ (done — open questions resolved, design docs finalized)

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
- [x] Additional high-priority endpoints (6 endpoints)
  - [x] `GET /v2/accounts/:address` — account info (sequence number + auth key), stateless account fallback
  - [x] `POST /v2/transactions/simulate` — BCS-only simulation with gas estimation query params
  - [x] `GET /v2/estimate_gas_price` — gas price estimation (deprioritized, regular, prioritized)
  - [x] `POST /v2/tables/:table_handle/item` — typed table item lookup (key_type, value_type, key)
  - [x] `GET /v2/transactions/by_version/:version` — get transaction by version number
  - [x] `GET /v2/blocks/by_version/:version` — get block containing a specific version
  - [x] Router, OpenAPI, and batch handler updated with all new endpoints
  - [x] 5 new batch methods: get_account, get_transaction_by_version, get_block_by_version, estimate_gas_price, get_table_item
  - [x] Path normalization updated for new routes (tables, by_version)
  - [x] 10 new integration tests (account, account-not-found, gas estimation, txn by version, txn by version not found, block by version, block by version not found, batch account, batch gas, batch txn by version)
- [x] Balance endpoint
  - [x] `GET /v2/accounts/:address/balance/:asset_type` — supports coins and fungible assets
  - [x] Sums legacy coin balance + paired fungible asset balance (including concurrent FA)
  - [x] Router, OpenAPI, and path normalization updated
  - [x] 3 new integration tests (APT balance, invalid asset, nonexistent coin)
- [x] Clippy & formatting lint pass
  - [x] `cargo +nightly fmt` — all v2 files formatted
  - [x] `cargo clippy` — fixed `needless_question_mark` in view.rs, `if_same_then_else` in middleware.rs
  - [x] Added `#[allow(clippy::result_large_err)]` for V2Error (intentionally rich error struct)
  - [x] Zero warnings from v2 code
  - [x] Total: 68 v2 tests passing (all green)
- [x] Request timeout middleware
  - [x] `request_timeout_ms` config field (default 30s, 0=disabled) in `ApiV2Config` and `V2Config`
  - [x] `RequestTimeout` error code → HTTP 408 with `V2Error::request_timeout()` constructor
  - [x] `timeout_layer()` middleware using `tokio::time::timeout`, applied between logging and CORS
  - [x] `REQUEST_TIMEOUTS` Prometheus counter (method, path labels)
  - [x] 2 integration tests (fast request succeeds, disabled timeout works)
- [x] Graceful shutdown / connection draining
  - [x] `graceful_shutdown_timeout_ms` config field (default 30s, 0=immediate) in `ApiV2Config` and `V2Config`
  - [x] `tokio::sync::watch<bool>` shutdown channel in `V2Context` (sender + receiver)
  - [x] `V2Context::trigger_shutdown()` — sends shutdown signal (for programmatic/testing use)
  - [x] `V2Context::shutdown_receiver()` — clones receiver for background tasks/servers
  - [x] `V2Context::shutdown_signal()` — async future that resolves on shutdown
  - [x] OS signal listener (Ctrl+C/SIGINT) spawned in `runtime.rs`
  - [x] `serve_with_graceful_shutdown()` — `axum::serve().with_graceful_shutdown()` + drain timeout via `select!`
  - [x] TLS `serve_tls()` — `tokio::select!` accept loop + atomic connection counter + drain poll loop
  - [x] WebSocket broadcaster exits cleanly on shutdown signal
  - [x] 3 integration tests (stops accepting, drains in-flight, immediate shutdown)
  - [x] Total: 73 v2 tests passing (all green)
- [x] SSE (Server-Sent Events)
  - [x] `GET /v2/sse/blocks` — stream new block notifications (SSE `event: block`, `id: <height>`)
  - [x] `GET /v2/sse/events` — stream filtered on-chain events (SSE `event: event`, `id: <version>`)
  - [x] `SseBlocksParams`: `after_height` query param for resumption after reconnect
  - [x] `SseEventsParams`: `event_types` (comma-separated patterns), `sender`, `start_version`
  - [x] Reuses `EventFilter` from WebSocket for efficient per-event matching
  - [x] Background task per connection: broadcast → filter → mpsc → SSE stream
  - [x] `sse_enabled` config flag (default true) in `ApiV2Config` and `V2Config`
  - [x] Block poller starts when either `websocket_enabled` or `sse_enabled` is true
  - [x] 15-second keep-alive with `text("keep-alive")` comment
  - [x] `lagged` events emitted when client falls behind broadcast buffer
  - [x] Routes added to router, OpenAPI spec, and SSE tag
  - [x] 6 integration tests (blocks stream, events stream, events with filters, blocks with after_height, disabled returns 503, spec includes SSE endpoints)
  - [x] Total: 79 v2 tests passing (all green)
- [x] E2E integration tests
  - [x] `test_e2e_submit_bcs_transaction` — submit a BCS-encoded CreateAccount tx, verify 200 accepted
  - [x] `test_e2e_submit_commit_verify_by_hash` — submit, commit, GET by hash succeeds
  - [x] `test_e2e_submit_and_wait_for_commit` — submit, wait endpoint, concurrent commit, wait returns 200
  - [x] `test_e2e_ws_new_block_on_commit` — WS subscribe new_blocks, commit tx, receive block notification
  - [x] `test_e2e_full_flow_account_creation` — submit CreateAccount, commit, verify account via accounts endpoint
  - [x] Total: 84 v2 tests (79 unit/integration + 5 E2E)
- [x] Documentation cleanup
  - [x] All 6 open questions resolved in scratchpad
  - [x] Phase breakdown updated (Phase 2 gRPC deferred, Phase 4 added)
  - [x] Design docs updated to reflect final implementation

## Implementation Summary

### Test Coverage
| Category | Count | Description |
|----------|-------|-------------|
| Endpoint unit tests | 24 | Core endpoint behavior |
| Co-hosting tests | 6 | Same-port v1/v2 proxy |
| WebSocket tests | 17 | Subscribe/unsubscribe/ping, event filtering |
| TLS tests | 4 | HTTPS health/info/resources, invalid cert |
| OpenAPI tests | 4 | JSON spec, YAML spec, schemas, tags |
| Path normalization | 8 | Dynamic segment normalization |
| Cursor tests | 4 | Encode/decode/version/type mismatch |
| SSE tests | 6 | Blocks stream, events stream, filters |
| Batch tests | 7 | Single/multiple/empty/error/new-methods |
| Timeout/shutdown | 5 | Fast request, disabled timeout, shutdown |
| E2E tests | 5 | Full submit→commit→verify flows |
| **Total** | **~84** | |

### Endpoint Inventory
| Endpoint | Method | Phase |
|----------|--------|-------|
| `/v2/health` | GET | 1 |
| `/v2/info` | GET | 1 |
| `/v2/accounts/:addr/resources` | GET | 1 |
| `/v2/accounts/:addr/resource/:type` | GET | 1 |
| `/v2/accounts/:addr/modules` | GET | 1 |
| `/v2/accounts/:addr/module/:name` | GET | 1 |
| `/v2/accounts/:addr` | GET | 4 |
| `/v2/accounts/:addr/balance/:asset_type` | GET | 4 |
| `/v2/accounts/:addr/transactions` | GET | 1 |
| `/v2/accounts/:addr/events/:creation_number` | GET | 1 |
| `/v2/transactions` | GET | 1 |
| `/v2/transactions` | POST | 1 |
| `/v2/transactions/:hash` | GET | 1 |
| `/v2/transactions/:hash/wait` | GET | 1 |
| `/v2/transactions/simulate` | POST | 4 |
| `/v2/transactions/by_version/:version` | GET | 4 |
| `/v2/blocks/:height` | GET | 1 |
| `/v2/blocks/latest` | GET | 1 |
| `/v2/blocks/by_version/:version` | GET | 4 |
| `/v2/view` | POST | 1 |
| `/v2/estimate_gas_price` | GET | 4 |
| `/v2/tables/:handle/item` | POST | 4 |
| `/v2/batch` | POST | 1 |
| `/v2/ws` | GET (WS) | 1 |
| `/v2/sse/blocks` | GET (SSE) | 4 |
| `/v2/sse/events` | GET (SSE) | 4 |
| `/v2/spec.json` | GET | 1 |
| `/v2/spec.yaml` | GET | 1 |
