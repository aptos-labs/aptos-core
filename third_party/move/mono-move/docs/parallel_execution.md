# Parallel execution: MonoMove under Block-STM

A running log of what has been measured, what the bottlenecks are, and what is
still open. Design rationale for the pieces themselves lives in the other docs
in this directory.

## What is integrated

MonoMove runs on Block-STM's parallel path. The pieces that make that work, all
of which are load-bearing rather than optimizations:

- **Versioned reads.** `Version` is `Option<(u32, u32)>`: `None` for a pre-block
  storage value, `Some((txn_idx, incarnation))` for another transaction's write.
  The read set records it so validation can compare.
- **Speculative aborts.** `ResourceProviderError::SpeculativeAbort` lets a read
  fail because the block halted or a dependency is unresolved, distinct from an
  invariant violation.
- **Resource groups.** `InMemoryStorageKey::ResourceGroup` gives a group its own
  addressable slot; members keep their `Resource` keys and are tags within it.
  `VersionedGroupData::group_members_at` reads a group as of a given index.
- **Read-write sets on every outcome.** `TxnOutcome::Discarded` and
  `ExecutedNoEffects` carry `Option<SessionEffects>`. A transaction that opened a
  session has reads to validate even when its writes are thrown away.
- **`SegmentedArena`.** Base values deserialized from storage outlive the
  transaction that read them, so they go in a block-lifetime arena rather than
  the session heap. It grows by segments, and each allocation returns the
  segment to pin it by. Growth is required for correctness, not speed: whether a
  value fits must not depend on how much the arena already held, or two
  Block-STM schedules could disagree on whether the same read succeeds.
- **Session-root evacuation.** `finish()` copies what the read-write set and the
  extensions still reach into a heap sized to the session's own high-water mark,
  and the block pins that instead of the whole session heap. Without it every
  writing transaction pins megabytes for the rest of the block, which does not
  survive realistic block sizes. `FrozenHeap` holds a `Vec<Heap>` because a
  partial copy leaves the read-write set pointing into both.
- **Per-worker arenas.** `GlobalContext::num_execution_workers()` caps
  `concurrency_level`, since a worker beyond that count has no arena to lock.

## What was reverted

Session buffer pooling: thread-local pools of heap buffers and stacks, with
`Heap::new_session` / `Heap::release`, `take_pooled_*` / `return_pooled_*`, and
a stack high-water mark so a returned stack only had to be zeroed as far as it
was used. `DEFAULT_HEAP_SIZE` had also been dropped from 10 MiB to 4 MiB on the
grounds that pooling made the size free.

It worked, roughly +50% at one worker and +27% at eight, because a
megabyte-scale buffer is past every allocator size class, so each transaction
paid an `mmap` and an `munmap` under the process-wide address-space lock. It is
still a pure optimization and none of the integration depends on it. Removed to
keep the integration reviewable; worth reconsidering once the integration lands.

## Measurements

Apple M4 Max, 10 performance + 4 efficiency cores. `aptos-executor-benchmark`
replaying 200 identical recorded blocks of 1000 `apt-fa-transfer`,
`--split-stages`, one transaction per sender, MonoMove on Block-STM v2.

Two caveats on every number below. The runs were made under `samply` with a
competing `rustc`, which costs about 30%: the 8-worker run measured 88k against
129k on a quiet machine. Both runs were back to back, so the 2-vs-8 comparison
holds even though the absolute figures are low. Lock contention is the line most
inflated by this.

### `--execution-threads 1` is not Block-STM

`executor.rs:2332` dispatches to `execute_transactions_sequential` when
`concurrency_level == 1`. Any "1 to 8 threads" figure compares two different
algorithms. The parallel path starts at 2 workers, and scaling has to be
measured from there.

### Whole stage, 2 to 8 workers

| | 2 workers | 8 workers | |
|---|---|---|---|
| execution stage | 31,864 t/s | 65,644 t/s | 2.06x |
| inner block executor | 35,760 t/s | 88,071 t/s | 2.46x |
| worker CPU per txn | 62.77 µs | 121.87 µs | 1.94x |

Speculative aborts are 0.011 per transaction. Conflicts are not a factor on
transfers, as expected.

### In-block only

Windowing to the milliseconds where at least one worker is inside
`execute_transactions_parallel_v2`, so the inter-block gaps are excluded:

| | 2 workers | 8 workers | |
|---|---|---|---|
| in-block wall | 5475 ms | 2156 ms | **2.54x** |
| in-block throughput | 36,530 t/s | 92,764 t/s | 2.54x |
| worker CPU per txn | 54.75 µs | 86.24 µs | 1.58x |

Wall clock and CPU accounting reconcile exactly: 4 / 1.58 = 2.54. Nothing is
unexplained.

Two things are not happening inside a block. Workers never wait for work
mid-block: `wait_for_dependency` does not appear in a single sample, and the
only park is block-edge spill. And there is no ramp-up or drain problem. The
entire loss is workers burning more CPU for the same work.

### Where the in-block CPU goes

Worker microseconds per transaction, in-block window.

| category | 2w | 8w | delta |
|---|---|---|---|
| lock contention | 1.20 | 10.13 | +8.94 |
| allocator (jemalloc) | 7.45 | 11.21 | +3.76 |
| mono runtime (non-interpreter) | 4.79 | 7.90 | +3.11 |
| block-edge park spill | 0.01 | 2.66 | +2.65 |
| hashmap probe / resize | 2.31 | 4.20 | +1.89 |
| siphash | 4.24 | 6.02 | +1.78 |
| mono block-stm glue | 1.95 | 3.17 | +1.23 |
| dashmap | 1.83 | 2.96 | +1.12 |
| metrics clock reads | 3.76 | 4.81 | +1.06 |
| mvhashmap | 0.87 | 1.90 | +1.03 |
| fx / other hashers | 1.68 | 2.70 | +1.01 |
| mono interpreter | 3.10 | 3.98 | +0.88 |
| mono value serde / layout | 1.65 | 2.48 | +0.83 |
| legacy move vm | 1.25 | 2.02 | +0.77 |
| mono loader | 0.85 | 1.58 | +0.74 |
| state view / state store | 1.56 | 2.25 | +0.69 |
| sha3 / keccak | 3.15 | 3.78 | +0.64 |
| bcs / serde | 2.99 | 3.52 | +0.53 |
| block-stm scheduler | 0.61 | 1.08 | +0.48 |
| aptos-vm / types | 0.67 | 0.77 | +0.10 |
| rocksdb | 8.83 | 7.05 | −1.79 |
| **total** | **54.75** | **86.24** | **+31.49** |

Grouped: contention +6.2 excluding the edge spill, hashing and hash-map work
+6.8, allocator +3.8, everything else +14.7.

### Block-STM phases, in-block

| phase | 2w | 8w | growth |
|---|---|---|---|
| `execute_v2` | 45.55 | 62.34 | 1.37x |
| `materialize_txn_commit` | 7.07 | 12.06 | 1.71x |
| worker task entry + `execute_transactions_parallel_v2` self | 0.49 | 5.07 | 10x |
| `validate` | 0.98 | 3.05 | 3.1x |
| everything else | 0.63 | 0.92 | 1.5x |

Validation is 3.5% of worker time at 8 workers. It triples, but it is not the
bottleneck and never was.

## Bottlenecks

### 1. Block boundaries

One gap per block, measured by 1 ms bucketing: 200 gaps at 2 workers averaging
3.88 ms, 196 at 8 workers averaging 4.17 ms. The length does not shrink with
worker count, so its share doubles as the parallel part gets faster: 12.3% of
the stage at 2 workers, 26.8% at 8. Removing it entirely at 8 workers takes the
stage from 3047 to 2230 ms, 65.6k to 89.7k t/s.

What runs in the gap, in microseconds per transaction amortized over the stage:

| work | µs/txn |
|---|---|
| `State::update` → `layered_map::new_layer_impl::SubTreeBuilder` | 5.9 |
| crossbeam-epoch `try_advance` | 3.5 |
| dropping the previous block's mvhashmap | 2.6 |
| `StateSlot::clone` + `HotStateLRU::get_slot` | 1.5 |
| BCS transaction size, gas schedule rebuild, signature bookkeeping | 3.3 |

By thread: `rayon-global` 12.7, `default_conc_dropper` 3.6, `txn_executor` 1.7,
`background` 1.6. None of it is inherently serial. It is already parallel, just
on a different pool. `WorkerPool::scope` is called once per block and all
workers park on `receiver.recv()` for the whole gap, so the two pools never
overlap.

### 2. Hashing, about 6.8 µs/txn at 8 workers

The mvhashmap DashMaps are built with `DashMap::new()`, so `RandomState` and
SipHash, keyed by `InMemoryStorageKey` and `(InMemoryStorageKey, StructTag)`.
6.02 µs/txn is pure SipHash and 4.20 more is probing. `versioned_data.rs:239`
and `versioned_group_data.rs:139-140`.

### 3. Lock contention, about 6.0 µs/txn at 8 workers

By owning site, in-block:

| site | 2w | 8w |
|---|---|---|
| `VersionedValue<MonoValue>::read` dependency mutex | 0.13 | 2.05 |
| mvhashmap DashMap shard locks | 0.21 | 2.01 |
| `SchedulerV2::next_task` | 0.03 | 1.01 |
| `TxnLastInputOutput` | 0.31 | 0.46 |
| `LoadedModule::get_instantiated_function` | 0.00 | 0.18 |

The first is the per-entry `Mutex<RegisteredReadDependencies>` at
`versioned_data.rs:49`. Every transfer reads the same block-global keys — APT FA
metadata, supply, chain id, features, gas schedule — so all workers serialize on
one mutex per key, inserting into a `BTreeMap` that grows to block size.

Registration cannot simply be skipped for storage reads. `read()` at
`versioned_data.rs:219-224` registers on whatever entry it resolved, including
the base entry at index 0, and that is required:
`split_off_affected_read_dependencies` at `:118-130` takes the entry at
`range(..=txn_idx).next_back()`, which for a key whose first block write is txn
*j* is the base entry. Readers above *j* are recorded nowhere else. v2 has no
re-validation sweep to catch a miss either — `Self::validate` runs from
`executor.rs:1172` (the v1 loop) and `executor.rs:1018` (a paranoid post-commit
check that returns `code_invariant_error`). v1 registers nothing because it
invalidates through estimate markers and validation waves instead.

`set_base_value` at `versioned_data.rs:360` also does `entry(key).or_default()`,
taking the shard write lock even when the entry already exists.

### 4. Baseline inflation, about 1.25x

Categories that share nothing still cost more per transaction at 8 workers:
interpreter 1.28x, sha3 1.20x, bcs 1.18x, value serde 1.50x, legacy vm 1.62x. No
lock and no shared data structure. Causes are core heterogeneity (8 workers plus
14 rayon plus 14 `non_exe` on 10 P and 4 E cores), shared last-level cache and
memory bandwidth, and some measurement overhead. If everything grew at 1.25x,
in-block CPU would be 68.4 µs rather than 86.2. The remaining 17.8 is the part
that is genuinely about sharing.

## Open

Nothing here is done.

- **Overlap block N+1 with block N's post-processing.** The largest single win,
  worth about 1.37x. The dependency is narrow: block N+1 only needs block N's
  writes, not the finished layered map. Hand the write set to the next block's
  base view as a pending overlay and let `State::update` finish underneath. The
  work is in the state view accepting an overlay that is not yet folded in.
- **Keep the worker pool resident across blocks.** Contained. Removes the
  `Barrier` sync, 3.67 µs/txn at 8 workers, and the wake-up round trip. Does not
  remove the gap itself.
- **Split the boundary cost into fixed and variable.** Run the same sweep at
  2000 and 4000 transactions per block. Costs nothing and says whether the
  ceiling is 1.37x or better.
- **Change the mvhashmap hasher.** One line per map, about 6.8 µs/txn.
- **Rework read-dependency registration.** Shard the mutex by `txn_idx % N` as a
  first step; `insert` touches one shard, `split_off` touches all N but only on
  a write. A dense per-block `Box<[AtomicU32]>` with a summary bitmap removes
  the lock entirely, at the cost of memory, so it would have to be allocated
  lazily once an entry crosses a reader threshold.
- **Read lock on the occupied path in `set_base_value`.**
- **Decompose `materialize_txn_commit`.** 12.06 µs/txn at 8 workers, second
  largest phase, grows at 1.71x. Not contention. Not yet broken down.
- **Allocator, 11.21 µs/txn.** Grows at 1.50x. Session buffer pooling was the
  answer and was reverted; revisit after the integration lands.
- **`eq_value` returns `false` unconditionally** (`mono_move/mod.rs:91-95`). That
  defeats the `still_valid` shortcut at `versioned_data.rs:132-136`, so any write
  to a hot key invalidates every registered reader even when the value is
  unchanged. Costs aborts rather than mutex time.
- **`get_read_summary` is empty** (`mono_move/reads.rs:229-232`).
- **`LoadedModule::instantiated_functions`** takes a `Mutex` and clones an
  `Arc<[LoadedModuleSlot]>` on every call. The specializer only emits
  `CallIndirect`; the inline cache is a `TODO(perf)` at `interpreter.rs:1322`.
  Small today, grows with worker count and call density.

Fixing the hasher and the contention would take in-block CPU to roughly 73 µs,
about 3.0x instead of 2.54x. Past that needs `materialize_txn_commit` and the
allocator, and then the 1.25x baseline caps this machine near 3.2x.

## Method

Profiles were taken with `samply record --save-only`, which does not
symbolicate: `nativeSymbols` is empty and `frameTable.address` is
library-relative. Resolve against a demangled `nm -n --defined-only` table
piped through `rustfilt`, with an image base of `0x100000000`. There is no
shared string table; each thread carries its own `stringArray`.

The Block-STM workers are the `par_exec-*` threads, not `rayon-global-*`.

samply's effective sampling rate varies with the number of live threads, around
1.0 to 1.2 ms per sample with 700+ threads in the process, so raw sample counts
are not comparable across runs. Calibrate per run: window to the measured
execution stage using the benchmark's own log timestamps mapped through
`meta.startTime`, then `µs_per_sample = workers × stage_seconds × 1e6 / samples`.
