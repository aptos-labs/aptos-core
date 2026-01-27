# Medium and Long-Term Performance Improvements

## Overview

This document details medium-risk, high-impact performance improvements that require more significant changes to the API architecture. These improvements focus on scalability, advanced caching strategies, and new capabilities.

**Estimated Timeline**: 4-12 weeks  
**Risk Level**: Medium to High  
**Breaking Changes**: Minimal (additive changes)

---

## Table of Contents

1. [Streaming Responses](#1-streaming-responses)
2. [Parallel Transaction Rendering](#2-parallel-transaction-rendering)
3. [View Function Result Caching](#3-view-function-result-caching)
4. [Optimized Resource Group Handling](#4-optimized-resource-group-handling)
5. [Connection Pooling Improvements](#5-connection-pooling-improvements)
6. [Read Replica Support](#6-read-replica-support)
7. [Implementation Priorities](#7-implementation-priorities)

---

## 1. Streaming Responses

### Problem Statement

Currently, all API responses are fully buffered in memory before being sent to the client. For large responses (e.g., 1000 transactions with events), this causes:
- High memory usage per request
- Slow time-to-first-byte
- Potential OOM under load

### Proposed Solution

Implement streaming responses for endpoints that return large collections.

### Implementation Details

#### 1.1 Streaming Transaction Reader

**File**: `api/src/streaming.rs` (new file)

```rust
use futures::stream::{Stream, StreamExt};
use poem::{web::sse::Event, IntoResponse, Response};
use tokio::sync::mpsc;

/// A stream that yields transactions as they're read from storage
pub struct TransactionStream {
    context: Arc<Context>,
    start_version: Version,
    limit: u16,
    ledger_version: Version,
}

impl TransactionStream {
    pub fn new(
        context: Arc<Context>,
        start_version: Version,
        limit: u16,
        ledger_version: Version,
    ) -> Self {
        Self {
            context,
            start_version,
            limit,
            ledger_version,
        }
    }

    /// Convert to an async stream of JSON lines (NDJSON format)
    pub fn into_ndjson_stream(self) -> impl Stream<Item = Result<String, std::io::Error>> {
        let (tx, rx) = mpsc::channel(32);
        
        let context = self.context.clone();
        let start_version = self.start_version;
        let limit = self.limit;
        let ledger_version = self.ledger_version;
        
        tokio::spawn(async move {
            let batch_size = 10u16; // Process in small batches
            let mut current_version = start_version;
            let mut remaining = limit;
            
            while remaining > 0 {
                let fetch_count = std::cmp::min(batch_size, remaining);
                
                let result = tokio::task::spawn_blocking({
                    let context = context.clone();
                    move || {
                        context.get_transactions(current_version, fetch_count, ledger_version)
                    }
                })
                .await;
                
                match result {
                    Ok(Ok(transactions)) => {
                        for txn in transactions {
                            let json = match serde_json::to_string(&txn) {
                                Ok(j) => j,
                                Err(e) => {
                                    let _ = tx.send(Err(std::io::Error::new(
                                        std::io::ErrorKind::Other,
                                        e,
                                    ))).await;
                                    return;
                                }
                            };
                            
                            if tx.send(Ok(format!("{}\n", json))).await.is_err() {
                                return; // Client disconnected
                            }
                        }
                        
                        current_version += fetch_count as u64;
                        remaining -= fetch_count;
                    }
                    Ok(Err(e)) => {
                        let _ = tx.send(Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e,
                        ))).await;
                        return;
                    }
                    Err(e) => {
                        let _ = tx.send(Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e,
                        ))).await;
                        return;
                    }
                }
            }
        });
        
        tokio_stream::wrappers::ReceiverStream::new(rx)
    }
}
```

#### 1.2 Streaming Endpoint

**File**: `api/src/transactions.rs`

```rust
use poem::{
    handler,
    web::{Data, Query},
    Response,
};

/// Get transactions as a stream (NDJSON format)
#[oai(
    path = "/transactions/stream",
    method = "get",
    operation_id = "get_transactions_stream",
    tag = "ApiTags::Transactions"
)]
async fn get_transactions_stream(
    &self,
    /// Starting ledger version
    start: Query<Option<U64>>,
    /// Number of transactions to stream
    limit: Query<Option<u16>>,
) -> Response {
    let ledger_info = match self.context.get_latest_ledger_info::<BasicError>() {
        Ok(info) => info,
        Err(e) => return e.into_response(),
    };
    
    let ledger_version = ledger_info.version();
    let start_version = start.0.map(|v| v.0).unwrap_or(0);
    let limit = std::cmp::min(
        limit.0.unwrap_or(100),
        self.context.max_transactions_page_size(),
    );
    
    let stream = TransactionStream::new(
        self.context.clone(),
        start_version,
        limit,
        ledger_version,
    );
    
    Response::builder()
        .content_type("application/x-ndjson")
        .header("X-Aptos-Chain-Id", ledger_info.chain_id.to_string())
        .header("X-Aptos-Ledger-Version", ledger_version.to_string())
        .body(Body::from_stream(stream.into_ndjson_stream()))
}
```

#### 1.3 Server-Sent Events (SSE) for Real-Time Updates

```rust
use poem::web::sse::{Event, SSE};

/// Subscribe to new transactions in real-time
#[oai(
    path = "/transactions/subscribe",
    method = "get",
    operation_id = "subscribe_transactions",
    tag = "ApiTags::Transactions"
)]
async fn subscribe_transactions(
    &self,
    /// Filter by sender address (optional)
    sender: Query<Option<Address>>,
) -> SSE {
    let context = self.context.clone();
    let sender_filter = sender.0.map(|a| a.into());
    
    let stream = async_stream::stream! {
        let mut last_version = context
            .get_latest_ledger_info::<BasicError>()
            .map(|li| li.version())
            .unwrap_or(0);
        
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        
        loop {
            interval.tick().await;
            
            let current_version = match context.get_latest_ledger_info::<BasicError>() {
                Ok(li) => li.version(),
                Err(_) => continue,
            };
            
            if current_version > last_version {
                // Fetch new transactions
                let transactions = context
                    .get_transactions(last_version + 1, 100, current_version)
                    .unwrap_or_default();
                
                for txn in transactions {
                    // Apply sender filter if provided
                    if let Some(ref filter) = sender_filter {
                        // Skip if doesn't match filter
                        // ...
                    }
                    
                    let json = serde_json::to_string(&txn).unwrap_or_default();
                    yield Event::message(json);
                }
                
                last_version = current_version;
            }
        }
    };
    
    SSE::new(stream)
}
```

### Expected Impact

| Metric | Before | After |
|--------|--------|-------|
| Memory per large request | 50-200MB | 1-10MB |
| Time to first byte | 500ms-5s | 50-100ms |
| Max concurrent large requests | 10-50 | 100-500 |

---

## 2. Parallel Transaction Rendering

### Problem Statement

`render_transactions_sequential()` processes transactions one at a time. For CPU-bound type conversion work, this underutilizes available cores.

### Proposed Solution

Use Rayon for parallel processing of independent transactions.

### Implementation Details

#### 2.1 Add Rayon Dependency

**File**: `api/Cargo.toml`

```toml
[dependencies]
rayon = "1.10"
```

#### 2.2 Parallel Rendering Implementation

**File**: `api/src/context.rs`

```rust
use rayon::prelude::*;

impl Context {
    /// Render transactions in parallel (for large batches)
    pub fn render_transactions_parallel<E: InternalError + Send>(
        &self,
        ledger_info: &LedgerInfo,
        data: Vec<TransactionOnChainData>,
    ) -> Result<Vec<aptos_api_types::Transaction>, E> {
        if data.is_empty() {
            return Ok(vec![]);
        }
        
        // For small batches, sequential is faster due to parallelization overhead
        if data.len() < 10 {
            return self.render_transactions_non_sequential(ledger_info, data);
        }

        // Batch fetch timestamps
        let versions: Vec<Version> = data.iter().map(|t| t.version).collect();
        let timestamps = self.db
            .get_block_timestamps_batch(&versions)
            .context("Failed to batch fetch timestamps")
            .map_err(|err| {
                E::internal_with_code(err, AptosErrorCode::InternalError, ledger_info)
            })?;

        let state_view = self.latest_state_view_poem(ledger_info)?;
        let db = self.db.clone();
        let indexer_reader = self.indexer_reader.clone();
        
        // Use rayon for parallel conversion
        let results: Result<Vec<_>, _> = data
            .into_par_iter()
            .map(|t| {
                let timestamp = *timestamps.get(&t.version).unwrap_or(&0);
                let converter = state_view.as_converter(db.clone(), indexer_reader.clone());
                converter.try_into_onchain_transaction(timestamp, t)
            })
            .collect();
        
        results
            .context("Failed to convert transaction data")
            .map_err(|err| {
                E::internal_with_code(err, AptosErrorCode::InternalError, ledger_info)
            })
    }
    
    /// Automatically choose sequential or parallel based on batch size
    pub fn render_transactions_auto<E: InternalError + Send>(
        &self,
        ledger_info: &LedgerInfo,
        data: Vec<TransactionOnChainData>,
        sequential_hint: bool,
    ) -> Result<Vec<aptos_api_types::Transaction>, E> {
        // Use sequential for ordered results or small batches
        if sequential_hint || data.len() < 10 {
            let timestamp = if !data.is_empty() {
                self.get_block_timestamp(ledger_info, data[0].version)?
            } else {
                0
            };
            self.render_transactions_sequential(ledger_info, data, timestamp)
        } else {
            self.render_transactions_parallel(ledger_info, data)
        }
    }
}
```

#### 2.3 Configure Thread Pool

**File**: `api/src/runtime.rs`

```rust
pub fn configure_rayon_pool(num_threads: usize) {
    rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .thread_name(|i| format!("api-rayon-{}", i))
        .build_global()
        .expect("Failed to configure rayon thread pool");
}

pub fn bootstrap(
    config: &NodeConfig,
    // ...
) -> anyhow::Result<Runtime> {
    // Configure rayon before starting the API
    let rayon_threads = config.api.max_runtime_workers
        .unwrap_or_else(|| num_cpus::get() / 2);
    configure_rayon_pool(rayon_threads);
    
    // ... rest of bootstrap
}
```

### Expected Impact

| Batch Size | Before (ms) | After (ms) | Speedup |
|------------|-------------|------------|---------|
| 10 txns | 15 | 15 | 1x |
| 50 txns | 75 | 25 | 3x |
| 100 txns | 150 | 40 | 3.75x |

---

## 3. View Function Result Caching

### Problem Statement

Popular view functions (e.g., `0x1::coin::balance`) are called repeatedly with the same arguments. Each call requires full VM execution.

### Proposed Solution

Cache view function results with intelligent invalidation.

### Implementation Details

#### 3.1 View Cache Structure

**File**: `api/src/view_cache.rs` (new file)

```rust
use mini_moka::sync::Cache;
use std::time::{Duration, Instant};

/// Key for view function cache
#[derive(Clone, Hash, Eq, PartialEq)]
pub struct ViewCacheKey {
    pub module: ModuleId,
    pub function: Identifier,
    pub type_args: Vec<TypeTag>,
    pub args: Vec<Vec<u8>>,
    pub version: Version,
}

/// Cached view function result
#[derive(Clone)]
pub struct ViewCacheEntry {
    pub values: Vec<Vec<u8>>,
    pub gas_used: u64,
    pub cached_at: Instant,
}

pub struct ViewFunctionCache {
    cache: Cache<ViewCacheKey, ViewCacheEntry>,
    config: ViewCacheConfig,
}

#[derive(Clone, Debug)]
pub struct ViewCacheConfig {
    /// Maximum entries in cache
    pub max_entries: u64,
    /// TTL for cache entries
    pub ttl: Duration,
    /// Maximum staleness (versions behind latest)
    pub max_version_staleness: u64,
    /// Functions to never cache (e.g., randomness-dependent)
    pub nocache_functions: HashSet<(AccountAddress, String, String)>,
    /// Functions to always cache (known pure functions)
    pub cache_functions: HashSet<(AccountAddress, String, String)>,
}

impl ViewFunctionCache {
    pub fn new(config: ViewCacheConfig) -> Self {
        let cache = Cache::builder()
            .max_capacity(config.max_entries)
            .time_to_live(config.ttl)
            .build();
        
        Self { cache, config }
    }
    
    /// Check if a function should be cached
    pub fn should_cache(&self, module: &ModuleId, function: &Identifier) -> bool {
        let key = (
            *module.address(),
            module.name().to_string(),
            function.to_string(),
        );
        
        // Explicit nocache takes precedence
        if self.config.nocache_functions.contains(&key) {
            return false;
        }
        
        // Explicit cache list
        if self.config.cache_functions.contains(&key) {
            return true;
        }
        
        // Default: cache common framework functions
        if module.address() == &AccountAddress::ONE {
            // Cache read-only coin/token/account functions
            let cacheable_modules = ["coin", "fungible_asset", "account", "object"];
            return cacheable_modules.contains(&module.name().as_str());
        }
        
        false
    }
    
    pub fn get(&self, key: &ViewCacheKey, current_version: Version) -> Option<ViewCacheEntry> {
        // Check version staleness
        if current_version.saturating_sub(key.version) > self.config.max_version_staleness {
            return None;
        }
        
        self.cache.get(key)
    }
    
    pub fn insert(&self, key: ViewCacheKey, entry: ViewCacheEntry) {
        self.cache.insert(key, entry);
    }
}
```

#### 3.2 Integration with View Function API

**File**: `api/src/view_function.rs`

```rust
fn view_request(
    context: Arc<Context>,
    accept_type: AcceptType,
    request: ViewFunctionRequest,
    ledger_version: Query<Option<U64>>,
) -> BasicResultWith404<Vec<MoveValue>> {
    let (ledger_info, requested_version) = context
        .get_latest_ledger_info_and_verify_lookup_version(ledger_version.map(|v| v.0))?;

    let view_function: ViewFunction = /* ... parse request ... */;
    
    // Check cache
    if context.view_cache().should_cache(&view_function.module, &view_function.function) {
        let cache_key = ViewCacheKey {
            module: view_function.module.clone(),
            function: view_function.function.clone(),
            type_args: view_function.ty_args.clone(),
            args: view_function.args.clone(),
            version: requested_version,
        };
        
        if let Some(cached) = context.view_cache().get(&cache_key, ledger_info.version()) {
            metrics::VIEW_CACHE_HITS.inc();
            return build_response(accept_type, cached.values, cached.gas_used, &ledger_info);
        }
        
        metrics::VIEW_CACHE_MISSES.inc();
        
        // Execute and cache
        let output = AptosVM::execute_view_function(/* ... */);
        
        if let Ok(ref values) = output.values {
            context.view_cache().insert(cache_key, ViewCacheEntry {
                values: values.clone(),
                gas_used: output.gas_used,
                cached_at: Instant::now(),
            });
        }
        
        // ... rest of response building
    } else {
        // Execute without caching
        // ...
    }
}
```

### Expected Impact

| Scenario | Before | After |
|----------|--------|-------|
| Repeated balance check | 5-20ms | <0.1ms |
| Cache hit rate (popular functions) | 0% | 60-80% |
| VM executions reduced | 0% | 50-70% |

---

## 4. Optimized Resource Group Handling

### Problem Statement

Resource groups are expanded inline in `get_resources_by_pagination()`, which:
- Increases response size
- Requires deserializing all group members
- Doesn't allow querying specific group members

### Proposed Solution

Add dedicated endpoints for resource groups with optional expansion.

### Implementation Details

#### 4.1 Resource Group Info Type

**File**: `api/types/src/resource_group.rs` (new file)

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceGroupInfo {
    /// The resource group tag
    pub group_tag: MoveStructTag,
    /// Address of the account
    pub address: Address,
    /// Total size of the group in bytes
    pub size_bytes: u64,
    /// Number of resources in the group
    pub member_count: u32,
    /// List of member types (without data)
    pub members: Vec<MoveStructTag>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceGroupMember {
    /// The resource type
    #[serde(rename = "type")]
    pub resource_type: MoveStructTag,
    /// The resource data
    pub data: MoveValue,
    /// Size in bytes
    pub size_bytes: u64,
}
```

#### 4.2 Resource Group Endpoints

**File**: `api/src/accounts.rs`

```rust
/// Get resource groups for an account (metadata only)
#[oai(
    path = "/accounts/:address/resource_groups",
    method = "get",
    operation_id = "get_account_resource_groups",
    tag = "ApiTags::Accounts"
)]
async fn get_account_resource_groups(
    &self,
    accept_type: AcceptType,
    address: Path<Address>,
    ledger_version: Query<Option<U64>>,
) -> BasicResultWith404<Vec<ResourceGroupInfo>> {
    // Return metadata about resource groups without expanding them
    // ...
}

/// Get a specific resource group with optional expansion
#[oai(
    path = "/accounts/:address/resource_group/:group_tag",
    method = "get",
    operation_id = "get_account_resource_group",
    tag = "ApiTags::Accounts"
)]
async fn get_account_resource_group(
    &self,
    accept_type: AcceptType,
    address: Path<Address>,
    group_tag: Path<MoveStructTag>,
    /// If true, expand all members (default: false)
    expand: Query<Option<bool>>,
    /// Specific member types to include (if expand=true)
    members: Query<Option<Vec<MoveStructTag>>>,
    ledger_version: Query<Option<U64>>,
) -> BasicResultWith404<ResourceGroupResponse> {
    let should_expand = expand.0.unwrap_or(false);
    
    if should_expand {
        // Fetch and expand group members
        // Optionally filter to specific member types
    } else {
        // Return group info with member list but no data
    }
}
```

#### 4.3 Efficient Group Member Access

```rust
impl Context {
    /// Get a single member from a resource group without deserializing others
    pub fn get_resource_group_member(
        &self,
        address: AccountAddress,
        group_tag: &StructTag,
        member_tag: &StructTag,
        version: Version,
    ) -> Result<Option<Vec<u8>>> {
        let state_key = StateKey::resource_group(&address, group_tag);
        let group_bytes = self.get_state_value(&state_key, version)?;
        
        if let Some(bytes) = group_bytes {
            // Deserialize just to get the map structure, then extract single member
            let group: BTreeMap<StructTag, Vec<u8>> = bcs::from_bytes(&bytes)?;
            Ok(group.get(member_tag).cloned())
        } else {
            Ok(None)
        }
    }
}
```

---

## 5. Connection Pooling Improvements

### Problem Statement

Current long-poll implementation (`wait_by_hash`) has a hard limit on active connections, and there's no connection reuse optimization.

### Proposed Solution

Implement connection coalescing and improved queue management.

### Implementation Details

#### 5.1 Request Coalescing

```rust
use dashmap::DashMap;
use tokio::sync::broadcast;

/// Coalesce multiple requests waiting for the same transaction
pub struct TransactionWaitCoalescer {
    /// Map of transaction hash -> broadcast sender
    waiters: DashMap<HashValue, broadcast::Sender<TransactionResult>>,
    /// Maximum waiters per transaction
    max_waiters_per_tx: usize,
}

impl TransactionWaitCoalescer {
    pub async fn wait_for_transaction(
        &self,
        hash: HashValue,
        timeout: Duration,
    ) -> Result<TransactionResult, WaitError> {
        // Check if someone is already waiting for this transaction
        let receiver = {
            let entry = self.waiters.entry(hash);
            match entry {
                dashmap::mapref::entry::Entry::Occupied(e) => {
                    // Join existing wait group
                    e.get().subscribe()
                }
                dashmap::mapref::entry::Entry::Vacant(e) => {
                    // Create new wait group
                    let (tx, rx) = broadcast::channel(1);
                    e.insert(tx);
                    
                    // Spawn the actual wait task (only once per hash)
                    self.spawn_wait_task(hash);
                    
                    rx
                }
            }
        };
        
        // Wait for result with timeout
        tokio::time::timeout(timeout, receiver.recv())
            .await
            .map_err(|_| WaitError::Timeout)?
            .map_err(|_| WaitError::ChannelClosed)
    }
    
    fn spawn_wait_task(&self, hash: HashValue) {
        let waiters = self.waiters.clone();
        
        tokio::spawn(async move {
            // Poll for transaction
            let result = /* ... poll storage ... */;
            
            // Notify all waiters
            if let Some((_, sender)) = waiters.remove(&hash) {
                let _ = sender.send(result);
            }
        });
    }
}
```

---

## 6. Read Replica Support

### Problem Statement

All API requests hit the primary database, which can become a bottleneck under high read load.

### Proposed Solution

Support read replicas for geographically distributed API servers.

### Implementation Details

#### 6.1 Replica-Aware Context

```rust
pub struct Context {
    /// Primary database (for writes and consistency-critical reads)
    primary_db: Arc<dyn DbReader>,
    
    /// Read replicas (for eventually consistent reads)
    read_replicas: Vec<Arc<dyn DbReader>>,
    
    /// Strategy for selecting replica
    replica_selector: ReplicaSelector,
    
    // ... other fields ...
}

pub enum ReplicaSelector {
    /// Round-robin selection
    RoundRobin(AtomicUsize),
    /// Least connections
    LeastConnections,
    /// Random
    Random,
    /// Latency-based (requires health checks)
    LatencyBased,
}

impl Context {
    /// Get a database reader for read operations
    pub fn read_db(&self, consistency: ReadConsistency) -> Arc<dyn DbReader> {
        match consistency {
            ReadConsistency::Strong => self.primary_db.clone(),
            ReadConsistency::Eventual => {
                if self.read_replicas.is_empty() {
                    self.primary_db.clone()
                } else {
                    self.select_replica()
                }
            }
        }
    }
    
    fn select_replica(&self) -> Arc<dyn DbReader> {
        match &self.replica_selector {
            ReplicaSelector::RoundRobin(counter) => {
                let idx = counter.fetch_add(1, Ordering::Relaxed) % self.read_replicas.len();
                self.read_replicas[idx].clone()
            }
            // ... other strategies ...
        }
    }
}

#[derive(Clone, Copy)]
pub enum ReadConsistency {
    /// Read from primary (guaranteed latest)
    Strong,
    /// Read from replica (may be slightly stale)
    Eventual,
}
```

#### 6.2 Consistency Hints in Endpoints

```rust
#[oai(
    path = "/accounts/:address/resources",
    method = "get",
    operation_id = "get_account_resources",
    tag = "ApiTags::Accounts"
)]
async fn get_account_resources(
    &self,
    // ... existing params ...
    
    /// Read consistency level (strong or eventual)
    /// Default: eventual for better performance
    consistency: Query<Option<ReadConsistency>>,
) -> BasicResultWith404<Vec<MoveResource>> {
    let consistency = consistency.0.unwrap_or(ReadConsistency::Eventual);
    let db = self.context.read_db(consistency);
    
    // Use selected db for reads
    // ...
}
```

---

## 7. Implementation Priorities

### Phase 1: Parallel Processing (Weeks 1-3)

| Task | Priority | Effort | Impact |
|------|----------|--------|--------|
| Add Rayon dependency | P0 | Low | - |
| Implement parallel rendering | P0 | Medium | High |
| Add configuration options | P0 | Low | - |
| Benchmark & tune | P0 | Medium | - |

### Phase 2: Streaming (Weeks 4-7)

| Task | Priority | Effort | Impact |
|------|----------|--------|--------|
| NDJSON streaming endpoint | P1 | Medium | High |
| SSE subscription endpoint | P2 | High | Medium |
| Client library updates | P1 | Medium | - |

### Phase 3: Advanced Caching (Weeks 8-10)

| Task | Priority | Effort | Impact |
|------|----------|--------|--------|
| View function cache | P1 | Medium | High |
| Cache configuration | P1 | Low | - |
| Metrics & monitoring | P1 | Low | - |

### Phase 4: Infrastructure (Weeks 11-12)

| Task | Priority | Effort | Impact |
|------|----------|--------|--------|
| Connection coalescing | P2 | Medium | Medium |
| Read replica support | P3 | High | High |
| Resource group optimization | P2 | Medium | Medium |

---

## Testing Strategy

### Load Testing

```bash
# Test parallel rendering
wrk -t12 -c400 -d30s "http://localhost:8080/v1/transactions?limit=100"

# Test streaming endpoint
curl -N "http://localhost:8080/v1/transactions/stream?limit=1000" | wc -l

# Test view function cache
for i in {1..1000}; do
  curl -s -X POST "http://localhost:8080/v1/view" \
    -H "Content-Type: application/json" \
    -d '{"function":"0x1::coin::balance","type_arguments":["0x1::aptos_coin::AptosCoin"],"arguments":["0x1"]}'
done
```

### Benchmarking Tools

- **wrk2**: HTTP benchmarking with latency percentiles
- **flamegraph**: CPU profiling for hotspots
- **heaptrack**: Memory allocation analysis
- **tokio-console**: Async task debugging

---

## Rollout Considerations

1. **Feature Flags**: All features behind configuration flags
2. **Gradual Enablement**: Start with testnet, then mainnet
3. **Monitoring**: Comprehensive metrics before rollout
4. **Rollback Plan**: Easy disable via configuration
5. **Documentation**: Update API docs for new endpoints
