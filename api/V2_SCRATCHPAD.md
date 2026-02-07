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
- Advanced event filtering in WebSocket

### Phase 3: Optimization
- Shared state views for batch requests
- Connection-level caching
- Adaptive polling for WebSocket broadcaster
- Performance benchmarking vs v1

## Progress

- [x] Design documents written
- [ ] ApiV2Config struct added
- [ ] Axum/Tower/utoipa dependencies added
- [ ] v2 module structure created
- [ ] V2Context implemented
- [ ] V2Error implemented
- [ ] Health/info endpoints
- [ ] Resource endpoints
- [ ] View function endpoint
- [ ] Transaction endpoints
- [ ] Block endpoints
- [ ] Batch endpoint
- [ ] WebSocket support
- [ ] Middleware (logging, size limit, CORS)
- [ ] Router integration (same-port dispatcher)
- [ ] HTTP/2 (h2c)
- [ ] OpenAPI spec generation
- [ ] Unit tests
- [ ] Integration tests
