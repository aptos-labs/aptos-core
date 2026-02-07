# Short-Term Performance Improvements

## Overview

This document details low-risk, high-impact performance improvements that can be implemented in the current v1 API without breaking changes. These improvements focus on reducing latency, decreasing database load, and optimizing resource usage.

**Estimated Timeline**: 2-4 weeks  
**Risk Level**: Low  
**Breaking Changes**: None

---

## Table of Contents

1. [Response Caching Layer](#1-response-caching-layer)
2. [Batch Timestamp Lookups](#2-batch-timestamp-lookups)
3. [State View Reuse](#3-state-view-reuse)
4. [Lazy Type Annotation](#4-lazy-type-annotation)
5. [Additional Metrics](#5-additional-metrics)
6. [Implementation Plan](#6-implementation-plan)

---

## 1. Response Caching Layer

### Problem Statement

Currently, every API request results in direct database reads, even for frequently accessed resources like:
- `0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>` (checked on almost every transaction)
- `0x1::account::Account` (account sequence numbers)
- Framework modules (rarely change)

### Proposed Solution

Add an LRU cache layer in the `Context` struct for hot resources and modules.

### Implementation Details

#### 1.1 Cache Structure

**File**: `api/src/context.rs`

```rust
use mini_moka::sync::Cache;
use std::time::Duration;

/// Cache key for resources
#[derive(Clone, Hash, Eq, PartialEq)]
pub struct ResourceCacheKey {
    pub address: AccountAddress,
    pub struct_tag: StructTag,
    pub version: Version,
}

/// Cache key for modules  
#[derive(Clone, Hash, Eq, PartialEq)]
pub struct ModuleCacheKey {
    pub address: AccountAddress,
    pub module_name: Identifier,
    pub version: Version,
}

pub struct Context {
    // ... existing fields ...
    
    /// LRU cache for frequently accessed resources
    /// Key: (address, struct_tag, version) -> Value: serialized bytes
    resource_cache: Cache<ResourceCacheKey, Arc<Vec<u8>>>,
    
    /// LRU cache for modules
    /// Key: (address, module_name, version) -> Value: serialized bytes
    module_cache: Cache<ModuleCacheKey, Arc<Vec<u8>>>,
    
    /// Cache configuration
    cache_config: CacheConfig,
}

#[derive(Clone, Debug)]
pub struct CacheConfig {
    /// Maximum number of resources to cache
    pub max_resource_entries: u64,
    /// Maximum number of modules to cache
    pub max_module_entries: u64,
    /// Time-to-live for cache entries
    pub ttl_secs: u64,
    /// Whether caching is enabled
    pub enabled: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_resource_entries: 10_000,
            max_module_entries: 1_000,
            ttl_secs: 60,
            enabled: true,
        }
    }
}
```

#### 1.2 Cache Integration

```rust
impl Context {
    /// Get a resource with caching
    pub fn get_resource_cached(
        &self,
        address: AccountAddress,
        struct_tag: &StructTag,
        version: Version,
    ) -> Result<Option<Vec<u8>>> {
        if !self.cache_config.enabled {
            return self.get_state_value(
                &StateKey::resource(&address, struct_tag)?,
                version,
            );
        }
        
        let cache_key = ResourceCacheKey {
            address,
            struct_tag: struct_tag.clone(),
            version,
        };
        
        // Check cache first
        if let Some(cached) = self.resource_cache.get(&cache_key) {
            metrics::CACHE_HITS.with_label_values(&["resource"]).inc();
            return Ok(Some(cached.as_ref().clone()));
        }
        
        metrics::CACHE_MISSES.with_label_values(&["resource"]).inc();
        
        // Fetch from DB
        let result = self.get_state_value(
            &StateKey::resource(&address, struct_tag)?,
            version,
        )?;
        
        // Cache the result if found
        if let Some(ref bytes) = result {
            self.resource_cache.insert(cache_key, Arc::new(bytes.clone()));
        }
        
        Ok(result)
    }
}
```

#### 1.3 Configuration

**File**: `config/src/config/api_config.rs`

```rust
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct ApiConfig {
    // ... existing fields ...
    
    /// Configuration for response caching
    pub cache: CacheConfig,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct CacheConfig {
    pub enabled: bool,
    pub max_resource_entries: u64,
    pub max_module_entries: u64,
    pub ttl_secs: u64,
}
```

#### 1.4 Cache Invalidation Strategy

For version-keyed caches, invalidation is automatic since each version creates a new cache key. However, for memory efficiency:

```rust
impl Context {
    /// Prune old cache entries (called periodically)
    pub fn prune_old_cache_entries(&self, current_version: Version) {
        let min_version = current_version.saturating_sub(1000); // Keep last 1000 versions
        
        // Resource cache cleanup happens automatically via TTL and LRU eviction
        // But we can also manually evict very old entries
        self.resource_cache.invalidate_entries_if(move |key, _| {
            key.version < min_version
        });
    }
}
```

### Expected Impact

| Metric | Before | After (Estimated) |
|--------|--------|-------------------|
| Hot resource read latency | 1-5ms | <0.1ms |
| DB reads per request | 3-10 | 1-3 |
| Memory usage | Baseline | +50-100MB |

---

## 2. Batch Timestamp Lookups

### Problem Statement

In `render_transactions_sequential()`, each transaction requires a separate call to `get_block_timestamp()`:

```rust
// Current implementation
for t in data {
    let timestamp = self.db.get_block_timestamp(t.version)?;
    // ... process transaction
}
```

For a page of 100 transactions, this results in 100 separate DB calls.

### Proposed Solution

Batch the timestamp lookups into a single DB operation.

### Implementation Details

#### 2.1 Add Batch Method to DbReader

**File**: `storage/storage-interface/src/lib.rs`

```rust
pub trait DbReader: Send + Sync {
    // ... existing methods ...
    
    /// Batch fetch block timestamps for multiple versions
    fn get_block_timestamps_batch(
        &self,
        versions: &[Version],
    ) -> Result<HashMap<Version, u64>>;
}
```

#### 2.2 Implement in AptosDB

**File**: `storage/aptosdb/src/lib.rs`

```rust
impl DbReader for AptosDB {
    fn get_block_timestamps_batch(
        &self,
        versions: &[Version],
    ) -> Result<HashMap<Version, u64>> {
        if versions.is_empty() {
            return Ok(HashMap::new());
        }
        
        // Group versions by block to minimize lookups
        let mut result = HashMap::with_capacity(versions.len());
        let mut block_cache: HashMap<u64, (u64, u64, u64)> = HashMap::new(); // height -> (start, end, timestamp)
        
        for &version in versions {
            // Check if we already have this block info cached
            let timestamp = if let Some(cached) = block_cache.values()
                .find(|(start, end, _)| version >= *start && version <= *end)
            {
                cached.2
            } else {
                // Fetch block info
                let (start, end, block_event) = self.get_block_info_by_version(version)?;
                let timestamp = block_event.proposed_time();
                block_cache.insert(block_event.height(), (start, end, timestamp));
                timestamp
            };
            
            result.insert(version, timestamp);
        }
        
        Ok(result)
    }
}
```

#### 2.3 Update Context Methods

**File**: `api/src/context.rs`

```rust
impl Context {
    pub fn render_transactions_sequential<E: InternalError>(
        &self,
        ledger_info: &LedgerInfo,
        data: Vec<TransactionOnChainData>,
        initial_timestamp: u64,
    ) -> Result<Vec<aptos_api_types::Transaction>, E> {
        if data.is_empty() {
            return Ok(vec![]);
        }

        // Batch fetch all timestamps upfront
        let versions: Vec<Version> = data.iter().map(|t| t.version).collect();
        let timestamps = self.db
            .get_block_timestamps_batch(&versions)
            .context("Failed to batch fetch timestamps")
            .map_err(|err| {
                E::internal_with_code(err, AptosErrorCode::InternalError, ledger_info)
            })?;

        let state_view = self.latest_state_view_poem(ledger_info)?;
        let converter = state_view.as_converter(self.db.clone(), self.indexer_reader.clone());
        
        let txns: Vec<aptos_api_types::Transaction> = data
            .into_iter()
            .map(|t| {
                let timestamp = *timestamps.get(&t.version).unwrap_or(&initial_timestamp);
                let txn = converter.try_into_onchain_transaction(timestamp, t)?;
                Ok(txn)
            })
            .collect::<Result<_, anyhow::Error>>()
            .context("Failed to convert transaction data from storage")
            .map_err(|err| {
                E::internal_with_code(err, AptosErrorCode::InternalError, ledger_info)
            })?;

        Ok(txns)
    }
}
```

### Expected Impact

| Metric | Before | After |
|--------|--------|-------|
| DB calls for 100 txns | 100+ | 1-5 |
| Latency for /transactions | 50-200ms | 10-50ms |

---

## 3. State View Reuse

### Problem Statement

Many endpoint handlers create multiple state views for the same version:

```rust
// Pattern seen in multiple handlers
let state_view = self.context.latest_state_view_poem(&ledger_info)?;
// ... do some work ...
let state_view2 = self.context.latest_state_view_poem(&ledger_info)?; // Redundant!
```

### Proposed Solution

Create the state view once at the handler entry point and pass it through.

### Implementation Details

#### 3.1 Refactor Handler Pattern

**Before**:
```rust
fn get_transaction_inner(
    &self,
    accept_type: &AcceptType,
    transaction_data: TransactionData,
    ledger_info: &LedgerInfo,
) -> BasicResultWith404<Transaction> {
    match accept_type {
        AcceptType::Json => {
            let state_view = self.context.latest_state_view_poem(ledger_info)?;
            // ...
        }
        AcceptType::Bcs => {
            // ...
        }
    }
}
```

**After**:
```rust
fn get_transaction_inner(
    &self,
    accept_type: &AcceptType,
    transaction_data: TransactionData,
    ledger_info: &LedgerInfo,
    state_view: &DbStateView,  // Passed in
) -> BasicResultWith404<Transaction> {
    match accept_type {
        AcceptType::Json => {
            let converter = state_view.as_converter(
                self.context.db.clone(),
                self.context.indexer_reader.clone(),
            );
            // ...
        }
        AcceptType::Bcs => {
            // ...
        }
    }
}
```

#### 3.2 Create State View Bundle

```rust
/// Bundle of commonly needed request context
pub struct RequestContext<'a> {
    pub ledger_info: LedgerInfo,
    pub state_view: DbStateView,
    pub converter: MoveConverter<'a, DbStateView>,
}

impl Context {
    pub fn request_context(&self) -> Result<RequestContext<'_>, BasicError> {
        let ledger_info = self.get_latest_ledger_info()?;
        let state_view = self.latest_state_view_poem(&ledger_info)?;
        let converter = state_view.as_converter(
            self.db.clone(),
            self.indexer_reader.clone(),
        );
        
        Ok(RequestContext {
            ledger_info,
            state_view,
            converter,
        })
    }
}
```

### Expected Impact

| Metric | Before | After |
|--------|--------|-------|
| State view creations per request | 2-5 | 1 |
| Memory allocations | Higher | Lower |

---

## 4. Lazy Type Annotation

### Problem Statement

The `MoveConverter` always fetches type annotations from storage, even when:
- The client requests BCS format (no JSON needed)
- The response only needs raw bytes

### Proposed Solution

Add a lazy evaluation mode that defers type resolution.

### Implementation Details

#### 4.1 Add Lazy Resource Type

**File**: `api/types/src/move_types.rs`

```rust
/// A resource that may or may not have decoded data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MoveResourceLazy {
    #[serde(rename = "type")]
    pub typ: MoveStructTag,
    
    /// Raw BCS bytes (always present)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_bytes: Option<HexEncodedBytes>,
    
    /// Decoded data (only present if decode=true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}
```

#### 4.2 Add Decode Parameter

**File**: `api/src/state.rs`

```rust
#[oai(
    path = "/accounts/:address/resource/:resource_type",
    method = "get",
    operation_id = "get_account_resource",
    tag = "ApiTags::Accounts"
)]
async fn get_account_resource(
    &self,
    accept_type: AcceptType,
    address: Path<Address>,
    resource_type: Path<MoveStructTag>,
    ledger_version: Query<Option<U64>>,
    /// If false, skip type annotation and return raw bytes in JSON
    decode: Query<Option<bool>>,  // NEW PARAMETER
) -> BasicResultWith404<MoveResource> {
    let should_decode = decode.0.unwrap_or(true);
    
    // For BCS, never decode
    let should_decode = should_decode && accept_type == AcceptType::Json;
    
    // ...
}
```

#### 4.3 Skip Annotation for BCS

```rust
fn resource(
    &self,
    accept_type: &AcceptType,
    address: Address,
    resource_type: MoveStructTag,
    ledger_version: Option<u64>,
    decode: bool,
) -> BasicResultWith404<MoveResource> {
    // ... fetch bytes from storage ...

    match accept_type {
        AcceptType::Json if decode => {
            // Full type annotation (existing behavior)
            let resource = state_view
                .as_converter(self.context.db.clone(), self.context.indexer_reader.clone())
                .try_into_resource(&tag, &bytes)?;
            BasicResponse::try_from_json((resource, &ledger_info, BasicResponseStatus::Ok))
        },
        AcceptType::Json => {
            // Raw bytes in JSON wrapper (no type annotation)
            let resource = MoveResource::raw(&tag, bytes);
            BasicResponse::try_from_json((resource, &ledger_info, BasicResponseStatus::Ok))
        },
        AcceptType::Bcs => {
            // Direct bytes passthrough (existing behavior)
            BasicResponse::try_from_encoded((bytes.to_vec(), &ledger_info, BasicResponseStatus::Ok))
        },
    }
}
```

### Expected Impact

| Scenario | Before | After |
|----------|--------|-------|
| BCS resource request | Type annotation (wasted) | Skip annotation |
| JSON with decode=false | Type annotation | Skip annotation |
| Annotation time savings | 0 | 1-5ms per resource |

---

## 5. Additional Metrics

### Problem Statement

Current metrics don't provide visibility into:
- Cache performance
- DB operation timing
- Type conversion overhead
- Memory allocation patterns

### Proposed Solution

Add comprehensive metrics for performance monitoring.

### Implementation Details

#### 5.1 New Metrics

**File**: `api/src/metrics.rs`

```rust
// Cache metrics
pub static CACHE_HITS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_api_cache_hits",
        "Number of cache hits by cache type",
        &["cache_type"]  // "resource", "module", "gas_schedule"
    )
    .unwrap()
});

pub static CACHE_MISSES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_api_cache_misses",
        "Number of cache misses by cache type",
        &["cache_type"]
    )
    .unwrap()
});

pub static CACHE_SIZE: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_api_cache_size",
        "Current number of entries in cache by type",
        &["cache_type"]
    )
    .unwrap()
});

// DB operation metrics
pub static DB_OPERATION_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_api_db_operation_latency_seconds",
        "Latency of database operations",
        &["operation"],  // "get_transaction", "get_events", "get_resource", etc.
        vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]
    )
    .unwrap()
});

// Type conversion metrics
pub static TYPE_CONVERSION_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_api_type_conversion_latency_seconds",
        "Latency of Move type conversions",
        &["conversion_type"],  // "resource", "transaction", "event"
        vec![0.00001, 0.0001, 0.0005, 0.001, 0.005, 0.01]
    )
    .unwrap()
});

// Batch operation metrics
pub static BATCH_SIZE: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_api_batch_size",
        "Size of batch operations",
        &["operation"],
        vec![1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 500.0, 1000.0]
    )
    .unwrap()
});
```

#### 5.2 Instrumentation Helpers

```rust
/// Helper macro for timing operations
macro_rules! time_operation {
    ($metric:expr, $label:expr, $op:expr) => {{
        let start = std::time::Instant::now();
        let result = $op;
        $metric
            .with_label_values(&[$label])
            .observe(start.elapsed().as_secs_f64());
        result
    }};
}

// Usage example
let resource = time_operation!(
    metrics::DB_OPERATION_LATENCY,
    "get_resource",
    self.db.get_state_value(&state_key, version)?
);
```

---

## 6. Implementation Plan

### Week 1: Foundation

| Day | Task | Files |
|-----|------|-------|
| 1-2 | Add new metrics infrastructure | `api/src/metrics.rs` |
| 3-4 | Implement cache structures | `api/src/context.rs` |
| 5 | Add cache configuration | `config/src/config/api_config.rs` |

### Week 2: Core Optimizations

| Day | Task | Files |
|-----|------|-------|
| 1-2 | Implement batch timestamp lookups | `storage/storage-interface/src/lib.rs`, `storage/aptosdb/src/lib.rs` |
| 3 | Update `render_transactions_sequential` | `api/src/context.rs` |
| 4-5 | Implement state view reuse pattern | `api/src/transactions.rs`, `api/src/accounts.rs` |

### Week 3: Polish & Testing

| Day | Task | Files |
|-----|------|-------|
| 1-2 | Add lazy type annotation | `api/src/state.rs`, `api/types/src/move_types.rs` |
| 3-4 | Performance testing & benchmarks | `api/src/tests/` |
| 5 | Documentation & review | `api/doc/` |

### Testing Strategy

1. **Unit Tests**: Cache behavior, batch operations
2. **Integration Tests**: End-to-end API tests with caching enabled/disabled
3. **Load Tests**: Benchmark before/after with `wrk2`
4. **Monitoring**: Deploy to testnet with new metrics

### Rollout Plan

1. **Feature Flag**: All optimizations behind configuration flags
2. **Testnet First**: Deploy to testnet for 1 week
3. **Gradual Rollout**: Enable on 10% -> 50% -> 100% of mainnet nodes
4. **Monitoring**: Watch error rates, latency percentiles, cache hit rates

---

## Appendix: Configuration Example

```yaml
api:
  enabled: true
  address: "0.0.0.0:8080"
  
  # New cache configuration
  cache:
    enabled: true
    max_resource_entries: 10000
    max_module_entries: 1000
    ttl_secs: 60
  
  # Existing configuration
  max_transactions_page_size: 100
  max_events_page_size: 100
```
