# Why MonoMove scales worse than V1 under Block-STM: investigation log

A dated log of measurements, the bottlenecks they point at, and what fixes
them. Newest entries at the bottom. The tooling is described at the end.

The question: MonoMove's speedup over V1 shrinks as the execution thread count
grows. Per-transaction cost is lower on MonoMove, so a fixed amount of shared
overhead per transaction costs it a larger fraction. The sweep separates the
two explanations by comparing growth in microseconds per transaction between
the VMs, component by component:

- Same growth on both VMs: the cost is in shared code (Block-STM, mvhashmap,
  state view, block boundaries). MonoMove's ratio is worse only because its
  own per-transaction cost is smaller. The fix belongs in the shared code.
- More growth on MonoMove in the same component: the cause is specific to
  MonoMove's integration (its provider, arenas, heap evacuation, output
  conversion).

## 2026-11-06: starting point

Speedup of MonoMove over V1 on the inner block executor, from the calibrated
sequential job (`e2e_perf_speedup.tsv`) and the parallel job at 16 threads
(run 42217687781 on `benchmark-c3d-60`, blocks of 1000/500, 30 blocks):

| workload | 1 thread (sequential path) | 16 threads |
|---|---|---|
| no-op | 8.13x | 3.71x |
| apt-fa-transfer | 4.85x | 2.59x |
| account-generation | 6.00x | 2.91x |
| order-book-no-matches1-market | 8.70x | 6.20x |

The order book keeps most of its advantage, the three cheap workloads lose
more than half of theirs. That is the signature of a fixed per-transaction
cost that both VMs pay: the cheaper the transaction, the larger its share.

### Finding: the e2e harness chains five transactions per sender

`aptos-executor-benchmark` defaults to `--transactions-per-sender 5` and the
e2e harness does not override it. `run_workload` draws `block_size / 5`
senders per block and lays each sender's five transactions out consecutively,
so four out of five transactions depend on the one right before them through
the sender's sequence number. On a local smoke run that gives 0.4 to 1.4
speculative aborts per transaction on `no-op` at 2 and 4 threads.

Both VMs pay this, but it is a scheduling constraint, not VM work, so it
dilutes exactly the way a fixed cost does. The sweep runs with one transaction
per sender by default so that the measured growth is the VMs' and Block-STM's
own, and keeps the 5-per-sender shape as a separate variant to quantify.

### Runner for the sweep

`benchmark-c3d-60` has no `perf`, `perf_event_paranoid=2`, and `/proc/sys` is
read-only, so profiles cannot be taken there. The sweep runs on ephemeral
`runs-on` c7a instances (64 vCPU = 64 physical AMD Genoa cores, no SMT, same
microarchitecture as the c3d runner), one workload per machine, with `perf`
installed from `linux-tools`. Numbers from the two machines are not compared
against each other.

## 2026-11-06: first sweep (run 42243120066, c7a.16xlarge, 1 txn per sender)

100 blocks, 3 repeats, 2M-account DB, `release` build, split stages. Blocks
of 1000 (`no-op`) or 500 transactions. Full tables in the run's artifacts;
`mono_move/docs` keeps only what the conclusions rest on.

### Headline

| workload | inner speedup 2T | 16T | V1 scaling 2T→16T | MonoMove scaling 2T→16T |
|---|---|---|---|---|
| no-op | 4.10x | 3.61x | 6.11x | 5.38x |
| apt-fa-transfer | 3.59x | 3.08x | 6.20x | 5.31x |
| account-generation | 3.69x | 3.22x | 6.06x | 5.29x |
| order-book-no-matches1-market | 7.65x | 3.58x | 0.98x | 0.46x |

Ideal scaling is 8x. The three independent workloads lose 0.8x of speedup
between 2 and 16 threads. The order book is a different regime: every
transaction writes the same market resource, so both VMs are serialized
(5–6 speculative aborts per transaction at 16 threads), V1 stays flat and
MonoMove *slows down* 2.2x from 2 to 16 threads.

### Where the time goes, per transaction (apt-fa-transfer, µs/txn)

Worker CPU is summed over all workers; wall is per worker.

| | V1 2T | V1 16T | Mono 2T | Mono 16T |
|---|---|---|---|---|
| execution stage wall | 212.7 | 41.0 | 63.3 | 17.1 |
| inner block executor wall | 203.9 | 32.9 | 56.8 | 10.7 |
| worker scope wall | 201.8 | 31.1 | 56.0 | 9.9 |
| worker CPU total | 398.9 | 479.8 | 107.3 | 145.5 |
| worker CPU execute_vm | 338.5 | 400.2 | 77.4 | 102.2 |
| worker CPU materialize | 26.4 | 37.6 | 17.3 | 27.1 |
| worker CPU execute_apply | 19.5 | 24.4 | 8.4 | 10.2 |
| worker CPU commit_hooks + scheduler | 11.5 | 15.2 | 3.6 | 5.5 |
| cold DB reads (RocksDB), inclusive | 33.0 | 33.6 | 21.4 | 25.3 |

Decomposition of the loss at 16 threads (inner wall minus ideal 2T CPU / 16):

| | V1 | MonoMove |
|---|---|---|
| ideal (2T worker CPU / 16) | 24.9 | 6.7 |
| CPU inflation (worker CPU growth / 16) | +5.1 | +2.4 |
| per-block cost inside the parallel function but outside the worker scope | +1.8 | +0.8 |
| per-block worker start/drain (scope wall × 16 − tracked worker wall) | +1.1 | +0.8 |
| measured inner wall | 32.9 | 10.7 |
| execution stage outside the inner executor (constant per txn) | +8.1 | +6.4 |
| measured execution stage wall | 41.0 | 17.1 |

The three findings below follow from this table.

### Finding 1: the stage outside Block-STM is the largest fixed cost, and it is shared

`execution stage − inner block executor` is 6.4 µs/txn on MonoMove and 8.1
on V1 for transfers (3.5 / 5.5 on no-op) at every thread count. It is the
executor's per-block work around `BlockExecutor::execute_block`: the module
cache manager's `try_lock` (environment check, ~0.5 ms per block), reading
the memorized state view cache back, `LedgerState::update`, output
conversion, the MVHashMap drop. It runs on the `txn_executor` thread and the
rayon pool between blocks, so no worker overlaps it. At 16 threads it is 37%
of MonoMove's execution stage and 20% of V1's. Same absolute cost on both
VMs: shared code, MonoMove only pays a larger share.

Fix: overlap block N+1's execution with block N's post-processing, or shrink
the serial parts. This is the largest lever on the execution-stage speedup
ratio at 16 threads (removing it: 2.40x → 3.07x on transfers).

### Finding 2: worker CPU inflates 1.2x (V1) and 1.35x (MonoMove) from 2 to 16 threads

Both VMs burn more CPU per transaction with more workers, and workers are
97% (V1) and 90% (MonoMove) busy at 16 threads, so the loss is inflation,
not idling. The growth is spread over many symbols. Ranked by symbol on
MonoMove (transfer, 16T − 2T, µs/txn): TLB-shootdown IPIs
(`asm_sysvec_call_function`) +6.4, perf's own context-switch accounting
+2.5 (artifact), SipHash +1.7, mvhashmap DashMap shards +2.5,
`VersionedValue::read` and read-dependency registration +1.2,
`LoadedModule::get_instantiated_function_ptr` +0.7, RocksDB +1.5. V1 has
the same shared items plus its own: `TypeVecInterner::intern_vec` +6.6,
`StructNameIndexMap::idx_to_struct_name` +3.3, `TwoKeyRegistry` +3.8.

The IPIs are the one item with a MonoMove-specific cause. Workers spend
1.76 µs/txn (V1: 0.40) in `_rjem_je_ecache_alloc → extent_recycle →
ehooks_default_zero_impl`: jemalloc zeroes a recycled large extent with
`madvise(MADV_DONTNEED)`, which flushes the TLB on every core the process
has run on. MonoMove allocates with `alloc_zeroed` per transaction (the
evacuation to-space, sized to the session's high-water mark) and per worker
per block (4 MiB arena segments). Every other thread then takes the IPI
(6.5 µs/txn at 16T) and re-faults the pages.

Fix (applied in this branch): both allocations use uninitialized memory.
`heap_alloc` zeroes every object it hands out and evacuation writes every
byte it reads, so the zeroing was redundant.

### Finding 3: MonoMove never promotes reads to the hot state

`MonoTxnOutput::storage_keys_read` and `storage_keys_written` were empty
(`TODO(completeness)`), so the block epilogue never made MonoMove's reads
hot. A key read but never written stays cold forever and goes to RocksDB in
every block. Measured on `no-op`: 1.10 cold DB reads per transaction on
MonoMove against 0.21 on V1, costing 15.9 µs/txn of worker CPU at 2T (23%
of MonoMove's worker time) against 1.9 on V1. Written keys are found in
the in-memory state delta on both VMs, so the repeated cold key has to be
one that is only read; the likely candidate is a vacant slot the
prologue/epilogue checks, such as `CoinStore<AptosCoin>` at the sender,
which V1 promotes as `HotVacant` after the first touch.

Fix (applied in this branch): report read and written keys from the output,
mapping group members to their group's slot.

### Finding 4 (order book): `instantiated_functions` mutex on the critical path

Serialized workload, 16 threads. MonoMove workers spend 1149 µs/txn under
futex or parking_lot: `SchedulerV2::next_task` 441 (shared, V1 has 3996),
the `WorkerPool` end-of-block barrier 233 (shared), and
`LoadedModule::get_instantiated_function_ptr` 461. The last is MonoMove's:
every call of a generic function takes a `parking_lot::Mutex` over the
module's instantiation map and clones an `Arc`. Sixteen workers re-executing
the same order-book code serialize on it, including the one worker whose
execution is the critical path. That is why MonoMove's wall per transaction
doubles from 8 to 16 threads while V1's does not move.

Fix (applied in this branch): the map is an `ArcSwap` snapshot; reads take
no lock and clone no `Arc`, inserts copy-on-write (once per instantiation
per module lifetime).

### Not a bottleneck

- Dependency waits: zero on all four workloads.
- Speculative aborts on the independent workloads: MonoMove 0.03–0.04 per
  txn at 16T (V1: 0), from `eq_value` always returning false. A few percent
  of re-execution, not the gap.
- Memory: peak RSS flat with thread count on both VMs; minor faults per
  transaction flat on the independent workloads.
- Rayon-pool CPU (`State::update`, hashing): ~40 µs/txn on both VMs,
  independent of thread count, parallel on 64 cores.

## Tooling

- `aptos-block-executor::worker_phases`: each Block-STM worker charges its
  time to a phase (`commit_hooks`, `scheduler`, `execute_vm`, `execute_apply`,
  `materialize`, `module_validation`) on both the wall clock and the thread CPU
  clock. The difference is time the worker was descheduled. Flushed to
  `aptos_execution_worker_phase_micros` once per worker per block.
- `aptos-executor-benchmark` prints, per transaction over the execution stage:
  the phase totals, process user/system CPU, minor/major faults, voluntary and
  involuntary context switches, peak RSS, and CPU by thread-name group. Also a
  `CLOCK_MONOTONIC` window so a `perf` recording can be cut to the stage.
- `third_party/move/mono-move/testsuite/e2e-perf/scaling_sweep.py` replays
  recorded blocks on both VMs across thread counts and reports growth per
  component. With `PROFILE=perf` it records selected replays and writes folded
  stacks (`profiles/*.folded.gz`) restricted to the execution stage.
- `third_party/move/mono-move/testsuite/e2e-perf/analyze_profile.py`
  attributes folded stacks to components (crate-level) and to inclusive frames
  (`execute_v2`, `materialize_txn_commit`, mvhashmap reads, allocator, ...).
- `.github/workflows/mono-move-scaling-sweep.yaml`: one job per workload on
  ephemeral runners. Dispatch with
  `gh workflow run mono-move-scaling-sweep.yaml --ref <branch> -f ...`.
