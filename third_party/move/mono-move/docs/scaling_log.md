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

The sweep runs with one transaction per sender by default so that the
measured growth is the VMs' and Block-STM's own, and keeps the 5-per-sender
shape as a separate variant. (Quantified below: it costs both VMs about a
quarter of their 16-thread throughput and leaves the ratio alone.)

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
byte it reads, so the zeroing was redundant. (Measured in the next entry:
under 1%, the remaining flushes are jemalloc's own.)

### Finding 3: MonoMove never promotes reads to the hot state

`MonoTxnOutput::storage_keys_read` and `storage_keys_written` were empty
(`TODO(completeness)`), so the block epilogue never made MonoMove's reads
hot. A key read but never written stays cold forever and goes to RocksDB in
every block. Measured on `no-op`: 1.10 cold DB reads per transaction on
MonoMove against 0.21 on V1, costing 15.9 µs/txn of worker CPU at 2T (23%
of MonoMove's worker time) against 1.9 on V1. Written keys are found in
the in-memory state delta on both VMs, so the repeated cold key has to be
one that is only read. (Identified in the next entry: the sender's primary
store group, which unmetered MonoMove reads but never writes. Promotion
would not have helped either: the benchmark's block epilogues carry no
promotions for either VM.)

Fix (applied in this branch, for production parity): report read and
written keys from the output, mapping group members to their group's slot,
and carry the epilogue payload's promotions in its output.

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

## 2026-11-06: A/B/C runs on the fixes (runs 42273129737 / 42272739772 / 42273314169)

Same base commit, same machine type, same settings as the first sweep. A =
baseline, B = the three MonoMove fixes, C = the fixes with MonoMove metered.

### Independent workloads: the fixes change nothing

| apt-fa-transfer, µs/txn | A 2T | B 2T | A 16T | B 16T |
|---|---|---|---|---|
| V1 inner wall | 203.5 | 204.6 | 32.9 | 32.8 |
| MonoMove inner wall | 55.7 | 56.2 | 10.7 | 10.7 |
| MonoMove worker CPU | 105.1 | 106.1 | 144.9 | 144.1 |

Within noise on every metric. In the profile the `madvise` zeroing hook fell
from 0.60 to 0.32 µs/txn of sender time and the IPI receive load roughly
halved, but the remaining TLB flushes come from jemalloc's own metadata and
purge paths, which V1 shares, and none of it is large enough to move
throughput. Finding 2's "MonoMove-specific cause" for the IPIs was real but
worth under 1%.

The hot state fix cannot show here: the benchmark disables
`CALCULATE_TRANSACTION_FEE_FOR_DISTRIBUTION`, so block epilogues are V0 and
carry no promotions for either VM. It stays as a parity fix for production.

### Order book: the instantiation lock was the critical path

| order book, MonoMove | 2T | 8T | 16T |
|---|---|---|---|
| inner wall A, µs/txn | 153 | 165 | 299 |
| inner wall B, µs/txn | 138 | 130 | 141 |
| inner TPS A | 6,533 | 6,071 | 3,348 |
| inner TPS B | 7,238 | 7,671 | 7,093 |
| V1 inner TPS (A ≈ B) | 845 | 853 | 848 |

MonoMove no longer slows down with more workers and gains 14% even at 2
threads, where the mutex was uncontended but the lock and the `Arc` clone
on every generic call still cost. Its 16-thread speedup over V1 goes from
3.9x to 8.4x, back to its sequential ratio. V1 is flat across thread counts
on this workload, before and after, as it should be for a fully serialized
block.

### Metered MonoMove (run C): what the unmetered comparison hides

| no-op, MonoMove | unmetered 2T | metered 2T | unmetered 16T | metered 16T |
|---|---|---|---|---|
| cold DB reads /txn | 1.02 | 0.05 | 1.02 | 0.06 |
| worker CPU materialize | 5.5 | 21.6 | 9.7 | 32.9 |
| output bytes /txn | 353 | 1431 | 353 | 1431 |
| speculative aborts /txn | 0.00 | 0.59 | 0.03 | 4.24 |
| inner wall | 33.7 | 52.8 | 6.3 | 54.3 |

Two things fall out. The no-op cold read in Finding 3 is an artifact of
running unmetered: with a zero fee the sender's store is never written, so
it is only read, and a key that is only read is never in the in-memory
delta. Charging a fee writes it and the cold reads vanish. And metered
MonoMove cannot scale at all here because every transaction writes the APT
supply, which V1 keeps conflict-free through an aggregator MonoMove does not
support (the known gap, out of scope).

For the e2e comparison this means the unmetered MonoMove numbers understate
the shared per-output costs it would pay in production (materialization 4x,
`State::update` and hashing proportional to bytes) and overstate its read
cost. Both VMs should be metered before the speedup ratios are quoted as
production numbers.

### Where the scaling gap comes from, final

apt-fa-transfer at 16 threads, run B, µs per transaction:

| | V1 | MonoMove | MonoMove share of its stage |
|---|---|---|---|
| ideal parallel work (2T worker CPU / 16) | 25.0 | 6.6 | 39% |
| CPU inflation 2T→16T (growth / 16) | +4.9 | +2.4 | 14% |
| per-block cost inside Block-STM (setup, finalize, epilogue, worker drain) | +2.9 | +1.6 | 10% |
| = inner block executor wall | 32.8 | 10.7 | |
| per-block cost outside Block-STM (env rebuild, output parse, `State::update`) | +8.0 | +6.3 | 37% |
| = execution stage wall | 40.8 | 16.9 | 100% |

Per-component growth of worker CPU from 2 to 16 threads, from the profile:

| component | V1 growth | MonoMove growth |
|---|---|---|
| kernel (IPIs, syscalls) | +7.8 | +5.8 |
| SipHash (mvhashmap DashMap keys) | +6.4 | +4.2 |
| DashMap shards | +6.1 | +2.0 |
| mvhashmap (reads, dependency registration) | +3.3 | +2.2 |
| libc | +5.9 | +3.8 |
| BTreeMap (versioned entries, dependencies) | +2.6 | +3.0 |
| allocator | +2.7 | +1.9 |
| RocksDB | −1.2 | +2.3 |
| metrics | +1.8 | +0.6 |
| uncategorized | +13.1 | +5.5 |
| **shared, total** | **+48** | **+31** |
| VM-own: legacy VM, interpreter, type interners, `StateKey` registry, view | +33 | |
| VM-own: mono core/context, runtime, interpreter, bcs | | +7 |
| **total** | **+88 (1.21x)** | **+47 (1.41x)** |

The shared components grow by about the same absolute amount on both VMs,
and V1's own code grows more than MonoMove's. MonoMove's ratio is worse
because the same shared growth lands on a base a quarter the size. Nothing
in the parallel part is specific to MonoMove after the order-book fix.

The same holds for the fixed costs: the per-block work outside Block-STM is
6.3 µs/txn on MonoMove and 8.0 on V1 (V1 pays more for `insert_verified`
and speculative log flushes), the same shared code, 37% of MonoMove's stage
against 20% of V1's.

### Proposed fixes, ranked by what they buy MonoMove at 16 threads

1. Overlap block N+1's execution with block N's post-processing, or move
   the serial parts off the critical path: `AptosEnvironment` rebuild in
   `AptosModuleCacheManager::try_lock` (1.0 µs/txn), `TransactionsToKeep::index`
   (1.0), `State::update` (1.3–1.7), cache priming and memorized reads.
   Worth up to 6.3 µs/txn, 37% of MonoMove's execution stage; takes its
   execution-stage speedup over V1 from 2.4x to 3.1x. Shared.
2. mvhashmap: replace `RandomState` with a fast hasher on the versioned
   data and group maps, and shard or remove the per-entry
   `Mutex<RegisteredReadDependencies>`. About 6 µs/txn of the +47 at 16T,
   both VMs. Shared.
3. Per-block costs inside Block-STM: `MVHashMap`/scheduler construction and
   drop, the sequential block epilogue, worker init and end-of-block drain.
   1.6 µs/txn, 10%. Shared.
4. jemalloc purge-driven TLB flushes: a few µs/txn of IPI handling across
   all threads at 16T, both VMs. Tunable through `MALLOC_CONF` decay
   settings; worth a measurement, not a code change.
5. MonoMove's own +7: `struct_tag_of` and read-set bookkeeping in
   `charge_non_read_set_slots`; and `eq_value` returning false, 0.04
   aborts/txn at 16T. Small.

Done on this branch: the instantiation map (order book +144% at 16T),
hot-state read/write keys (parity), uninitialized evacuation and arena
regions (small).

## 2026-11-06: five transactions per sender (run 42279519289)

The e2e job's block shape, on the fixed build, 2 and 16 threads, no profiles.

| 16 threads | V1 inner µs/txn | MonoMove inner µs/txn | aborts/txn (both) | MonoMove/V1 |
|---|---|---|---|---|
| no-op, 1 per sender | 22.9 | 6.3 | 0.0 | 3.6x |
| no-op, 5 per sender | 33.1 | 8.2 | 1.9 | 4.0x |
| apt-fa-transfer, 1 per sender | 32.8 | 10.7 | 0.0 | 3.1x |
| apt-fa-transfer, 5 per sender | 44.5 | 13.9 | 2.0 | 3.2x |

Two speculative aborts per transaction cost both VMs 23–31% of throughput at
16 threads and about 30% more worker CPU, in proportion to each VM's own
execution cost, so the ratio does not move. The e2e job's chained senders
make its absolute numbers pessimistic but are not why MonoMove's ratio
falls with thread count. Dependency waits stay at zero: v2 aborts and
re-executes rather than parking.

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

## 2026-11-07: overnight batch

Each experiment is one commit on the branch and one CI sweep against the
commit before it, on the same recording. Cross-instance noise on the runner
is ±5–10% per job and moves both VMs together, so V1 is the control in every
comparison below: an effect that is not larger than the V1 shift is noise.

### Block size 1000 vs 4000: fixed and variable per-block cost (run 42313751859)

Same transactions, blocks of 4000 instead of 1000, 16 threads, µs/txn:

| apt-fa-transfer, 16T | inner (Block-STM) | execution stage | outside Block-STM |
|---|---|---|---|
| V1, 1000 | 32.7 | 40.9 | 8.2 |
| V1, 4000 | 31.7 | 36.4 | 4.7 |
| MonoMove, 1000 | 10.7 | 16.9 | 6.2 |
| MonoMove, 4000 | 9.4 | 12.8 | 3.4 |

Solving the two block sizes for a fixed and a variable part: the work
outside Block-STM is 1.7–2.2 ms per block plus 2.9–4.2 µs per transaction
(env rebuild, cache lock, `State::update` and index allocation are the fixed
part; `TransactionsToKeep::index`, cache priming and output conversion the
variable part). Inside Block-STM the fixed part is only 0.27 ms per block
(mvhashmap and scheduler construction, worker start and drain, epilogue).
MonoMove's execution stage drops 24% with the larger block, V1's 11%: the
fixed per-block cost is the same milliseconds on both and MonoMove's block is
shorter. Order book and account generation did not run (job flake).

### Environment cache (commit c5fed0aec0)

`AptosModuleCacheManager::try_lock` rebuilt `AptosEnvironment` on every block:
read six config resources, deserialize the gas schedule and features, build
the native table. The manager now keeps the six raw config values and
rebuilds only when one changed or the block is not the immediate successor.
Outside-Block-STM time per transaction, blocks of 1000, both thread counts:

| | before | after |
|---|---|---|
| order book, MonoMove | 7.2–7.5 | 5.8 |
| order book, V1 | 8.8–9.2 | 7.7 |
| apt-fa-transfer, MonoMove | 6.2–6.6 | 5.3 (5 per sender) |
| apt-fa-transfer, V1 | 8.2–8.6 | 7.1 (5 per sender) |

About 1.0–1.4 µs/txn, 1.0–1.4 ms per block, shared. MonoMove still goes
through the manager for the module cache lock and the gas parameters it
reads from the environment; a MonoMove-only path that skips the environment
entirely would remove the remaining lock and check, under 0.3 ms per block.

### mvhashmap hasher (commit 3d041648db, run 42313734661)

`versioned_data`, `versioned_group_data` and `versioned_delayed_fields` hash
their DashMaps with `ahash` instead of SipHash. In the 16-thread profiles
SipHash drops from 4.9 to 1.6 µs/txn of worker CPU on MonoMove and from 13.4
to 11.5 on V1 (the remainder is the executor-side `HashMap<StateKey, _>`s in
the read and write sets and the state view, which still use SipHash). Inner
wall clock moves inside noise: no-op both VMs −11%, transfer MonoMove +1% V1
+4%, account generation both +7%. Kept: it is a few percent of worker CPU
for free, but it is not the scaling gap.

### jemalloc purge (run 42313769089, `MALLOC_CONF=dirty_decay_ms:-1,muzzy_decay_ms:-1`)

No effect at all: inner wall, worker CPU, system CPU and minor faults all
within 3% of the default, and the TLB-shootdown IPI share in the profiles is
unchanged (6.6 µs/txn at 16T on both VMs with and without purging). The IPIs
are not jemalloc's purge. Candidates: `munmap` of the per-block arena regions
and thread migration between cores; both are per block, not per transaction,
and would shrink with block size, which matches the 4000-block result. Not
worth a code change at this point.

### Kill switch for MonoMove (commit 14f77fad07, runs 42316388679 / 42316422942)

V1 stops an aborted incarnation at the next gas charge; MonoMove ran it to
the end. `ResourceProvider::check_interrupted` is polled every 64 basic
blocks and on every call, and raises an abort when the scheduler has already
invalidated the incarnation. Order book (the conflict-heavy workload),
MonoMove, 16 threads:

| | before | after |
|---|---|---|
| aborts/txn | 6.1 | 6.3 |
| interrupts/txn | – | 0.48 |
| worker CPU in VM, µs/txn | 714 | 598 useful + 109 cut short |
| inner TPS | 7157 | 7273 |

The kill switch cuts 16% of VM CPU and no wall clock. The order book is a
serial chain through the market object: at 16 threads every transaction is
executed about seven times and the critical path is the chain of useful
executions, 140 µs each, 7.1k TPS at both 2 and 16 threads. The abort of
transaction i arrives when i−1 applies its writes, which is when i is itself
about to finish, so only 8% of aborts (0.48 of 6.3) catch a running
incarnation. On the chained no-op and transfer workloads (2 aborts/txn) the
interrupt count is zero: those aborts always arrive after the incarnation
completed. The kill switch is a CPU saver, not a throughput lever; the lever
on this workload is not executing i until i−1 has applied (see D below).

### End-to-end timeline in pipelined mode (run 42313786309, `BLOCK_TIMELINE_PATH`)

The benchmark's normal mode (stages pipelined, not `--split-stages`), 16
threads, per stage: time inside a block per transaction and the gap between
consecutive blocks on that stage.

| 16 threads | execute inside / gap | ledger update inside / gap | commit inside / gap | end to end TPS |
|---|---|---|---|---|
| no-op, V1 | 26.8 / 8.7 | 5.6 / 30.1 | 36.7 / 0.0 | 26.4k |
| no-op, MonoMove | 10.1 / 20.7 | 4.5 / 26.5 | 31.1 / 0.0 | 31.0k |
| transfer, V1 | 39.9 / 28.5 | 13.4 / 54.0 | 68.2 / 0.0 | 13.8k |
| transfer, MonoMove | 17.9 / 44.7 | 12.7 / 49.3 | 63.0 / 0.0 | 15.5k |
| account gen, V1 | 46.0 / 21.3 | 62.8 / 0.5 | 68.1 / 0.0 | 14.0k |
| account gen, MonoMove | 19.8 / 39.4 | 11.0 / 48.0 | 59.8 / 0.0 | 16.3k |

In pipelined mode the end-to-end rate is the commit stage's (zero gap: it
never waits for input) on every workload, 31–68 µs/txn, and the execution
stage idles 45–70% of the time on MonoMove. A 2.5x faster execution stage
buys 10–17% end to end. Commit is `StateStore::commit` + RocksDB writes +
`save_transactions`; it is where the next investigation has to go if end-to-end
TPS is the target. The `--split-stages` numbers above measure the execution
stage in isolation, which is the right measurement for the VM comparison but
not for what the node would deliver.

### Pipelined indexing (commit 35d80c2ab4, runs 42319845686 / 42319862775)

`APTOS_PIPELINED_EXECUTION=1`: block N+1 executes against block N's raw
write set (a `PendingWrites` overlay in the state view) while a thread
indexes block N (`parse_raw_output`: `TransactionsToKeep::index`, cache
priming, `State::update`). Root hashes are identical to the serial path on
every workload and VM. Execution stage, 16 threads, µs/txn, same job:

| 16T | serial | pipelined | parse off the critical path | overlay build on it |
|---|---|---|---|---|
| no-op, MonoMove | 9.88 | 9.47 | 3.0 | 1.6 |
| transfer, MonoMove | 19.07 | 18.18 | 5.3 | 3.1 |
| account gen, MonoMove | 18.01 | 17.10 | 5.2 | 2.9 |
| transfer, V1 | 46.88 | 44.28 | 5.6 | 3.1 |

The stage shrinks 4–6%, less than the 4–5 µs/txn the parse step costs,
because building the overlay (clone every write's key and value into a
`HashMap`, then look each key up in the memorized reads for the usage delta)
takes 3 µs/txn on the critical path, and about 2 µs/txn of block-tree and
channel work remains. The overlay build is single-threaded over 5–6 writes
per transaction at about 500 ns each; it can be built in parallel, or the
state view can read through the raw write set and memorized reads without
copying. Not pursued tonight: the stage is not the end-to-end bottleneck (see
the timeline above), so the remaining outside-Block-STM cost matters only
for the VM comparison, where it is now 2.9–6.1 µs/txn on MonoMove against
12 µs/txn inside Block-STM.

## 2026-11-07: Block-STM v2 scheduling (commit 3881709981, runs 42325161687 / 42325171548 / 42325181250)

Design notes and the full list of ideas are in `block_stm_v2_opportunities.md`.
Three flags, off by default, A/B'd on the same recording in one job:
`BLOCK_STM_SENDER_HINTS` (a transaction's first execution waits for the
previous transaction of the same sender to execute once),
`BLOCK_STM_PREDICTIVE_STALL` (an aborted reader waits for the writer whose
entry it reads next), `BLOCK_STM_IDLE_BACKOFF` (idle workers yield, then
sleep 5 µs between polls).

### Sender hints: the chained-sender penalty is gone

Five transactions per sender (the e2e harness shape), 16 threads:

| 16T | base inner µs/txn (TPS) | hints | aborts/txn base → hints | worker CPU |
|---|---|---|---|---|
| no-op, V1 | 33.2 (30.1k) | 22.0 (45.5k) | 1.95 → 0.00 | −36% |
| no-op, MonoMove | 8.2 (122k) | 5.4 (184k) | 1.94 → 0.11 | −40% |
| transfer, V1 | 44.1 (22.7k) | 33.0 (30.3k) | 1.97 → 0.00 | −28% |
| transfer, MonoMove | 13.4 (74.5k) | 10.7 (93.8k) | 2.06 → 0.18 | −23% |

At 2 threads the same flag is worth +8% (transfer) to +32% (no-op, MonoMove).
The hinted schedule is faster than independent senders on the same build
(no-op MonoMove 184k against 134–150k with one transaction per sender):
a sender's five transactions run back to back on one worker, so its
account and store are inserted into the map once and stay in cache.
MonoMove's residual 0.1–0.2 aborts/txn are its group-slot collisions, the
same as with one transaction per sender.

The hint costs one pass over the block (`sender_predecessors`, a
`HashMap<AccountAddress, TxnIndex>`), and needs no change to validation:
a deferred transaction is simply not in the initial queue and is entered
when its predecessor finishes incarnation 0.

### Predictive stalls: no effect on chains, and why

Two variants ran: hold an aborted reader for the next writer above it at
the invalidated key, and let a writer that was aborted mid-execution stall
the readers its applied writes invalidated. Neither changed a number
(0.00 stalls, aborts identical to base). On a sender chain the reader
always reads the entry immediately below, so there is no "next writer"
between the aborting writer and the reader; and an incarnation aborted
mid-execution is stopped by the kill switch on both VMs, so it applies no
writes and invalidates nobody. The cascade (4+3+2+1 aborts per chain of
five) is readers re-executing against the aborted transaction's *existing*
stale entries, which only the third variant addresses: when a transaction
is aborted, the readers currently registered on its entries are stalled
until it has executed again. That variant is implemented after this run,
under the same flag, and is measured next. The order-book runs, where the
window of 16 does leave gaps between writers, are below.

### Idle backoff: returns 14 cores on the serial workload

Order book, 16 threads, worker CPU µs/txn:

| | base | backoff | all three flags |
|---|---|---|---|
| MonoMove scheduler (spinning) | 776 | 207 | 213 |
| MonoMove total | 1652 | 1044 | 1011 |
| MonoMove inner TPS | 7202 | 7724 (+7%) | 7672 (+7%) |
| V1 scheduler (spinning) | 6080 | 1156 | 1212 |
| V1 total | 14242 | 9139 | 8571 |
| V1 inner TPS | 851 | 852 | 854 |

The spin was also slowing the chain's own worker on MonoMove: 7% more
throughput with 14 workers asleep instead of polling the execution queue's
mutex. The remaining 207 µs/txn of "scheduler" CPU is the 5 µs sleeps
themselves: a `nanosleep` round trip costs a few µs of kernel time, so 14
workers sleeping in 5 µs slices still burn a third of a core each. The
sleep now backs off to 100 µs; with 14 sleepers the expected pick-up
latency for new work stays under 10 µs.

### Predictive stalls, third variant (commit 44281b41b7, runs 42328913619 / 42328923186)

Readers registered on an aborted transaction's entries are stalled until
it has executed again. On sender chains: still zero. A unit test with real
transactions (`e2e-testsuite` `same_sender_chain_with_scheduling_flags`,
with a debug print) showed why: an incarnation that runs with a stale
sequence number is *discarded* by the prologue, so it writes nothing, has
no entries, and nothing can be stalled on it. Every reader of the chain
reads the last successful writer's entry, and the cascade is "the next
transaction in the chain succeeds and invalidates all the others again".
Sender hints are the right tool for that shape, and they are measured
above. A mock-executor chain, where aborted incarnations do write, records
stalls as designed (`chain_stalls_readers_of_aborted`).

On the order book the aborted incarnations do write (orders placed on a
stale book), and the variant fires:

| 16T | aborts/txn | stalls/txn | VM CPU µs/txn (useful + wasted) | inner TPS |
|---|---|---|---|---|
| MonoMove base | 6.35 | – | 609 + 112 | 7104 |
| MonoMove pred | 5.13 | 2.89 | 549 + 87 | 6855 |
| MonoMove all three flags | 5.20 | 3.01 | 539 + 86 | 7583 |
| V1 base | 5.92 | – | 6114 + 1362 | 849 |
| V1 pred | 4.63 | 3.21 | 5069 + 1034 | 852 |
| V1 all three flags | 4.74 | 3.27 | 5110 + 1050 | 855 |

A fifth of the aborts and 10–17% of VM CPU, no throughput: the chain
bound again. Worth keeping as a CPU saver; the lever on this shape remains
the chain itself (D6 window, or a hint from the transaction naming its hot
object).

Regression check on the independent workloads (run 42328932781, all
three flags against base, 2 and 16 threads): every number within 2%, as
expected (0.03 aborts/txn, no stalls, no idle workers).

### Parallel overlay build (commit a190e9a329, run 42329987632)

`PendingWrites::new` now folds the block's writes on the execution pool and
merges by version. `pipelined_pending_writes` went from 3.1 to 2.4–2.9
µs/txn, far less than the parallelism should give, so the cost is not the
map inserts. A perf profile of the pipelined stage is queued (run
42331262428) to find what the per-write cost is.

### Whole-process CPU in pipelined mode (run 42331397445, 16 threads)

Thread-group CPU from `/proc/self/task` over the run (perf over-attributes
idle rayon threads through `__perf_event_task_sched_in`, so the folded
stacks are not usable for this; the `/proc` numbers are). µs of CPU per
transaction:

| transfer, 16T | par_exec (Block-STM) | smt_update | non_exe | rayon-global | io | total |
|---|---|---|---|---|---|---|
| V1 | 489 (55%) | 132 | 122 | 61 | 47 | 897 |
| MonoMove | 158 (29%) | 133 | 110 | 61 | 47 | 544 |

| no-op, 16T | par_exec | smt_update | non_exe | rayon-global | io | total |
|---|---|---|---|---|---|---|
| V1 | 329 (61%) | 37 | 84 | 40 | 22 | 538 |
| MonoMove | 91 (37%) | 25 | 64 | 36 | 15 | 247 |

MonoMove removes two thirds of the execution CPU and 39% (transfer) to 54%
(no-op) of the process's. What remains is the same on both VMs: the state
tree update (`smt_update` + the hashing part of `non_exe`, keccak in the
profiles), the ledger update on `rayon-global`, and RocksDB on `io`. In
pipelined mode those stages set the end-to-end rate (commit at 63 µs/txn
wall for transfers), which is why a 2.5x faster execution stage gives
+12% end to end. Past this point the throughput question is a storage
question, not a VM one.

Second attempt (commit b2b0725d0b, run 42333543734): keeping the slots in
the per-worker maps and skipping the second collect brings the build to
1.9–2.1 µs/txn (no-op 0.9–1.5). Its profile is now mostly rayon's own
fork, steal and yield for a job of about 12 µs of CPU per transaction
spread over 16 threads: the work is too small for a pool round trip and
too large to do serially. The right place for it is inside Block-STM's
post-commit materialization, which already visits every output, inserting
into a shared overlay as transactions commit; that is the execution-pool
shape from `execution_pool_and_group_writes.md`. With this build the
pipelined execution stage is 15.0 µs/txn for a MonoMove transfer against
16.0 serial, and the work outside Block-STM is 4.5 µs/txn of which 2 is
this build and about 2 is block-tree and channel handling.

## 2026-11-07 morning: where things stand

Branch `george/mono-scaling-sweep`, PR #20915. Commits tonight, in order:
`9fac1bf2a2` block timeline and pipelined mode in the sweep; `3d041648db`
mvhashmap on ahash; `c5fed0aec0` environment cache; `14f77fad07` MonoMove
kill switch; `35d80c2ab4` pipelined indexing; `3881709981` predictive
stalls, sender hints, idle backoff (flags, off by default); `6a96a24020`
output bytes by kind; `44281b41b7` stall the readers of an aborted
transaction's entries; `a190e9a329` and `b2b0725d0b` parallel overlay
build; tests for the flags. Design notes: `block_stm_v2_opportunities.md`,
`execution_pool_and_group_writes.md`.

What moved the numbers:

1. **Sender hints** (`BLOCK_STM_SENDER_HINTS=1`): the e2e harness shape (five
   transactions per sender) goes from 2.0 aborts/txn to 0 on V1 and 0.1–0.2
   on MonoMove, +26–52% throughput at 16 threads on both VMs, −23–40% worker
   CPU. Independent workloads unchanged. This is the one to turn on.
2. **Idle backoff** (`BLOCK_STM_IDLE_BACKOFF=1`): on a serial-chain block, 14
   idle workers stop burning 776 (MonoMove) / 6080 (V1) µs/txn spinning;
   +7% throughput on MonoMove from less contention on the queue.
3. **Predictive stalls** (`BLOCK_STM_PREDICTIVE_STALL=1`): −20% aborts and
   −10–17% VM CPU on the order book, nothing on sender chains (stale
   incarnations there are discards with no entries). CPU only.
4. **Environment cache** and **pipelined indexing with the parallel
   overlay**: the work outside Block-STM per block goes from 6.2 to 4.5
   µs/txn on a MonoMove transfer (−1.5 ms per block of 1000); the execution
   stage from 16.9 to 15.0.
5. Kill switch, ahash, MALLOC_CONF: kept where free, none a throughput
   lever.

What did not move, and why: the order book is bound by its chain (140
µs per link on MonoMove, 7.1–7.7k TPS at any thread count); MonoMove's
ratio to V1 at 16 threads is set by the fixed per-block costs, which are
the same milliseconds on both VMs and a larger share of MonoMove's shorter
block (block size 4000 recovers most of it); and in pipelined mode the
node is commit-bound at 31–68 µs/txn, with the storage pipeline costing
the same ~390 µs/txn of CPU on both VMs against MonoMove's 158 of
execution. The next levers are outside the VM: member-level group writes
(write set −60–70%, protocol change, write-up in
`execution_pool_and_group_writes.md`) and the state-tree update.

Open questions for the user: whether sender hints should become the
default (a one-line change) and whether the member-level group write
format is worth a protocol proposal. The full 2–16 thread curve with the
hints on is in the last section below.

### Open question: V1's state view reads

`CachedStateView` counters over the execution stage (transfer, 16T, one
transaction per sender): V1 makes 2.6 `get_state_slot` calls per
transaction, 1.84 served by the speculative layer and 0.74 by RocksDB, for
42 µs/txn of its worker CPU (`storage/get_state_value` timer, 8% of V1's
total). MonoMove's count on the same recording is 0.01–1.8 depending on the run
(no-op serial 1.76 with 0.9 cold, the same binary in pipelined mode 0.009;
account generation 0.12), while V1's is stable at 1.5–3.7, even though
MonoMove's base reads go through the same `get_state_value`. Either V1
reads keys MonoMove does not (existence checks on keys that are never
present would each be a cold negative lookup), or the counter's window
misses MonoMove's reads in some configurations. Not resolved tonight;
the V1 figure alone is worth a look since it is a V1-only 8%, and the
MonoMove figure needs the counter checked before it is trusted.

### The scaling curve with the hints on (run 42336069071, five per sender)

Inner TPS (Block-STM only) and aborts per transaction, 2 to 16 threads,
`BLOCK_STM_SENDER_HINTS=1 BLOCK_STM_IDLE_BACKOFF=1` against base, same job:

| | 2T | 4T | 8T | 16T | 16T/2T |
|---|---|---|---|---|---|
| no-op, V1 base | 6.8k (0.50) | 9.9k (1.39) | 16.9k (1.76) | 30.3k (1.95) | 4.5x |
| no-op, V1 hints | 7.7k (0) | 13.8k (0) | 25.3k (0) | 45.9k (0) | 6.0x |
| no-op, MonoMove base | 32.3k (0.82) | 47.2k (1.33) | 75.2k (1.77) | 123k (1.94) | 3.8x |
| no-op, MonoMove hints | 41.9k (0.01) | 70.0k (0.02) | 119k (0.05) | 183k (0.11) | 4.4x |
| transfer, V1 base | 4.7k (0.49) | 7.3k (1.51) | 12.6k (1.84) | 22.6k (1.96) | 4.8x |
| transfer, V1 hints | 5.1k (0) | 9.3k (0) | 16.9k (0) | 30.1k (0) | 5.9x |
| transfer, MonoMove base | 17.5k (0.46) | 26.1k (1.46) | 43.8k (1.82) | 72.8k (2.07) | 4.2x |
| transfer, MonoMove hints | 19.2k (0.01) | 33.1k (0.03) | 57.2k (0.08) | 93.7k (0.18) | 4.9x |
| account gen, V1 base | 4.0k (0.47) | 6.6k (1.32) | 11.1k (1.81) | 19.8k (1.96) | 4.9x |
| account gen, V1 hints | 4.3k (0) | 7.8k (0) | 14.1k (0) | 25.1k (0) | 5.8x |
| account gen, MonoMove base | 16.5k (0.47) | 24.7k (1.57) | 40.8k (1.87) | 68.9k (2.04) | 4.2x |
| account gen, MonoMove hints | 18.0k (0) | 30.8k (0.03) | 52.0k (0.07) | 84.7k (0.15) | 4.7x |
| order book, V1 | 845 | 842 | 862 | 854 | 1.0x |
| order book, MonoMove | 7.7k | 7.7k | 7.8k | 7.0k → 7.8k with hints | 1.0x |

With the hints the chained workloads scale like independent ones. MonoMove
still scales 4.4–4.9x against V1's 5.8–6.0x from 2 to 16 threads, which is
the fixed per-block cost (the same milliseconds on both VMs, a larger share
of MonoMove's shorter block) measured at the start of this log, and the
ratio of MonoMove to V1 at 16 threads is 3.4x (account generation) to
4.0x (no-op) against 3.8–4.6x at 2 threads.

## 2026-11-07, session 2: protocol-level experiments on MonoMove

Direction for this session: MonoMove only, protocol changes allowed, the
question is how far end-to-end throughput can be stretched.

### Member-level group writes, B4 (commit 85bdcca081, flag `APTOS_MEMBER_GROUP_WRITES=1`)

A transaction's write set names the group members it changed under their
own resource keys; the block executor assembles each written group's blob
once at the end of the block, from the multi-version map, and the state
takes those blobs at the block's last version (member keys never reach the
state). The ledger keeps the member-level write sets. Runs 42378999479
(pipelined mode, end to end) and 42379007905 (split stages), MonoMove, 16
threads:

| 16T, pipelined | end-to-end TPS | output bytes/txn | materialize CPU µs/txn | storage CPU µs/txn (smt + non_exe + io) |
|---|---|---|---|---|
| transfer, unmetered | 15.1k → 15.7k | 1221 → 830 | 27.6 → 28.3 | 284 → 286 |
| transfer, metered | 14.5k → 12.9k | 1963 → 964 | 38.1 → 22.9 | 320 → 311 |
| no-op, metered | 18.0k → 16.9k | 1431 → 626 | 28.7 → 15.7 | 156 → 148 |
| account gen, unmetered | 15.9k → 15.8k | 688 → 669 | 23.0 → 30.6 | 229 → 229 |

The write set shrinks 30–55% and the per-transaction group assembly goes
(−40% materialize CPU in metered mode), but end-to-end throughput does not
move: the storage pipeline's CPU is the same to the microsecond. The commit
cost is per updated *state key*, and member-level writes leave the set of
updated keys (one blob per group per block) exactly as it was; the write-set
bytes only touch the ledger column, a minor part of `io`. The first
prototype also assembled the blobs serially at block end, which doubled the
execution stage's wall time (inner 11.7 → 24.0 µs/txn with unchanged CPU);
83294696f8 assembles them on one arena per worker in parallel (remeasured
below).

Verdict: not a throughput lever for this pipeline. It remains a ledger-size
and execution-CPU saving (which would matter for state sync bandwidth), at
the price of a format change and of state at a version inside a block no
longer following from the write sets.

### What the commit stage actually spends (run 42331397445, perf, idle samples removed)

Transfer, MonoMove, 16T, real CPU per transaction in the storage threads:

| thread pool | real µs/txn | largest items |
|---|---|---|
| `smt_update` | 156 | reading Jellyfish Merkle nodes from RocksDB 40, in-memory sparse Merkle combine/hash 66, keccak 28 |
| `non_exe` | 136 | transaction accumulator node reads from RocksDB 18, ledger update assembly 15, write-batch puts 11, keccak 36 |
| `io` | 89 | state KV shard commits (RocksDB write batches) 66 |
| `rayon-global` | 35 | layered map (speculative state) 15 |

Two things stand out for the storage side: the state Merkle update reads
its sibling nodes from RocksDB per key (40 µs/txn of CPU on top of the
hashing), and the transaction accumulator does the same (18). Both are
cache-shaped problems (the node LRU cache misses), not hashing. The hashing
itself is about 65 µs/txn across the two pools. None of this depends on the
VM or on the write-set format.

### The serial commit hook (commit 83294696f8)

Block-STM v2 commits transactions in order under one lock; the profile put
2.5 µs/txn of MonoMove worker CPU there (10 on V1), half of it the
hot-state accumulator inserting every committed transaction's read and
written keys into two hash sets, plus the block-end gas histograms (5000
observations while every worker waits). The promotion set is reads minus
writes over the block, which does not depend on the order, so the
accumulator is now concurrent and fed from the parallel post-commit phase,
and the histograms run on the rayon pool. Five per sender with hints,
before → after:

| 16T | commit-hook CPU µs/txn | inner wall µs/txn |
|---|---|---|
| no-op, MonoMove | 2.8 → 2.2 | 5.46 → 5.44 |
| transfer, MonoMove | 4.2 → 3.1 | 10.67 → 10.75 |
| transfer, V1 | 10.5 → 9.2 | 33.3 → 33.3 |

CPU down a quarter, wall unchanged: at 16 threads the hook holder is busy
a fifth of the time but the other workers never wait for it, so it was not
on the critical path. Kept, since it is what would bind first at higher
thread counts (the remaining 2–3 µs/txn is the gas-limit accounting and
the commit queue).

### Execution pool, B2 (commits afead1348e, b12d56efcc, 9643dc9b6f; flag `APTOS_EXECUTION_POOL=1`)

The multi-version map is kept from one block to the next on the same fork:
after a block it is collapsed to one base entry per key (the last committed
value, no versions, no readers) and parked in the module cache manager,
which drops it on a non-consecutive block or a changed environment. A key
the previous block touched then costs no state-view read and no base-value
insertion. Root hashes are identical to the base run on every workload
(the pool changes where values come from, never what they are). Three
iterations were needed before the collapse itself was cheap:

| transfer, 16T, split stages | inner µs/txn | worker CPU µs/txn | collapse µs/txn (wall) |
|---|---|---|---|
| base | 10.6 | 141 | – |
| pool, serial collapse over the whole map | 72 | 143 | ~60 |
| pool, parallel collapse, serial prune | 18.8 | 139 | ~8 |
| pool, one parallel pass per shard | 13.1 | 136 | 2.9 |

| no-op, 16T, split stages | inner µs/txn | worker CPU µs/txn | collapse µs/txn |
|---|---|---|---|
| base | 6.8 | 81 | – |
| pool, one parallel pass per shard | 5.85 (−14%, 147k → 171k TPS) | 53 (−35%) | 0.8 |

Where the key set is stable (no-op: 2000 senders cycling, the framework and
config keys) the pool removes a third of MonoMove's worker CPU: the
`set_raw_base_values` / `set_base_value` insertions, the state-view lookups
behind them and their page faults. Where every block brings new keys
(transfer and account generation: receivers never repeat) the map grows by
thousands of keys a block and the collapse, which walked every key, cost
more than the pool saved. The next iteration tracks the keys touched since
the last collapse and walks only those, which makes the collapse
proportional to the block rather than to the map.

Two things the pool had to get right: a key served from the pool is never
read through the state view, so the commit's usage and pruning accounting
("must cache read") did not find its old slot; with the pool, the parse
step primes the state cache for every updated key (off the critical path in
pipelined mode). And the previous block's registered readers must be
dropped from every entry the next block can write to, or they would be
invalidated as if they were the new block's transactions.

### Execution pool, iterations 4 to 6 (commits 8c09c81454, 47c617deca, 24a24c771e, 8a23843bc0; runs 42387704455, 42390576269, 42392236313)

Three more rounds on the collapse and on what the pool costs the commit path.

| transfer, 16T, split stages | inner µs/txn | worker CPU | collapse (wall) | `get_state_value` time | state-view gets /txn |
|---|---|---|---|---|---|
| base | 11.0 | 145 | – | 1.3 | 0.12 |
| pool, touched keys, readers touch too (8c09c81454) | 18.4 | 142 | 7.75 | 29.4 | 3.25 |
| pool, touched keys, block-tagged readers (47c617deca) | 14.6 | 141 | 4.05 | 27.7 | 3.25 |
| pool, plus committed slots (24a24c771e) | 14.6 | 142 | 4.03 | 27.1 | 0.78 |

- Tracking touched keys cut the collapse from "the whole map" to "what the
  block wrote", but the first version counted reads as touches (a reader
  registers on an entry, and the collapse had to drop those registrations),
  which on transfer is most of the map again. Tagging every dependency set
  with the block it was registered under lets a stale set be dropped on
  first access instead, so only written keys are visited: 4 µs/txn.
- 4 µs/txn for 7 items per transaction (3 resources, 2 group members, 2
  group sizes) is 0.6 µs wall per item, which with `par_chunks` over 16+
  threads means the walk was not running in parallel in practice. Rather
  than chase why, the collapse is now lazy (8a23843bc0): each per-key map
  carries the block it belongs to, the map records how many transactions
  each block committed, and whoever first touches a key under a later
  block settles it (latest committed entry becomes the base, the rest go,
  or the key is forgotten if nothing committed). Ending a block pushes one
  number and bumps a counter. Keys nobody touches again keep their old
  entries until the pool is dropped at its key budget.
- The priming the pool needs ("must cache read") was 23–29 µs/txn of CPU
  on the IO pool: a `rayon` task per written key, and for keys last written
  before the measurement window, a cold DB read (0.7/txn at 25–35 µs) to
  learn the old slot. The committed-slots map (`CommittedSlots`, kept by the
  executor next to the pool, same fork-continuity rule, 2x the pool's key
  budget) answers from what the previous blocks read or wrote; the
  state-view gets fell from 3.25 to 0.78 per transaction. The remaining 0.78
  were keys the pool holds from a read rather than a write, which the first
  version did not record; the memorized reads of every block now go in
  before its writes, as cold slots (the hot-state LRU is the authority on
  hotness for the keys it holds, and the state update asserts cold for the
  ones it does not).

The no-op numbers held throughout (5.7–5.8 inner µs/txn, 171–175k TPS vs
145k base, worker CPU 53 vs 82).

### Execution pool with lazy settling (commits 8a23843bc0, a74932ba64, 55674aadcc; runs 42393849381 split, 42393851872 pipelined)

| 16T, split stages | inner µs/txn | inner TPS | worker CPU | `execute_vm` CPU | cold DB reads /txn | RSS |
|---|---|---|---|---|---|---|
| no-op base | 6.83 | 146k | 81 | 61 | 1.02 | 1.76 GB |
| no-op pool | 5.13 (−25%) | 195k | 54 (−33%) | 35 | 0.04 | 1.67 GB |
| transfer base | 10.68 | 94k | 142 | 97 | 0.83 | 1.68 GB |
| transfer pool | 10.59 | 94k | 140 | 90 | 0.83 | 2.10 GB |
| account generation base | 13.05 | 77k | 159 | 118 | 3.02 | 1.53 GB |
| account generation pool | 12.55 (−4%) | 80k | 150 | 105 | 2.04 | 2.08 GB |

The collapse is gone from the block boundary (0.06 µs/txn, the counter
push). What the pool buys depends on what the block reads that an earlier
block touched. The sweep's senders are 2000 accounts cycling every two
blocks (`--main-signer-accounts 2000`), receivers a pool of 200k that
never repeat. On no-op every read is a sender key and every one is served
from the pool: a third of the worker CPU goes (`set_base_value` /
`set_raw_base_values`, the state-view lookups and the cold RocksDB read
behind them). On transfer the sender's account and store come from the
pool (`execute_vm` −7 µs/txn) but the receiver's store is a fresh key
either way (cold reads unchanged at 0.83/txn), and the saving is eaten by
two costs the pool adds on the worker: `execute_apply` +3 µs/txn (a
500k-key map misses the cache where a 5k-key one did not) and
`materialize` +2.6 µs/txn. The second is structural: a group member
written in an earlier block stays a `MonoValue::Write` in the pool
forever, so every later blob assembly re-serializes it, where a fresh
block would clone the stored bytes. Writing the materialized bytes back
into the entry at commit would remove it. A 2M-key budget (`big`, run
42397060800) changes nothing: the working set already fits. Pipelined
end-to-end TPS is unchanged on all three (commit-bound).

The pool's parse-side work (`absorb`, the committed-slot priming) showed
up as +76 µs/txn of CPU on the global rayon pool (62 → 138, transfer). The
profile (run 42395301955) puts almost none of it on the work itself
(`CommittedSlots::get` 0.7%, DashMap inserts 1–2%): it is rayon's idle
loop (`Sleep::sleep`, `crossbeam_epoch::try_advance`, `wake_specific_thread`),
64 workers woken for each small parallel region and spinning for a job
that 16 shards cannot give them, on 32 pinned CPUs. The lookups now ride
the priming's own per-key tasks on the IO pool and the absorb fans out to
four tasks (d13b62ef1f): rayon-global is back to 65–71 µs/txn (base
58–63) and process CPU per transaction is the same as base on all three
workloads (runs 42397060800 split, 42397063238 pipelined). Same lesson as
the eager collapse earlier: the "serial-looking" 4 µs/txn was not the
walk, it was the pool's threads contending.

Pool verdict: on MonoMove at 16 threads, −25% inner time where the
working set repeats (no-op: 146k → 195k inner TPS), −4% on account
generation, neutral on transfer; worker CPU −33% / −6% / −1%; +0.4–0.6 GB
RSS; outputs identical (root hashes equal base on every run). Nothing
end-to-end in pipelined mode. Flag `APTOS_EXECUTION_POOL=1`, MonoMove only.

### The curve with every lever on (run 42398445427, five per sender, split stages, MonoMove)

`all` = `BLOCK_STM_SENDER_HINTS=1 BLOCK_STM_IDLE_BACKOFF=1 APTOS_EXECUTION_POOL=1`
at commit d13b62ef1f. Inner block-executor TPS (thousands) and aborts per
transaction; worker CPU in µs/txn.

| | 2T base | 2T all | 16T base | 16T all | 2→16 base | 2→16 all | all/base 16T |
|---|---|---|---|---|---|---|---|
| no-op | 31.6k (0.82 ab) | 46.6k (0.01) | 121k (1.94) | 194k (0.11) | 3.84x | 4.16x | 1.60x |
| transfer | 17.5k (0.46) | 18.7k (0.00) | 72.9k (2.05) | 89.3k (0.16) | 4.17x | 4.79x | 1.22x |
| account generation | 16.2k (0.46) | 17.0k (0.00) | 66.7k (2.03) | 77.1k (0.15) | 4.12x | 4.52x | 1.16x |
| order book, one market | 7.5k (1.12) | 7.5k (1.07) | 7.1k (6.88) | 7.7k (5.33) | 0.94x | 1.02x | 1.08x |

| worker CPU µs/txn | 2T base | 2T all | 16T base | 16T all |
|---|---|---|---|---|
| no-op | 54 | 37 | 103 | 58 |
| transfer | 106 | 100 | 182 | 146 |
| account generation | 113 | 109 | 186 | 160 |
| order book | 255 | 256 | 1563 | 841 |

Sender hints take the chained-sender aborts to ~0.1/txn and are most of
the transfer and account-generation gain; the pool is most of no-op's
(its working set repeats); idle backoff halves the order book's worker
CPU without moving its chain-bound throughput. The 2→16 scaling factor
improves on every independent workload because the levers remove
per-transaction work that grew with the thread count (re-executions,
base insertions) rather than the fixed per-block part, which is what
remains between 4.8x and 8x.
