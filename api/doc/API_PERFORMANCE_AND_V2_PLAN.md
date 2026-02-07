# Aptos REST API: Performance Analysis and V2 Roadmap

## Executive Summary

This document provides a comprehensive analysis of the current Aptos REST API implementation (`api/`), identifies performance improvement opportunities, and outlines a plan for a potential v2 API redesign. The analysis is based on a thorough review of the codebase, including the runtime, context, endpoints, type conversions, and configuration.

---

## Table of Contents

1. [Current Architecture Overview](#1-current-architecture-overview)
2. [Performance Analysis](#2-performance-analysis)
3. [Identified Performance Improvements](#3-identified-performance-improvements)
4. [V2 API Design Proposal](#4-v2-api-design-proposal)
5. [Migration Strategy](#5-migration-strategy)
6. [Implementation Priorities](#6-implementation-priorities)

---

## 1. Current Architecture Overview

### 1.1 Technology Stack

- **Web Framework**: Poem (OpenAPI-native Rust web framework)
- **Serialization**: JSON and BCS (Binary Canonical Serialization)
- **Runtime**: Tokio with configurable worker pool
- **Storage Interface**: `DbReader` trait abstraction over RocksDB
- **Async Handling**: `tokio::task::spawn_blocking` for DB operations

### 1.2 API Categories

| Category | Endpoints | Primary Use |
|----------|-----------|-------------|
| **Accounts** | `/accounts/:address`, `/accounts/:address/resources`, `/accounts/:address/modules` | Account data, resources, modules |
| **Transactions** | `/transactions`, `/transactions/by_hash/:hash`, `/transactions/by_version/:version` | Transaction queries and submission |
| **Blocks** | `/blocks/by_height/:height`, `/blocks/by_version/:version` | Block information |
| **Events** | `/accounts/:address/events/:creation_number` | Event queries |
| **State** | `/accounts/:address/resource/:type`, `/tables/:handle/item` | State lookups |
| **View** | `/view` | View function execution |
| **General** | `/-/healthy`, `/spec`, `/estimate_gas_price` | Health checks, API spec |

### 1.3 Request Flow

```
Request → Poem Router → Middleware (CORS, Compression, Size Limit, Logging)
       → Endpoint Handler → spawn_blocking(DB Operations)
       → Type Conversion (MoveConverter) → Response (JSON/BCS)
```

### 1.4 Key Components

1. **Context** (`context.rs`): Central application state holder with DB access, mempool sender, and caches
2. **MoveConverter** (`types/convert.rs`): Converts internal types to API response types
3. **Response System** (`response.rs`): Macro-based response type generation with proper OpenAPI spec
4. **Metrics** (`metrics.rs`): Prometheus metrics for request latency, gas estimation, etc.

---

## 2. Performance Analysis

### 2.1 Current Performance Characteristics

#### Strengths
- **BCS Support**: Direct bytes passthrough from storage when using BCS Accept header
- **Caching**: Gas schedule, gas estimation, and execution config cached per epoch
- **Compression**: Optional gzip compression middleware
- **Pagination**: Cursor-based pagination for large result sets
- **Connection Pooling**: Long-poll support with active connection limits

#### Bottlenecks Identified

1. **JSON Serialization Overhead**
   - Every JSON response requires full Move type resolution via `MoveConverter`
   - `try_into_resource()` calls `AptosValueAnnotator::view_resource()` for each resource
   - Type annotations fetched from storage for every request

2. **Sequential Transaction Processing**
   - `render_transactions_sequential()` processes transactions one-by-one
   - Each transaction requires individual timestamp lookup

3. **Blocking Operations in Async Context**
   - Heavy use of `spawn_blocking` for DB operations
   - Creates thread-per-request pattern for CPU-bound work

4. **Resource Group Expansion**
   - `get_resources_by_pagination()` expands resource groups inline
   - No caching of resource group contents

5. **State View Creation**
   - New state view created for each request
   - `latest_state_view_poem()` called multiple times per request in some paths

### 2.2 Metrics Analysis Points

Current metrics tracked:
- `aptos_api_requests`: Latency by method/operation/status (sub-ms buckets)
- `aptos_api_response_status`: Latency by status code
- `aptos_api_post_body_bytes`: POST body sizes
- `aptos_api_gas_estimate`: Gas estimation values
- `aptos_api_gas_used`: Gas usage per operation
- `aptos_api_wait_transaction`: Long-poll gauge

**Missing Metrics:**
- DB read latency per operation type
- Type conversion time
- Cache hit/miss ratios
- Memory allocation patterns

---

## 3. Identified Performance Improvements

### 3.1 Short-Term Improvements (Low Risk)

#### 3.1.1 Add Response Caching Layer

**Location**: `api/src/context.rs`

```rust
// Proposed: Add LRU cache for frequently accessed resources
pub struct Context {
    // ... existing fields ...
    resource_cache: Arc<Cache<(AccountAddress, StructTag, Version), Vec<u8>>>,
    module_cache: Arc<Cache<(AccountAddress, Identifier, Version), Vec<u8>>>,
}
```

**Benefits**: Reduce DB reads for hot resources (e.g., `0x1::coin::CoinStore`)

**Risk**: Cache invalidation complexity; start with short TTL or version-based keys

#### 3.1.2 Batch Transaction Timestamp Lookups

**Location**: `api/src/context.rs::render_transactions_sequential()`

**Current**:
```rust
for t in data {
    let timestamp = self.db.get_block_timestamp(t.version)?;
    // ...
}
```

**Proposed**:
```rust
// Batch fetch block info for version range
let timestamps = self.db.get_block_timestamps_batch(
    data.iter().map(|t| t.version).collect()
)?;
```

**Benefits**: Single DB call instead of N calls

#### 3.1.3 Lazy Type Annotation

**Location**: `api/types/src/convert.rs`

Currently, `try_into_resource()` always fetches type annotations. For BCS responses or when clients don't need decoded values, this is wasted work.

**Proposed**: Add `lazy_convert` flag to defer type resolution

```rust
pub fn try_into_resource_lazy(
    &self,
    tag: &StructTag,
    bytes: &[u8],
    decode: bool,
) -> Result<MoveResource> {
    if decode {
        self.inner.view_resource(tag, bytes)?.try_into()
    } else {
        // Return raw bytes with type tag only
        Ok(MoveResource::raw(tag, bytes))
    }
}
```

#### 3.1.4 State View Reuse

**Location**: Multiple endpoint handlers

**Current**: Many handlers create state view multiple times:
```rust
let state_view = self.context.latest_state_view_poem(&ledger_info)?;
// ... later ...
let state_view2 = self.context.latest_state_view_poem(&ledger_info)?;
```

**Proposed**: Create once at handler entry, pass through

### 3.2 Medium-Term Improvements (Moderate Risk)

#### 3.2.1 Streaming Responses

**Current**: All responses are fully buffered before sending

**Proposed**: Implement streaming for large responses

```rust
// For /transactions endpoint with large page sizes
#[oai(path = "/transactions/stream", method = "get")]
async fn get_transactions_stream(
    &self,
    // ...
) -> poem::Response {
    let body = Body::from_async_read(TransactionStreamReader::new(/*...*/));
    Response::builder()
        .content_type("application/x-ndjson")
        .body(body)
}
```

**Benefits**: Lower memory usage, faster time-to-first-byte

#### 3.2.2 Parallel Transaction Rendering

**Location**: `api/src/context.rs`

**Proposed**: Use `rayon` or `tokio::task::spawn_blocking` with parallelism

```rust
pub fn render_transactions_parallel(
    &self,
    data: Vec<TransactionOnChainData>,
) -> Result<Vec<Transaction>> {
    use rayon::prelude::*;
    
    data.into_par_iter()
        .map(|t| self.convert_single_transaction(t))
        .collect()
}
```

#### 3.2.3 Pre-computed View Function Results

For commonly called view functions (e.g., token balance checks), maintain a cache of recent results:

```rust
pub struct ViewFunctionCache {
    cache: Cache<(ModuleId, Identifier, Vec<TypeTag>, Vec<Vec<u8>>), CachedViewResult>,
}

struct CachedViewResult {
    values: Vec<Vec<u8>>,
    computed_at_version: Version,
    gas_used: u64,
}
```

**Risk**: Staleness; only applicable for read-only functions with known inputs

#### 3.2.4 Optimized Resource Group Handling

**Current**: Resource groups are expanded in `get_resources_by_pagination()`

**Proposed**: Add dedicated endpoint for resource groups without expansion:

```rust
#[oai(path = "/accounts/:address/resource_groups", method = "get")]
async fn get_account_resource_groups(&self, ...) -> BasicResultWith404<Vec<ResourceGroupInfo>>
```

### 3.3 Long-Term Improvements (Higher Risk)

#### 3.3.1 Read Replicas for API

Introduce read replica support for geographically distributed API servers:

```rust
pub struct Context {
    primary_db: Arc<dyn DbReader>,
    read_replicas: Vec<Arc<dyn DbReader>>,
    replica_selector: ReplicaSelector,
}
```

#### 3.3.2 Query Language / GraphQL Support

Add a query endpoint that allows clients to specify exactly what data they need:

```graphql
query {
  account(address: "0x1") {
    resources(filter: { type: "0x1::coin::CoinStore" }) {
      type
      data { coin { value } }
    }
  }
}
```

#### 3.3.3 WebSocket Subscriptions

Real-time subscriptions for events and transactions:

```rust
#[oai(path = "/ws/subscribe", method = "get")]
async fn subscribe_events(
    &self,
    ws: WebSocket,
    event_types: Query<Vec<String>>,
) -> impl IntoResponse {
    // ...
}
```

---

## 4. V2 API Design Proposal

### 4.1 Design Principles

1. **Performance First**: Default to BCS, opt-in to JSON
2. **Explicit Over Implicit**: No magic type resolution unless requested
3. **Streaming Native**: Support streaming for large responses
4. **Versioned Types**: Include version in all API types for future evolution
5. **Batch Operations**: First-class support for batch requests
6. **Backward Compatible Headers**: V2 endpoints under `/v2` prefix

### 4.2 Proposed V2 Endpoint Structure

```
/v2/
├── accounts/
│   ├── /:address                          # Account info
│   ├── /:address/resources                # List resources (paginated)
│   ├── /:address/resources/:type          # Single resource
│   ├── /:address/modules                  # List modules (paginated)
│   ├── /:address/modules/:name            # Single module
│   └── /:address/balance                  # Unified balance endpoint
├── transactions/
│   ├── /                                  # List transactions
│   ├── /by_hash/:hash                     # By hash
│   ├── /by_version/:version               # By version
│   ├── /submit                            # Submit (single)
│   ├── /submit_batch                      # Submit batch
│   ├── /simulate                          # Simulate
│   └── /stream                            # Streaming endpoint (new)
├── blocks/
│   ├── /latest                            # Latest block (new)
│   ├── /by_height/:height                 
│   └── /by_version/:version               
├── events/
│   ├── /by_key/:address/:creation_number  
│   └── /stream                            # Streaming endpoint (new)
├── state/
│   ├── /resource/:address/:type           
│   ├── /module/:address/:name             
│   ├── /table/:handle/:key                # Simplified table access
│   └── /raw/:state_key                    # Raw state access
├── view/
│   ├── /                                  # Execute view function
│   └── /batch                             # Batch view functions (new)
├── gas/
│   └── /estimate                          # Gas estimation
└── health/
    ├── /ready                             # Readiness probe
    └── /live                              # Liveness probe
```

### 4.3 V2 Type Changes

#### 4.3.1 Envelope Type

All V2 responses wrapped in standard envelope:

```rust
#[derive(Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub data: T,
    pub ledger_info: LedgerInfoSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<ApiWarning>>,
}

#[derive(Serialize, Deserialize)]
pub struct LedgerInfoSummary {
    pub chain_id: u8,
    pub ledger_version: u64,
    pub ledger_timestamp_usec: u64,
}
```

#### 4.3.2 Simplified Transaction Type

Remove redundant fields, add computed fields:

```rust
#[derive(Serialize, Deserialize)]
pub struct TransactionV2 {
    pub version: u64,
    pub hash: HashValue,
    pub sender: Option<AccountAddress>,  // None for non-user txns
    pub sequence_number: Option<u64>,
    pub gas_used: u64,
    pub success: bool,
    pub vm_status: String,
    pub timestamp_usec: u64,
    
    // Type-specific payload
    #[serde(flatten)]
    pub payload: TransactionPayloadV2,
    
    // Only included if requested
    #[serde(skip_serializing_if = "Option::is_none")]
    pub events: Option<Vec<EventV2>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changes: Option<Vec<WriteSetChangeV2>>,
}
```

#### 4.3.3 Resource with Metadata

```rust
#[derive(Serialize, Deserialize)]
pub struct MoveResourceV2 {
    #[serde(rename = "type")]
    pub resource_type: String,
    pub data: serde_json::Value,
    
    // V2 additions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_key_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
}
```

### 4.4 V2 Query Parameters

Standardized query parameters across all endpoints:

| Parameter | Type | Description |
|-----------|------|-------------|
| `ledger_version` | `u64` | Historical query at specific version |
| `start` | `string` | Pagination cursor |
| `limit` | `u16` | Page size (default: 25, max: 1000) |
| `include` | `string[]` | Include optional fields (e.g., `events`, `changes`) |
| `format` | `json` \| `bcs` | Response format (default: `bcs`) |

### 4.5 V2 Error Response

```rust
#[derive(Serialize, Deserialize)]
pub struct ApiErrorV2 {
    pub error_code: String,          // e.g., "RESOURCE_NOT_FOUND"
    pub message: String,
    pub ledger_info: Option<LedgerInfoSummary>,
    
    // For VM errors
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vm_error_code: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_index: Option<usize>,  // For batch operations
}
```

### 4.6 V2 Batch Request Format

```rust
#[derive(Serialize, Deserialize)]
pub struct BatchRequest {
    pub operations: Vec<BatchOperation>,
}

#[derive(Serialize, Deserialize)]
pub struct BatchOperation {
    pub id: String,  // Client-provided ID for correlation
    pub method: String,  // e.g., "get_resource", "view"
    pub params: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
pub struct BatchResponse {
    pub results: Vec<BatchResult>,
}

#[derive(Serialize, Deserialize)]
pub struct BatchResult {
    pub id: String,
    #[serde(flatten)]
    pub result: Result<serde_json::Value, ApiErrorV2>,
}
```

---

## 5. Migration Strategy

### 5.1 Phased Rollout

**Phase 1: Performance Improvements (v1)**
- Implement caching layer
- Optimize batch timestamp lookups
- Add missing metrics
- Estimated: 2-4 weeks

**Phase 2: V2 Core Endpoints**
- `/v2/accounts/*`
- `/v2/transactions/*` (read-only)
- V2 type definitions
- Estimated: 4-6 weeks

**Phase 3: V2 Write Endpoints**
- `/v2/transactions/submit`
- `/v2/transactions/simulate`
- `/v2/view/*`
- Estimated: 3-4 weeks

**Phase 4: V2 Advanced Features**
- Streaming endpoints
- Batch operations
- WebSocket subscriptions
- Estimated: 6-8 weeks

### 5.2 Backward Compatibility

1. **V1 Maintained**: All v1 endpoints continue to work unchanged
2. **Deprecation Warnings**: Add `X-Api-Deprecated` header to v1 responses
3. **Documentation**: Clear migration guides with code examples
4. **SDK Updates**: Update official SDKs to support both v1 and v2

### 5.3 Feature Flags

```rust
pub struct ApiConfig {
    // ...existing fields...
    
    /// Enable V2 API endpoints
    pub v2_enabled: bool,
    
    /// Enable streaming responses
    pub streaming_enabled: bool,
    
    /// Enable batch operations
    pub batch_enabled: bool,
    
    /// Default response format for V2 (json or bcs)
    pub v2_default_format: ResponseFormat,
}
```

---

## 6. Implementation Priorities

### Priority 1: Quick Wins (1-2 weeks)

| Task | Impact | Effort | Risk |
|------|--------|--------|------|
| Add cache hit/miss metrics | Medium | Low | Low |
| Batch timestamp lookups | High | Low | Low |
| State view reuse | Medium | Low | Low |
| Add DB operation timing metrics | High | Low | Low |

### Priority 2: Performance Foundations (2-4 weeks)

| Task | Impact | Effort | Risk |
|------|--------|--------|------|
| Resource cache layer | High | Medium | Medium |
| Parallel transaction rendering | High | Medium | Medium |
| Lazy type annotation for BCS | Medium | Low | Low |

### Priority 3: V2 Foundation (4-6 weeks)

| Task | Impact | Effort | Risk |
|------|--------|--------|------|
| V2 type definitions | High | Medium | Low |
| V2 response envelope | High | Low | Low |
| V2 accounts endpoints | High | Medium | Low |
| V2 transactions read endpoints | High | Medium | Low |

### Priority 4: V2 Advanced (6+ weeks)

| Task | Impact | Effort | Risk |
|------|--------|--------|------|
| Streaming responses | High | High | Medium |
| Batch request support | High | Medium | Medium |
| V2 write endpoints | High | Medium | Medium |
| WebSocket subscriptions | Medium | High | High |

---

## Appendix A: Files to Modify

### Performance Improvements
- `api/src/context.rs` - Add caching, batch operations
- `api/src/transactions.rs` - Parallel rendering
- `api/types/src/convert.rs` - Lazy conversion
- `api/src/metrics.rs` - Additional metrics
- `config/src/config/api_config.rs` - Cache configuration

### V2 API
- `api/src/lib.rs` - New API tags and module exports
- `api/src/v2/` - New directory for V2 endpoints
- `api/types/src/v2/` - New directory for V2 types
- `api/src/runtime.rs` - V2 route registration

---

## Appendix B: Benchmark Recommendations

### Endpoints to Benchmark
1. `GET /v1/accounts/:address/resources` - Resource listing
2. `GET /v1/transactions` - Transaction listing
3. `POST /v1/transactions` - Transaction submission
4. `POST /v1/view` - View function execution
5. `GET /v1/blocks/by_height/:height?with_transactions=true` - Block with txns

### Metrics to Capture
- P50/P95/P99 latency
- Requests per second at various concurrency levels
- Memory usage under load
- DB read amplification

### Tools
- `wrk2` for HTTP benchmarking
- `flamegraph` for CPU profiling
- `heaptrack` for memory profiling

---

## Appendix C: Related Issues and PRs

*To be populated with relevant GitHub issues as implementation progresses*

---

**Document Version**: 1.0  
**Last Updated**: January 27, 2026  
**Author**: API Performance Analysis
