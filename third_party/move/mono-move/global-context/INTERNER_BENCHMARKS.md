# Concurrent Interner Benchmarking Infrastructure

This document describes the comprehensive benchmarking infrastructure for evaluating different concurrent interner implementations.

## Overview

We've implemented **7 different concurrent interner strategies** that systematically vary two key dimensions:

1. **Map implementation**: RwLock (BTreeMap/HashMap) vs DashMap
2. **Arena allocation**: Coupled, Decoupled, Mutex, Sharded, Per-thread, Chunked

## Implementations

### 1. RwLock<BTreeMap + Arena> (`rwlock_btree`)
**Baseline**: Everything under single RwLock, deterministic ordering.

- Characteristics:
  - Simple, proven pattern
  - Deterministic iteration (useful for debugging)
  - O(log n) lookup
  - Write lock blocks ALL reads and allocations
  - Poor scalability

### 2. RwLock<HashMap + Arena> (`rwlock_hashmap`)
**Baseline variant**: HashMap for O(1) lookup vs BTreeMap's O(log n).

- Characteristics:
  - Simple, proven pattern
  - O(1) lookup vs O(log n) for BTreeMap
  - Write lock blocks ALL reads and allocations
  - Non-deterministic iteration
  - Poor scalability

### 3. RwLock<HashMap> + Mutex<Arena> (`rwlock_decoupled`)
**Decoupled**: Map and arena have independent locks.

- Characteristics:
  - Map operations don't block allocations
  - Allocations don't block reads (mostly)
  - Still have write lock on map
  - Can leak allocations on lost races (acceptable tradeoff)

### 4. DashMap + Mutex<Arena> (`dashmap_mutex`)
**Lock-free reads** with single-mutex allocations.

- Characteristics:
  - Completely lock-free reads
  - DashMap has 64 internal segments (fine-grained write locks)
  - Arena mutex is bottleneck on writes
  - Leaks allocations on lost races

### 5. DashMap + Sharded Arena (`dashmap_sharded`)
**Lock-free reads** with sharded allocations (64 arenas).

- Characteristics:
  - Lock-free reads
  - 64 independent arenas (minimal contention)
  - Scales well to 64+ cores
  - 64× memory overhead (each arena allocates independently)

### 6. DashMap + Per-thread Array (`dashmap_perthread_array`)
**Advanced**: Zero-contention allocations with explicit thread indices.

- Characteristics:
  - Lock-free reads
  - **Zero contention** (each thread locks only its own arena)
  - Explicit index assignment (no dynamic HashMap lookups)
  - **Best write performance** (direct array indexing ~2-3ns overhead)
  - Better cache locality (array-based)
  - Requires stable thread pool (threads don't exit during interner lifetime)
  - Must set index at thread startup (one-time cost per thread)

### 7. DashMap + Chunked Arena (`dashmap_chunked`)
**Advanced**: Pre-allocated chunks with atomic index allocation.

- Characteristics:
  - Lock-free reads
  - Truly concurrent writes (atomic index)
  - Only locks on chunk exhaustion (rare)
  - Complex implementation
  - Wasted memory (pre-allocated slots)
  - Chunk swap contention when buffer fills

## Benchmark Suite

We've implemented 5 comprehensive benchmark scenarios:

### 1. Read Throughput Benchmark
Measures lock-free vs locked reads under 100% cache hit.
- Pre-populates with 10k strings
- Tests pure lookup performance
- Scales from 1 to 16 cores

### 2. Write Throughput Benchmark
Measures arena allocation performance under 100% cache miss.
- Each iteration allocates unique strings
- Tests allocation contention
- Scales from 1 to 16 cores

### 3. Mixed Workload Benchmark
Measures interaction effects with varying read/write ratios.
- Tests at 50%, 75%, 90%, 95%, and 99% read ratios
- Critical for understanding lock contention effects
- Simulates realistic workloads

### 4. Warmup Performance Benchmark
Measures cold-start to steady-state transition.
- Phase 1: Cold start (100 txns, 70% writes)
- Phase 2: Steady state (900 txns, 99% reads)
- Simulates realistic blockchain workload

### 5. Latency Distribution Benchmark
Measures tail latencies (P50, P90, P99, P99.9, P99.99).
- Separate read and write latency measurements
- Critical for P99.9 requirements
- Uses flat sampling mode for accurate tail latencies

## Running Benchmarks

### Prerequisites
```bash
# Recommended: Disable CPU frequency scaling for stable results
sudo cpupower frequency-set -g performance

# Optional: Disable turbo boost for consistency
echo 1 | sudo tee /sys/devices/system/cpu/intel_pstate/no_turbo
```

### Run All Benchmarks
```bash
cd third_party/move/mono-move/global-context
cargo bench --bench interner_bench
```

### Run Specific Benchmark
```bash
# Read throughput only
cargo bench --bench interner_bench -- read_throughput

# Mixed workload at 90% reads
cargo bench --bench interner_bench -- mixed_workload/90

# Specific implementation and core count
cargo bench --bench interner_bench -- dashmap_perthread_array_8
```

### View Results
```bash
# Open HTML report
open target/criterion/report/index.html
```

## Expected Results

Based on the design characteristics, we expect:

### Read Performance
- **DashMap variants** should be 3-5× faster than RwLock variants
- **DashMap overhead**: 15-30ns vs 6-11ns for plain HashMap
- Arena strategy should **not** affect read performance

### Write Performance
- **Per-thread (array-indexed)** > Chunked > Sharded >> Mutex > RwLock
- Sharding benefits should appear at **>8 cores**
- Per-thread overhead: ~2-3ns per operation

### Mixed Workload (90% read / 10% write)
- **Per-thread** or **Chunked** likely winners
- DashMap should isolate reads from writes
- RwLock will show significant read slowdown during writes

### Mixed Workload (99% read / 1% write)
- All DashMap variants should perform well
- Differences mainly in write-path overhead

### Memory Usage
- **Sharded**: 64× initial allocation overhead
- **Per-thread**: One arena per OS thread
- **Chunked**: <10% waste with exponential growth

### Latency
- **P99.9 latency**: Lock-free should be 10-100× better
- **Worst-case under contention**: RwLock has long tail

## Performance Targets

The winning implementation should achieve:
- **Read**: >200M ops/sec at 32 cores
- **Write**: >10M ops/sec at 32 cores
- **Mixed (90% read)**: >150M ops/sec at 32 cores
- **P99.9 latency**: <50μs
- **Memory**: <100 bytes per entry

## Implementation Details

### Thread-Local Index Setup

For the per-thread array interner, threads must set their indices at startup:

```rust
use global_context::interner_impls::dashmap_perthread_array::{
    DashMapPerThreadArrayInterner,
    set_thread_index,
};

let thread_count = num_cpus::get();
let interner = Arc::new(DashMapPerThreadArrayInterner::new(thread_count));

std::thread::scope(|s| {
    let handles: Vec<_> = (0..thread_count)
        .map(|worker_idx| {
            let interner = Arc::clone(&interner);
            s.spawn(move || {
                // Set thread index once at startup
                set_thread_index(worker_idx);

                // Now use the interner
                let ptr = interner.intern(&value);
            })
        })
        .collect();
});
```

### Arena Memory Management

All implementations use a custom arena allocator that:
- Uses exponential growth (256, 512, 1024, 2048, ... elements)
- Never deallocates individual items (stable pointers)
- Maintains a pool of previous buffers
- Provides `flush()` for inter-block cleanup

### Safety

All interners use `unsafe` code for:
- Arena allocation (stable pointer generation)
- `Send`/`Sync` implementations (synchronized through locks/atomics)

Safety invariants:
- Pointers are stable (never invalidated until flush)
- Access is properly synchronized
- T must be `Send + Sync + 'static`

## Next Steps

1. **Run full benchmark suite** across 1-64 cores
2. **Analyze results** to identify optimal implementation
3. **Profile winner** with perf/flamegraph
4. **Integrate winner** into MonoMove GlobalContext
5. **End-to-end testing** with real workloads

## File Structure

```
third_party/move/mono-move/global-context/
├── src/
│   ├── lib.rs
│   ├── context.rs
│   └── interner_impls/
│       ├── mod.rs                      # Trait definition
│       ├── arena.rs                    # Shared arena allocator
│       ├── rwlock_btree.rs             # Implementation 1
│       ├── rwlock_hashmap.rs           # Implementation 2
│       ├── rwlock_decoupled.rs         # Implementation 3
│       ├── dashmap_mutex.rs            # Implementation 4
│       ├── dashmap_sharded.rs          # Implementation 5
│       ├── dashmap_perthread_array.rs  # Implementation 6
│       └── dashmap_chunked.rs          # Implementation 7
├── benches/
│   ├── interner_bench.rs               # Main benchmark suite
│   └── helpers.rs                      # Benchmark utilities
├── Cargo.toml
└── INTERNER_BENCHMARKS.md              # This file
```

## References

- [DashMap](https://docs.rs/dashmap) - Concurrent HashMap implementation
- [Parking Lot](https://docs.rs/parking_lot) - Fast synchronization primitives
- [Arc-Swap](https://docs.rs/arc-swap) - Lock-free Arc swapping
- [Criterion](https://docs.rs/criterion) - Statistical benchmarking

## Contributing

When adding new interner implementations:
1. Implement the `InternerImpl` trait
2. Add comprehensive unit tests
3. Add to benchmark helper's `InternerType` enum
4. Document characteristics and trade-offs
5. Run full benchmark suite
