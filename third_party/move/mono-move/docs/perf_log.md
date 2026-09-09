# Sequential Block-STM performance log

Goal: 10x MonoMove over the legacy MoveVM on the `bench_*` workloads under
sequential Block-STM.

## Status: this document is the deliverable

Every VM, block-executor and storage change described below has been **reverted**.
What remains on the branch is the benchmark suite, this log, and the profiling
scripts under `docs/perf-tools/`.

That was a deliberate choice. The experiments reached 8.17x on clob and 7.16x on
aave against a 10x target, and the analysis at the end of this document shows
that the remaining gap is not reachable by more changes of the same kind. Landing
22 small, individually-marginal diffs across the interpreter, the loader, the
allocator, the metrics crate and the block executor would have bought about 40%
and made every one of those files harder to review for the next person. The
measurements are worth more than the code.

`docs/perf-tools/` and the "Change index" appendix together make any single
experiment reconstructible in an afternoon. Two of them — E7 (ahash) and E21
(strided metric flush) — are worth relanding on their own merits and are called
out there.

## Method

All numbers come from `aptos-executor-benchmark` replaying identical recorded
blocks for both VMs, so the two arms execute the same transactions.

```
--block-executor-type aptos-vm-with-block-stm --execution-threads 1
--generate-then-execute --num-generator-workers 1 --block-size 500
run-executor --replay-blocks <blocks> {--disable|--enable}-feature-after-init ENABLE_MONO_MOVE
```

30 blocks x 500 transactions, 14030 transactions executed. The tracked metric
is the `inner block executor` component TPS, which excludes ledger update and
commit. Those two overlap with execution and never bind here.

Two rules make the numbers comparable.

Both arms are built as separate binaries and interleaved rep by rep, so any
drift over the sweep hits every arm equally. Absolute TPS moved by ~30% between
sessions, so a number from an earlier sweep is not a valid control.

The reported value is the **max** over reps, not the median. The workload is a
deterministic replay and the interference on this machine is one-sided: another
process can only make a run slower, never faster. Under one-sided noise the max
is the estimator that converges on the true cost. Medians hid a real 11% gain
behind one contaminated rep in the first E2 sweep.

Both binaries must also be built with identical flags, including debug info. The
first E3-vs-E4 sweep was thrown out because the E3 binary was 139 MB and the E4
binary 108 MB: the E3 one had been built for profiling and carried symbols. The
legacy arm, which no MonoMove change can touch, read 1744 TPS from the E3 binary
and 1905 from the E4 one, a 9% gap that is purely the build. Watch the legacy
arm — it is the built-in control, and any movement in it means the comparison is
invalid.

The harness and the profile-analysis scripts are in `perf-tools/`. Its README
has the full set of rules for running a sweep and for reading a profile without
being misled.

## Baseline (2026-09-06)

| workload | legacy inner TPS | mono inner TPS | speedup |
|---|---|---|---|
| bench-clob | 1738 | 10112 | 5.82x |
| bench-aave | 1836 | 9875 | 5.38x |

Peak RSS for the mono clob run: 294 MiB.

## Scoreboard

| change | clob | aave |
|---|---|---|
| baseline | 5.82x | 5.38x |
| E2 growable heap | 6.47x | 6.13x |
| E2 re-measured on a quieter machine | 6.61x | 6.09x |
| E3 uninitialized interpreter stack | 6.90x | 6.49x |
| E3 rebuilt without profiling symbols | 6.72x | 6.14x |
| E4 fewer allocations per operation | 6.81x | 6.20x |
| E4 re-measured in the E5 sweep | 6.80x | 6.21x |
| E5 inline cache for indirect calls | 7.22x | 6.40x |
| E6 shared root pool (reverted) | 7.07x | 6.34x |
| E7 ahash for the per-block maps | 7.25x | 6.48x |
| E7 re-measured in the E8 sweep | 7.27x | 6.51x |
| E8 cached storage tags | 7.48x | 6.51x |
| E8 re-measured in the E9/E11 sweep | 7.38x | 6.50x |
| E9 memoized dependency slices (reverted) | 7.26x | 6.29x |
| E11 streamed write set, measured on top of E9 | 7.37x | 6.42x |
| E11 streamed write set, on E8 after the E9 revert | **7.60x** | **6.63x** |
| E11 re-measured in the E13 sweep | 7.49x | 6.58x |
| E12+E13+E14 sized read-set, inlined moves, sized codec buffer | **7.74x** | **6.86x** |
| E12+E13+E14 re-measured in the E15/E16 sweep | 7.67x | 6.84x |
| E15 pre-sized group blob (reverted, inside noise) | 7.67x | 6.85x |
| E16 state-view stopwatch removed (measurement only) | 7.86x | 6.95x |
| E12+E13+E14 re-measured in the E17 sweep | 7.71x | 6.88x |
| E17 fused move runs | **7.91x** | **6.95x** |
| E17 re-measured in the E18 sweep | 7.81x | 6.86x |
| E18 taken resource groups, ahash, sized read set | **8.01x** | **7.11x** |
| E19 drop no-op writes (reverted) | 8.03x vs 8.12x | 7.07x vs 7.29x |
| E20 state-view metrics removed (measurement only) | 8.18x vs 8.06x | 7.37x vs 7.19x |
| E21 strided flush check in `aptos-metrics-core` | **8.17x** (E20 ceiling 8.16x) | **7.16x** (E20 ceiling 7.29x) |

Only compare rows measured in the same sweep. Absolute TPS drifts between
sessions and the ratio itself depends on the build: the same E3 source reads
6.90x with profiling symbols and 6.72x without, because the symbol-free binary
speeds up the legacy arm more than the mono one.

## Where the time goes

From the samply decomposition (see `project_mono_e2e_vs_microbench_gap`, and
the tables below are per transaction, clob):

| region | legacy | mono | ratio |
|---|---|---|---|
| profiled thread | 737.1 us | 131.6 us | 5.60x |
| inside the VM (`execute_user_transaction`) | 669.0 us | 87.1 us | 7.68x |
| outside the VM | 68.1 us | 44.5 us | 1.53x |

Two separate problems. The VM-to-VM ratio on real transaction code is 7.7x, not
the 20-26x the criterion micro-benches show, because the micro-benches are
almost entirely the value-representation work that MonoMove deletes. And 34% of
mono's remaining time is outside the VM, where nothing improved.

Storage reads cost the same in both: `aptos_storage_interface` inclusive is
15.4 us/txn in mono and 17.0 us/txn in legacy. That is a hard floor.

## Experiments

Each entry records what changed, what it was expected to save, and what it
actually saved.

### E1: reuse the per-block 64 MiB arena

Status: implemented, measured, reverted. No gain — the premise was wrong.

The idea was that `UnsyncMap`'s 64 MiB `SharedArena` costs a mmap and a munmap
per block. I gave `SharedArena` a `reset`, moved the arena into a per-thread
slot, and rewound it at block start when `Arc::strong_count == 1` proved no
pointer from the previous block survived.

Result on clob: 5.74x against a 5.80x baseline. Nothing, inside noise.

The reason is a misattribution in the earlier profile. All the `munmap` under
`UnsyncMap::into_modules_iter` was read as "the arena being freed", because
`into_modules_iter` takes `self` and therefore drops the whole map. Walking the
actual sample stacks shows otherwise. On clob, of the 149 `munmap` samples on
the executor thread:

| callee chain | samples |
|---|---|
| `Arc::drop_slow` under `hashbrown::RawTable::drop` under `into_modules_iter` | 118 |
| `Arc::drop_slow` under `TxnOutcome` drop glue | 31 |
| the arena's own `Arc` | 0 |

Those `Arc`s are the per-transaction `FrozenHeap`s pinned by the write set, not
the arena. The arena is one allocation per block against 500 heaps, so it was
never going to matter.

The aave arm of that sweep read 2.42x, but the same run also shows ledger
update at 6823 TPS against 40226 in the baseline and commit at 13303 against
52578. Those stages do not touch the VM. The machine was busy with something
else; the number is discarded, not attributed to the change.

### E2: stop allocating 11 MiB per transaction

Status: implemented, measured, kept. **clob 5.82x -> 6.47x, aave 5.38x -> 6.13x.**

Every transaction allocates a 1 MiB zeroed stack and a 10 MiB heap in
`InterpreterContext::with_heap_size`. This is where the mmap traffic actually
comes from. Per transaction on the executor thread, mono clob:

| symbol | share of thread |
|---|---|
| `__munmap` | 8.1% |
| `__mmap` | 4.5% |
| `_platform_memset` (the zeroed stack) | 3.2% |
| `InterpreterContext::new_idle` | 1.7% |

Roughly 17% of the executor thread, and legacy pays none of it.

Two things make it expensive rather than merely large. jemalloc's
`oversize_threshold` is 8 MiB, so a 10 MiB request goes to the dedicated
oversize arena and is unmapped eagerly on free instead of being recycled. And
the heaps are retained: `InterpreterContext::finish` wraps the heap in
`Arc<FrozenHeap>` (`runtime/src/interpreter.rs:477`), the write-set entries the
block executor keeps in `UnsyncMap` pin it, and `StorageRead::ExternalHeap`
carries an `Arc<dyn ReadPin>` for reads. So a transaction whose writes survive
to the end of the block keeps its whole 10 MiB heap alive, and all 500 are
released at once when the map drops.

Measured RSS stays at 294 MiB because untouched pages of a 10 MiB mapping never
become resident. The cost is the syscalls and the first-touch faults, not the
footprint.

#### How much a transaction actually uses

`runtime/src/measure.rs` records the high-water mark of both regions under
`MONO_MEASURE=1`. It is scaffolding and comes out before the commit.

| workload | heap max | heap avg | stack max | GCs |
|---|---|---|---|---|
| bench-clob | 4.2 KiB | 1.6 KiB | 1.8 KiB | 0 |
| bench-aave | 10.0 KiB | 2.9 KiB | 3.3 KiB | 0 |

Three orders of magnitude of headroom, and not one collection in 14030
transactions.

#### Size sweep

Fixed heap and stack sizes, mono inner TPS, no legacy arm:

| heap | clob | aave |
|---|---|---|
| 32 KiB | 11222 | 11240 |
| 64 KiB | 11442 | 11211 |
| 128 KiB | 11389 | 11258 |
| 512 KiB | 11283 | 11225 |
| 2 MiB | 11013 | 10628 |
| 8 MiB | 10239 | 9776 |
| 10 MiB (before) | 8758 | 9782 |

Flat from 32 KiB to 512 KiB, then falling once the request crosses jemalloc's
8 MiB threshold.

The stack does not matter on its own. At a 10 MiB heap, shrinking the stack to
64 KiB gives clob 10156 and aave 10022. Shrinking both gives 11422 and 11221,
no better than shrinking the heap alone. So the stack was left at 1 MiB.

#### The change

`INITIAL_HEAP_SIZE = 256 KiB`, growing on collection up to the existing
`MAX_HEAP_SIZE = 10 MiB`. 256 KiB rather than 64 KiB because the flat region is
wide and the larger start buys margin for transactions heavier than these two
without touching the fast path.

Failure semantics are unchanged. `MAX_HEAP_SIZE` still bounds both the heap and
any single allocation, so a transaction that fits today still fits and one that
aborts today still aborts. What changes is that a transaction reaches that bound
by collecting into progressively larger to-spaces instead of starting there.

Result, four interleaved reps per arm:

| workload | legacy | mono before | mono after | before | after |
|---|---|---|---|---|---|
| bench-clob | 1738 / 1728 | 10112 | 11182 | 5.82x | 6.47x |
| bench-aave | 1836 / 1828 | 9875 | 11210 | 5.38x | 6.13x |

The predicted 17% of the executor thread, recovered as 11% and 14% of end-to-end
inner TPS.

### E3: stop zeroing the interpreter stack

Status: implemented, measured, kept. **clob 6.61x -> 6.90x, aave 6.09x -> 6.49x.**

E2 shrank the heap but left the 1 MiB stack at `MemoryRegion::new_zeroed`. On
macOS jemalloc, `alloc_zeroed` of an extent that large goes
`extent_recycle -> extent_commit_zero -> pages_commit_impl -> mmap`, one syscall
per transaction, and the first touch of each page then faults. `__mmap` alone
was 2.8% of the mono executor thread.

Zeroing was never load-bearing. `Function::zero_frame` makes the runtime zero
`param_region_size..extended_frame_size` when it creates a frame, so the GC sees
null in a slot the function has not written yet, and `safe_point_layouts` only
names slots already written at that PC. The region is therefore written before
it is read by construction, which is exactly `new_uninit`'s contract.

`new_uninit` also poisons the region with `0xAA` under `debug_assertions`. Fresh
OS pages read as zeros, so without the poison any code that wrongly depended on
the old zeroing would keep passing in tests.

| workload | legacy | mono before | mono after | before | after |
|---|---|---|---|---|---|
| bench-clob | 1737 | 11484 | 11983 | 6.61x | 6.90x |
| bench-aave | 1856 | 11146 | 12050 | 6.09x | 6.49x |

+4.3% and +8.1% on mono TPS, against a 2.8% `__mmap` share. The gain is larger
than the syscall it removed because the page faults and TLB misses on first
touch were charged elsewhere.

## Where the time goes after E3

Fresh decomposition of the mono clob executor thread, 4575 samples:

| phase | share |
|---|---|
| interpreter | 58.5% |
| block-STM glue | 13.8% |
| materialization | 13.6% |
| loader | 11.7% |
| other | 2.3% |

The interpreter's share rose from 52.8% to 58.5% across E2 and E3, which is what
a real win looks like: the denominator shrank and the numerator did not.

It also sets the ceiling. Deleting *all* non-interpreter work would give
1/0.585 = 1.71x, so about 11.8x. 10x is reachable, but not by finding one more
big item. It needs the whole 41.5% tail attacked.

## Where hashing comes from

The question from the goal, answered against the E3 profile. There is no single
hot map. The cost is four separate ones, each hashing a different key type with
a different hasher.

| site | hasher | key | driven by |
|---|---|---|---|
| `ResourceReadWriteSet::entries` | ahash (hashbrown) | `InMemoryStorageKey` | `table::borrow_box` — the CLOB's AVL queue is a `Table`, so every node read is a map probe |
| `UnsyncMap::{fetch_data, set_base_value}` | SipHash (std `HashMap`) | `StateKey` | one probe per read and per write, block executor side |
| `TxnOutcome::materialize` group map | SipHash | `StateKey` | resource-group assembly |
| `ModuleReadSet` | FxHash | interned pointer | rebuilt empty per transaction, so it shows up as `reserve_rehash` |

Only the first is inside the VM. The other three are block-executor and
materialization cost that legacy pays too, in absolute terms.

`keccakf` is 1.15% of the thread and is **not** table keys, which was the
initial guess. Walking its callers: `AuthenticationKey` sha3 0.67%,
`SessionId::hash` 0.32%, `TwoKeyRegistry` 0.16%. That is transaction prologue
work, not the data structures.

## Named costs not yet addressed

Percentages are of the mono clob executor thread.

**`mach_absolute_time`, 3.07%.** The single largest leaf. Essentially all of it
is under `cached_state_view::get_state_slot`, reached from
`BlockSTMSequentialProvider`. That function takes one `TIMER.timer_with` (a
clock read at start and another on drop) plus two to four `COUNTER.inc_with`,
and `aptos-metrics-core`'s thread-local counters call `maybe_flush()` — which
reads the clock — on *every* increment, not once per flush interval. Four or
five clock reads per cold state read.

This is shared infrastructure. Legacy pays the same absolute cost but, being
6.9x slower, it is only ~0.5% of legacy's time. Fixing it would move mono ~3%
and legacy ~0.5%, so ~2.5% of ratio. Left alone for now because it touches
production metrics that other teams read.

**`RootPool::new` inlined into `InterpreterContext::run`, 1.01%.** A
`ProductionNativeContext` is built per native call and owns a `RootPool` by
value. Addressed in E4.

**`RootPool::alloc` + `ReferenceHandle::drop` from `table::borrow_box`, ~1.2%.**
Rooting machinery on the hottest native in the CLOB.

**Loader, ~3.1%.** `charge_non_read_set_slots` self time 1.84%, `ModuleReadSet`
insert/rehash/hash ~1.3%. Partly addressed in E4.

**Struct tags rebuilt per write, ~0.55% in materialization and more elsewhere.**
`nominal_tag` / `struct_tag_of` reconstruct a `StructTag` from the interned type
on every write, which runs `Identifier::new` and therefore
`move_core_types::identifier::is_valid` over the module and struct names. There
is already a `TODO(perf)` asking for a cached tag at
`core/src/storage/resource_provider.rs:112`.

**Allocator traffic inside materialization, ~3.7% of the thread.** malloc/free,
`finish_grow`, and memmove are 27% of the materialization region. Addressed in
E4.

**`ContractEventV2::new`, 0.71%.** Runs `bcs::serialized_size` over the event's
`TypeTag` to compute a size. `SignedTransaction::txn_bytes_len` is another
0.36%.

### E4: stop copying bytes that do not need copying

Status: implemented, measured, kept. **clob 6.72x -> 6.81x, aave 6.14x -> 6.20x.**

Two allocation-per-operation patterns, both found in the E3 profile.

`drain_write_set` called `serialize`, which allocated a fresh `Vec` per written
value and grew it from empty. A transaction with N writes therefore did N
allocations plus their growth reallocs. `serialize_into` takes the output buffer
by reference, so the whole drain uses one buffer: the first few writes grow it,
the rest reuse the capacity. `Bytes::copy_from_slice` still takes an exact-size
copy at the end, so the write set is unchanged.

`InMemoryStorageKey::TableItem` held its key as `Box<[u8]>` while
`table::handle_and_key` already produced an owned `Vec<u8>`. Every table
operation therefore did one extra malloc, memcpy, and free purely to change the
container. The four `NativeContext::table_*` methods now take the key by value
and the variant stores a `Vec<u8>`.

Also in this batch: `RootPool`'s inline capacity drops from 16 to 4, taking the
per-native-call pool from ~400 bytes of stack traffic to ~112, and
`record_loaded_and_charge_slots` gets a `ModuleReadSet::meter_once` that does in
one map lookup what `get` plus `mark_metered` did in two, with a `reserve` up
front so the read-set stops rehashing as a dependency set is recorded.

| workload | legacy | mono before | mono after | before | after |
|---|---|---|---|---|---|
| bench-clob | 1908 / 1906 | 12819 | 12985 | 6.72x | 6.81x |
| bench-aave | 2064 / 2061 | 12674 | 12768 | 6.14x | 6.20x |

+1.3% and +0.7%. Small, but the same sign on both workloads and the legacy arms
agree to 0.15%, so it is not noise. Four separate allocation sites were removed
and together they were worth about one percent, which sets the scale for
this kind of change: picking off single-percent items will not reach 10x.

### E5: inline cache for indirect calls

Status: implemented, measured, kept. **clob 6.80x -> 7.22x, aave 6.21x -> 6.40x.**

The specializer never emits `CallDirect`. Every Move-to-Move call is a
`CallIndirect` carrying `(module_id, func_name, ty_args)`, and every execution
of one re-ran the full resolution: intern the module id, look it up in the
read-set, look the function slot up in the module, then walk the callee's whole
mandatory dependency set with one more read-set lookup per dependency. The E3
profile put 108 of the 123 `Loader::load_function` samples under
`InterpreterContext::run`, so this was a pointer being recomputed, not a module
being loaded.

The triple is baked into the instruction, and a transaction pins one version of
every module it touches, so within a transaction the address of the instruction
alone determines the target. `runtime/src/call_cache.rs` is a 512-slot
direct-mapped table keyed by that address, built per transaction alongside the
rest of the interpreter context.

Two properties make it safe to keep this simple. A miss costs nothing but the
old path, because a repeat `load_function` in the same transaction returns the
same pointer and charges no gas — every module it would charge for is already
metered. So capacity and eviction are free choices, and no entry needs
validating on lookup. Lifetime is the one thing that is not free: an entry must
not outlive its transaction, or a module upgrade would be invisible to the call
site. A fresh context per transaction gives that, and `reset` clears the table
explicitly because it also installs a fresh gas budget.

| workload | legacy | mono before | mono after | before | after |
|---|---|---|---|---|---|
| bench-clob | 1924 / 1905 | 13071 | 13757 | 6.80x | 7.22x |
| bench-aave | 2059 / 2046 | 12784 | 13100 | 6.21x | 6.40x |

+5.2% on clob, +2.5% on aave. Five times the size of E4 for a smaller diff. The
remaining `TODO(perf)` at the call site is patching: rewriting a hit site to
`CallDirect` would remove the lookup as well, but code arrays are shared across
transactions, so that needs a per-transaction copy of the code first.

### E6: share one root pool per transaction — reverted

Status: implemented, measured, reverted. **clob 7.22x -> 7.07x, aave 6.40x ->
6.34x.**

`ProductionNativeContext` builds a `RootPool` per native call while the
interpreter already owns one per transaction, and `RootPool::new` plus
`alloc`/`drop` was 5.0% of the interpreter region in the E3 profile. Borrowing
the interpreter's pool instead of building one looked free: the native's handles
are all released before it returns, so the pool comes back empty with its slots
recycled, and the GC would scan strictly more roots than before, never fewer.

It measured 1.2-1.6% slower, in the same direction on both workloads and on
every rep, against legacy arms that agree to 0.5%. The likely reason is that a
transaction-wide pool reaches a high-water mark that spills its `SmallVec` past
the four inline slots and stays spilled, turning every root access into an extra
load; a per-call pool almost always stays inline. Sharing also puts the pool
behind a reference into the interpreter context rather than in the native
context itself, which costs an indirection and may cost aliasing information.

Reverted. Worth recording because the change looked strictly better on paper:
fewer constructions, fewer allocations, more roots scanned. Allocation count is
not the thing that matters here; whether the hot structure stays inline is.

## Where the time goes after E5

The phase splits above are shares of one arm, which makes them useless for
comparing arms. This one is absolute, and it took two corrections to get right.

Method, stated precisely because both mistakes came from getting it wrong.
Profile both arms at 4999 Hz on the same recorded clob blocks. Two threads in
this benchmark are named `txn_executor`; the small one runs genesis and the
feature-flag transaction, the large one replays the measured blocks. Take only
the large one. Then weight each sample by its `threadCPUDelta` rather than
counting samples, because samply drops samples and does not drop them at the
same rate in both arms — on this pair the mono thread's 3935 samples represent
1.092 s of CPU while the sample count alone implies 0.787 s, a 39% undercount,
against 5% for legacy.

With that, the executor thread's CPU accounts for 95% of mono's reported
inner-block-executor time and 93% of legacy's, and the CPU ratio is 7.17x
against a reported 7.32x. The model closes.

| bucket | mono | legacy |
|---|---|---|
| interpreter, natives, VM glue | 25.54 us | 272.00 us |
| hashing and hash tables | 9.05 us | 52.42 us |
| allocator | 7.77 us | 94.68 us |
| memmove / memset / memcmp | 6.78 us | 22.48 us |
| storage engine (rocksdb, LZ4) | 5.04 us | 11.60 us |
| loader, verifier, specializer | 4.69 us | 2.42 us |
| metrics | 3.22 us | 9.21 us |
| state view | 2.71 us | 7.05 us |
| serialization and type names | 2.44 us | 8.73 us |
| block-STM glue | 1.55 us | 20.79 us |
| thread synchronization | 1.39 us | 5.14 us |
| materialization | 0.98 us | 3.68 us |
| unclassified | 6.66 us | 47.40 us |
| **total CPU on the executor thread** | **77.82 us** | **557.61 us** |

Three things follow.

The ratio is decided on this thread. Off-thread and off-CPU time is 5% of the
budget on one side and 7% on the other, so there is no hidden per-block cost to
chase and no pipeline stall worth fixing. Everything has to come out of the
77.82.

Legacy pays more in every bucket except loading. There is no shared fixed floor
in the sense of an identical absolute cost on both sides; legacy performs more
state reads and more metric increments for the same transaction, so even the
shared infrastructure costs it two to four times more. The only bucket where
mono is more expensive is the loader, at 4.69 us against 2.42 us.

The interpreter is a third of mono's time and everything else is two thirds.
For 10x, mono has to reach 55.76 us/txn, which means cutting 22.06 us — 28% of
the budget. The non-interpreter tail is 52.28 us, so the target is reachable
without touching the interpreter at all, but only by taking large bites out of
hashing, allocation, and memory traffic.

### Two ways to read this profile wrong

Both are worth writing down, because both produced confident wrong conclusions
earlier in this log.

Summing every thread that shares a name. The genesis thread contributed 346
samples, 317 of which sat in `visit_dependencies_and_verify` under the legacy
framework prefetch. Read as replay cost that looked like 4.5 us/txn of redundant
legacy verification running inside the mono arm, and it is not: the prefetch is
already skipped when MonoMove is enabled, and those samples are the one-time
genesis block, outside the measured window. Timestamps settle it — all 317 land
in the first 0.1 s and the replay thread has zero.

Scaling sample counts by the nominal sampling rate. That produced 56.11 us/txn
for mono against 530.04 for legacy, an apparent 9.45x on-thread against 7.22x
reported, and an invented 2.1x of "per-block work that does not scale". The gap
was the sampler, not the system.

## Where hashing comes from, measured again after E5

The earlier answer named four maps. With absolute numbers the picture sharpens:
hashing and hash-table probing is 9.05 us per transaction, 11.6% of mono's
executor thread, split roughly

| cost | share of thread | site |
|---|---|---|
| SipHash over storage keys | 4.5% | `UnsyncMap::{write, set_base_value, fetch_data, get_group}`, `TxnOutcome::materialize` |
| hashbrown probing | 4.4% | mostly `global_storage::get_or_create_resource_entry` |
| keccak | 0.9% | prologue authentication keys, not data structures |

The SipHash share is std's `RandomState`, which `std::collections::HashMap`
installs by default. Its keys are `InMemoryStorageKey`: a 32-byte address plus
an interned type pointer for a resource, or a 32-byte table handle plus the
serialized key bytes for a table item. Every probe pushes 40 to 60 bytes through
a hash designed for adversarial short-string workloads.

Legacy also spends 4.01% of its thread in SipHash, but in different places —
`LatestView::unmetered_get_lazily`, `get_module_or_build_verified`,
`read_cached_group`, `BlockGasLimitProcessor`. Almost none of it is `UnsyncMap`.
That asymmetry is what makes the hasher worth changing: the map that dominates
mono's hashing barely registers in legacy's.

## The per-block arena is 260x larger than it needs to be

`UnsyncMap` reserves a 64 MiB `SharedArena` per block for values materialized
out of storage into MonoMove's representation. E1 established that allocating it
costs nothing measurable. What it actually holds had never been measured.

Instrumented with a new `Heap::used`, the high-water mark over 30 blocks of 501
transactions is 251 KB on clob and 290 KB on aave. The reservation is 260 times
the footprint.

This costs time nowhere, since only the touched pages ever fault in. It costs
64 MiB of address space per block executed concurrently, and it is the reason
the arena cannot grow: a bump arena that reallocates would move objects that
transactions hold raw pointers into. The fix is the linked-list-of-pages arena,
which would let the initial reservation drop to a few hundred kilobytes and grow
by chaining rather than by moving. `Heap::used` is kept; the probe that printed
it was temporary.

## E7: stop hashing storage keys with SipHash

Following the measurement above. `UnsyncMap` holds the four per-block maps that
the sequential executor probes on every state access: `resource_map`,
`group_cache`, `delayed_field_map`, and `groups`. All four were plain
`std::collections::HashMap`, so all four hashed with `RandomState`, which is
SipHash-1-3.

SipHash is the right default for a map whose keys might be attacker-chosen short
strings. These keys are neither short nor attacker-chosen in that sense. A
resource key is a 32-byte address plus an 8-byte interned type pointer. A table
item adds a 32-byte handle and the serialized key bytes. Forty to sixty-five
bytes per probe, three probes per cold read.

The change is a type alias and a `default()` instead of `new()`:

```rust
type FastMap<K, V> = HashMap<K, V, ahash::RandomState>;
```

ahash is still keyed and still randomly seeded per process, so the two
properties that matter are preserved. It stays resistant to collision flooding
from user-controlled storage keys, unlike a plain multiplicative hash such as
FxHash. And iteration order stays nondeterministic across nodes, exactly as it
already was with `RandomState`, so nothing that was previously
iteration-order-dependent becomes newly so.

| workload | before | after | mono TPS |
|---|---|---|---|
| clob | 7.16x | **7.25x** | 13759 to 13960 |
| aave | 6.37x | **6.48x** | 13014 to 13331 |

Five reps interleaved, best of five, legacy arms agreeing to within 0.6%. Kept.

The gain is small and that is expected: the cost table predicted about 0.6% of
ratio from halving mono's SipHash, and this delivers 1.2% on clob and 1.7% on
aave because ahash is more than twice as fast on keys this long. What it does
not fix is the probing itself, which is a comparable cost and needs the keys
interned rather than hashed faster.

## What 10x actually requires

The bucket table above says where the CPU goes. It does not say which savings
move the ratio, and those are not the same question.

Mono spends 77.82 us of executor-thread CPU per transaction, legacy 557.61.
For a 10x ratio mono has to reach 55.76 us. Two ways to get there, with very
different arithmetic.

Cut something only mono pays. Then legacy is unchanged and mono needs to drop
Y = 22.06 us.

Cut something both pay. Then both sides drop by X and the ratio is
`(557.61 - X) / (77.82 - X) = 10`, which gives X = 24.51 us. That is more than
mono's entire shared-infrastructure spend — allocator, memory traffic, storage
engine, metrics, state view and thread synchronization together are 26.9 us, and
zeroing all of them is not on offer. Shared work cannot deliver 10x on its own.
Worse, shared savings help legacy proportionally more, because legacy pays more
for the same work.

So the target is 22.06 us of mono-only cost. Here is the honest inventory of
what has been identified and sized so far.

| item | estimate | mono-only? |
|---|---|---|
| interned-type to `StructTag` cache (E8) | 1.8 us | yes |
| loader warm-path memo (E9) | 1.0-1.5 us | yes |
| hashbrown probing in the resource read-write set | 2-3 us | yes |
| heap zeroing on allocation | 0.5 us | yes |
| resource-group merge in materialization | 1.5 us | partly |
| `get_state_slot` prometheus timer (E10) | 3.3 us | no, shared |
| **identified total** | **~10-11 us** | |

That lands mono around 67 us, or 8.3x. The remaining 11 us has to come from the
interpreter or from a structural change, and neither is a tuning exercise.

### Structural split of mono's 77.82 us

Measured with an exclusive partition below the `MonoTransactionExecutor` frame,
so the buckets sum to the thread.

| region | us/txn | share |
|---|---|---|
| execute the payload | 61.12 | 78.5% |
| materialize the output | 9.57 | 12.3% |
| outside the per-transaction boundary | 5.90 | 7.6% |
| unmatched | 1.22 | 1.6% |

Per-transaction setup is not a cost. `production_natives` is a lazy static and
measures 0.00. So do `get_usage` and `execution_guard`. `InterpreterContext::new`
is 0.89 us and `StateViewModuleProvider` construction 1.34 us. There is no
fixed per-transaction overhead worth removing.

Inside materialization, using the same method:

| region | us/txn |
|---|---|
| resource-group merge | 2.93 |
| value serialization | 1.63 |
| state key construction | 1.62 |
| unmatched | 1.55 |
| allocation | 1.42 |
| hashing | 0.32 |
| memory ops | 0.11 |

### Coarse subtrees against legacy

Inclusive cost of named subtrees, both arms, same blocks.

| subtree | mono | legacy | ratio |
|---|---|---|---|
| interpreter | 57.25 | 437.95 | 7.65x |
| natives | 13.96 | 66.26 | 4.75x |
| storage read | 11.00 | 54.45 | 4.95x |
| materialization | 10.20 | 36.78 | 3.61x |
| loader | 7.95 | 4.75 | 0.60x |

Materialization at 3.61x is the worst ratio and it is not a defect. BCS
serialization and resource-group re-encoding are representation-independent:
both VMs produce the same bytes by the same rules, so mono has no structural
advantage. Only the 1.62 us of state key construction is genuinely avoidable,
and that is E8.

The loader is the real defect. Mono is absolutely slower than legacy at 7.95 us
against 4.75, with a warm module cache. Self-cost bookkeeping accounts for
2.39 us of it: `load_function` 1.18, `charge_non_read_set_slots` 0.57,
`meter_once` 0.44, `get_loaded` 0.20. The rest is hashing and allocation
underneath, plus roughly 1.2 us of verifier and specializer. The warm path
re-walks the whole mandatory-dependency list on every call and does one hash
lookup per dependency, almost always to learn that everything is already metered
and charge zero.

## E8: stop rebuilding storage tags

Every resource read and every write-set entry needs the type's `StructTag`, and
mono built a fresh one each time. `struct_tag_of` walks the interned type graph
and allocates two `Identifier`s and a `Vec<TypeTag>`. Each `Identifier::new`
runs `move_core_types::identifier::is_valid` over the string, which alone was
1.33 us per transaction of self cost — the tenth most expensive symbol on the
thread. The tag is then handed to `StateKey::resource`, which looks it up in the
global key registry and drops it.

Two `TODO(perf)` notes already asked for this, one on `type_tag_of` and one on
the free `nominal_tag` helper. Both are now removed.

The cache is a `DashMap<InternedType, Arc<StructTag>>` on `Context`, reached
through a new `StructTagProvider` trait that `ExecutionGuard` implements. The
trait lives in `mono-move-core` and the implementation in
`mono-move-global-context`, which is the only direction that does not create a
dependency cycle. Every caller that used the free function now goes through the
guard: the two storage providers, `drain_write_set`, and
`materialize_storage_key`.

The cache has to be cleared on arena reset. `InternedType` is a pointer into the
global type arena, so after a reset the same address can name a different type.
`MaintenanceGuard::reset_all_caches` clears it alongside the layout table. That
is also why the cache cannot be a `static` or a thread-local in core — it has to
be reachable from the maintenance path.

| workload | before | after | mono TPS |
|---|---|---|---|
| clob | 7.27x | **7.48x** | 14014 to 14241 |
| aave | 6.51x | 6.51x | 13355 to 13512 |

Five reps interleaved, best of five. Read the mono TPS column, not the ratio:
E8 changes no legacy code, and the aave ratio is flat only because that arm's
legacy run happened to come in 1.2% faster. The saving is 1.14 us per
transaction on clob and 0.89 on aave.

Predicted 1.8 us, delivered 1.14. The gap is the part of the region that E8 does
not remove: `StateKey::resource` still takes a read lock on the global key
registry and probes it on every call. Caching the `StateKey` itself rather than
the tag would take that too, but a `StateKey` is not valid across blocks the way
a tag is, so it needs a different lifetime story.

## E9: memoize fully charged dependency slices (reverted)

`Loader::charge_non_read_set_slots` re-walks a function's whole mandatory
dependency list on every call, doing one map probe per dependency. On a warm
cache every probe returns `AlreadyMetered` and charges zero, so the walk is
pure overhead after the first time.

E9 memoized it. `ModuleReadSet` gained a second map keyed on the dependency
slice's address, and `charge_non_read_set_slots` returned early when the slice
was already there. Skipping is observationally identical: `GasMeter::charge`
is `remaining.checked_sub(amount)` so charging zero cannot fail, module state
only moves towards metered, and read-set entries are never removed. Retaining
the `Arc<[LoadedModuleSlot]>` in the memo keeps the address from naming a
different slice.

It measured slower on both workloads. Legacy was flat across the three arms
(1951 / 1943 / 1957 TPS on clob, 2092 / 2087 / 2094 on aave), so the machine
was steady and the mono column is the signal.

| workload | E8 | E9 | delta |
|---|---|---|---|
| clob | 14394 | 14095 | -2.1% |
| aave | 13604 | 13135 | -3.4% |

Reverted. The ceiling was always small — `charge_non_read_set_slots` has 0.53
us of self cost and `meter_once` 0.43 — and the memo added a second
`FxHashMap` to a struct built fresh per transaction, an `Arc` clone and drop
per distinct slice, and a hash and probe on every call. A dependency slice is
usually two or three entries, so the memo probe costs about what the walk it
replaces costs, and then it also has to insert.

The lesson generalizes: a memo only pays when the work it skips is larger than
a hash lookup. Below that, do the work.

## The full partition, and why 10x needs the interpreter

Every sample on the executor thread classified into exactly one bucket by its
leaf symbol, CPU-weighted, clob. The buckets sum to the thread total, so this
is a partition and not a set of overlapping regions. Measured on the E5 build,
which predates E7 and E8, so the hashing and state key rows are now smaller
than shown.

| bucket | mono us | mono % | legacy us | legacy % |
|---|---|---|---|---|
| interpreter / legacy VM core | 18.32 | 23.5 | 241.45 | 43.3 |
| unclassified | 10.33 | 13.3 | 126.50 | 22.7 |
| memory traffic | 6.78 | 8.7 | 22.48 | 4.0 |
| storage backend | 6.60 | 8.5 | 13.21 | 2.4 |
| allocator | 5.94 | 7.6 | 45.91 | 8.2 |
| hashing | 5.53 | 7.1 | 36.06 | 6.5 |
| loader | 4.01 | 5.2 | 1.52 | 0.3 |
| hash probing | 3.89 | 5.0 | 16.26 | 2.9 |
| instrumentation | 3.23 | 4.2 | 8.18 | 1.5 |
| natives | 2.91 | 3.7 | 0.09 | 0.0 |
| materialize | 2.56 | 3.3 | 5.44 | 1.0 |
| state key | 2.27 | 2.9 | 8.15 | 1.5 |
| block executor | 1.82 | 2.3 | 27.11 | 4.9 |
| thread sync | 1.39 | 1.8 | 5.26 | 0.9 |
| mono storage | 1.02 | 1.3 | - | - |
| global context | 0.83 | 1.1 | - | - |

Legacy's `natives` row is near zero because its native functions inline into
the `move_vm_runtime` symbols that land in the VM core bucket. Legacy's
`unclassified` is mostly `drop_glue` at 37.7 us and the gas meter at 20.6 us
across `charge_simple_instr`, `charge_create_ty`, `charge_copy_loc` and
`charge_move_loc`.

### Removing a shared bucket can make the ratio worse

For a bucket that both VMs pay, the new ratio after removing it is
`(557.61 - L) / (77.82 - M)`. That only improves on 7.17x when mono's share of
the bucket is larger than its share of the total.

| bucket removed from both | new ratio |
|---|---|
| storage backend | 7.64x |
| memory traffic | 7.53x |
| instrumentation | 7.37x |
| hash probing | 7.32x |
| thread sync | 7.23x |
| hashing | 7.21x |
| allocator | **7.12x** |

Deleting the allocator entirely from both VMs would *lower* the ratio. Legacy
allocates 7.7x more than mono, which is more than the overall 7.17x, so the
allocator is currently helping mono's ratio. The same is nearly true of
hashing. This is the trap in "optimize the biggest bucket": on a ratio metric,
shared work is only worth attacking where mono's share exceeds legacy's.

### The state view is a fixed cost

`CachedStateView` inclusive is **11.00 us/txn in mono and 13.46 in legacy**.
Both VMs read the same state, so the DB path costs the same absolute amount no
matter how fast the VM gets. It is 14.1% of mono's budget and 2.4% of legacy's.
Underneath it, rocksdb plus LZ4 plus the schema layer is 6.70 us in mono and
14.31 in legacy.

This is Amdahl's law arriving. Mono has already made the VM part roughly 16x
faster; what is left is increasingly the part that does not move.

### The arithmetic

Mono is at 77.82 us of executor-thread CPU against legacy's 557.61, an on-thread
ratio of 7.17x. Reaching 10x means mono at 55.76 us, so 22.06 us has to go.

Optimistic ceilings on everything outside `InterpreterContext::run`:

| item | best case saving |
|---|---|
| memory traffic beyond interpreter copies | 3.6 |
| instrumentation (E10, shared) | 3.2 |
| loader down to legacy's absolute cost | 2.5 |
| allocator | 2.0 |
| hashing after E7 | 2.0 |
| natives' own code | 1.5 |
| hash probing | 1.5 |
| state keys after E8 (E13) | 1.5 |
| **total** | **17.8** |

That is short of 22.06 before discounting any of it, and several entries are
shared with legacy so they return less than face value on the ratio. The state
view's 11.00 us cannot be touched from inside the VM, and materialization's
2.56 us is BCS, which is representation-independent.

**10x on these workloads is not reachable without making `InterpreterContext::run`
itself faster.** It is 18.32 us of leaf self cost, 23.5% of the thread and the
largest single bucket. Everything else is worth roughly 1x of ratio in total.

### Where the interpreter's own time goes

Restricting to samples under `InterpreterContext::run` — 55.07 us/txn, 70.8% of
the thread — the self-cost leaders are:

| symbol | us/txn | share of region |
|---|---|---|
| `InterpreterContext::run` | 15.30 | 27.8% |
| `_platform_memmove` | 3.06 | 5.6% |
| `mach_absolute_time` | 2.86 | 5.2% |
| `Loader::load_function` | 1.18 | 2.1% |
| `hashbrown::make_hash` | 1.11 | 2.0% |
| `LZ4_decompress_safe_continue` | 1.11 | 2.0% |
| `NodeRef::get_strong` | 1.10 | 2.0% |
| `RawTable::find` | 0.99 | 1.8% |
| `_rjem_malloc` | 0.97 | 1.8% |
| `_platform_memset` | 0.90 | 1.6% |

`run` at 15.30 us is the dispatch loop plus every inlined opcode handler. The
named opcode helpers that did not inline are small: `exec_int_bit_and` 0.69,
`exec_int_cast` 0.58, `int_cmp_bool` 0.57.

## E10: state-view instrumentation

`mach_absolute_time` showing up as the third-largest self cost *inside the
interpreter* was not something the earlier profiles had named. Walking its
callers, essentially all of it is `CachedStateView::get_state_slot`'s prometheus
timer and counter, reached through the resource read path:

```
mach_absolute_time <- Timespec::now <- cached_state_view <- BlockSTMStorage <- get_or_create_resource_entry
mach_absolute_time <- Timespec::now <- LocalKey::inc_with_by <- cached_state_view <- BlockSTMStorage
```

Inclusive cost of the timer and the thread-local metric aggregation together:

| | mono | legacy |
|---|---|---|
| `Timespec::now` | 3.11 | - |
| `inc_with_by` + `observe_with` | 2.27 | - |
| combined | **3.76** | **9.26** |

Removing it from both would move 7.17x to 7.40x. This is shared production
observability in `storage/storage-interface/`, two `Instant::now()` per state
read, so it is reported rather than removed.

## E11: stream the write set instead of building a map

`ExecutorTask::Output::resource_write_set` returns `HashMap<Key, Value>`. The
sequential executor's only use of it is to drain it into the multi-version map:

```rust
for (key, value) in output.resource_write_set().into_iter() {
    unsync_map.write(key, value);
}
```

So every transaction allocated a `HashMap`, hashed each key into it, hashed each
key again on drain, then dropped the table. On the mono side the map was built
from `writes_unordered()`, which is already a flat iteration over the read-write
set, so the map existed only to be taken apart.

The change adds a default method to the trait:

```rust
fn for_each_resource_write(&self, callback: &mut dyn FnMut(Self::Key, Self::Value)) {
    for (key, value) in self.resource_write_set() {
        callback(key, value);
    }
}
```

The default keeps every existing implementation working unchanged. Mono
overrides it to walk `writes_unordered()` directly, and inverts its own
`resource_write_set` to go through the callback so the two cannot drift. The
sequential executor calls the new method.

Measured against `bin-e8` after the E9 revert, 5 reps:

| arm | clob legacy | clob mono | ratio | aave legacy | aave mono | ratio |
|---|---|---|---|---|---|---|
| e8 | 1954 | 14288 | 7.31x | 2091 | 13622 | 6.51x |
| e11 | 1952 | 14833 | **7.60x** | 2093 | 13869 | **6.63x** |

+3.8% on clob and +1.8% on aave. Legacy is flat across arms, so the mono column
is trustworthy. Clob gains more because it writes more resources per
transaction, which is exactly the quantity this removes work per unit of.

Note that the earlier E11 number, 7.37x/6.42x, was measured on top of E9 and so
carried E9's regression. E11's own effect is the table above.

## The dispatch loop: how many micro-ops does a transaction run?

The partition says 10x is unreachable without making `InterpreterContext::run`
faster, and `run`'s own self cost is 15.30 us/txn. Whether that is slow depends
on a number no profile reports: micro-ops dispatched per transaction. At 30k ops
a transaction that is about 1.7 cycles per op and there is nothing to win. At 3k
it is about 17 cycles per op and there is a lot.

`size_of::<MicroOp>()` is 48 bytes, asserted at
`core/src/instruction/mod.rs:2519`. That is large for a bytecode instruction — a
64-byte cache line holds 1.3 of them — so if the op count is high, instruction
fetch is a plausible suspect.

Counting is a five-line patch: an `AtomicU64` bumped at the top of the loop
body, accumulated once per `run`. It has to sit at the top rather than next to
`regs.pc += 1`, because the `continue` arms skip the increment.

### The answer: about 20 cycles per micro-op

A transaction is three `run` calls: prologue, payload, epilogue. Measured over
the 14030-transaction replay:

| | ops / txn | `run` self, us/txn | cycles / op |
|---|---|---|---|
| clob | ~3545 | 14.49 | **18.4** |
| aave | ~2192 | 10.90 | **22.4** |

The `run` self costs come from a fresh profile of `bin-e11b`, scaled by the
ratio of the A/B TPS to the profiled thread total so the sampling overhead is
removed (clob 67.4/74.89, aave 72.1/77.42). Cycles at the M4 Max P-core's
4.512 GHz.

Two workloads with different op mixes land within 20% of each other. That is the
signature of a cost paid per dispatch rather than per unit of work, and 20
cycles is high: a tuned switch interpreter runs 5 to 10.

### What the dispatch loop compiles to

Disassembling `bin-e11b` at the loop's back edge:

```
add  x26, x26, #1        ; pc += 1
ldr  x24, [x20, #0x68]   ; code.len(), reloaded from func every iteration
cmp  x26, x24
b.hs <out of bounds>
ldr  x8,  [x20, #0x60]   ; code.ptr, reloaded from func every iteration
madd x1,  x26, x22, x8   ; instr = ptr + pc * 48
ldrb w2,  [x1, #0x28]    ; discriminant, at byte 40 of the 48-byte MicroOp
sub  w8,  w2, #6
cmp  w2,  #5
mov  w9,  #0x34
csel w8,  w8, w9, hi
and  x8,  x8, #0xff
ldrh w10, [x11, x8, lsl #1]   ; 16-bit offset table
add  x9,  x9, w10, lsl #2
br   x9                       ; one indirect branch for all 84 variants
```

The chain from `pc += 1` to a known branch target is: `madd` (3) + `ldrb` (4) +
four ALU ops (3) + `ldrh` (4) = about 14 cycles. The two `ldr`s off `func` are
loop-invariant, so an out-of-order core issues them early and they are not on
the chain.

Fourteen cycles of latency only costs 14 cycles per op if the CPU cannot run
ahead. It can, provided the indirect branch is predicted. Measured 18 to 22
cycles per op against a ~14-cycle chain says the predictor is missing often:
one `br` site with 84 targets is the classic interpreter dispatch bottleneck.

Good news for a different hypothesis: `pc` is in `x26` and `fp` in `x27`, so
`regs` is not spilled to the stack despite `&mut regs` being passed to a dozen
helpers. Those helpers are all `#[inline(always)]`.

Also, `size_of::<MicroOp>() == 48` is not the problem. The dynamic instruction
stream is 3545 x 48 = 170 KB per transaction; L1 delivers that in about 2700
cycles, under one cycle per op. Shrinking the op to 32 bytes would buy about
0.3 cycles of the 18. The 48-byte size is worth fixing for footprint, not for
speed.

### Where that leaves the plan

Two levers, in order of size:

1. **Fewer dispatches.** Fusing common fall-through pairs into single ops cuts
   dispatch cost proportionally. This is the large one and it needs to know
   which pairs actually occur, so the next step is a dynamic pair histogram.
2. **A shorter chain per dispatch.** Hoisting `code.ptr` / `code.len` out of the
   loop, and replacing the `pc * 48` index with a pointer cursor, removes the
   `madd` and one load. Worth a few cycles when the branch does mispredict,
   nothing when it does not.

Replicated dispatch — a copy of the dispatch sequence at the end of every arm,
so the predictor sees one site per opcode — is the standard fix for the
mispredicts themselves. It needs computed goto or guaranteed tail calls, and
Rust has neither on stable, so it is not available here.

## The micro-op histogram: a third of all ops are frame-to-frame moves

The pair histogram from the previous section, run over both workloads. The
counter sits at the top of the loop body and records the current op plus its
fall-through predecessor. A pair is counted only when the previous op fell
through, since only those are candidates for fusion. Nested `run` calls (a
native re-entering the interpreter) hold no lock and are not counted.

| | ops per `run` | ops per txn |
|---|---|---|
| clob | 1191.8 | ~3575 |
| aave | 733.9 | ~2202 |

### clob

Top single ops, as a share of all dispatches:

| share | op |
|---|---|
| 15.33% | `Move8` |
| 14.90% | `Move(16)` |
| 8.99% | `Return` |
| 8.91% | `CallIndirect` |
| 6.08% | `IntCast.u64->u8` |
| 4.13% | `IntBitAnd` |
| 4.13% | `StoreImm1` |
| 3.97% | `DeriveRefOffsetImm` |
| 3.67% | `StoreImm8` |
| 2.74% | `ReadRefOffset` |
| 2.60% | `JumpZeroByte` |
| 2.38% | `CallNative` |
| 2.31% | `JumpIntCmp` |
| 2.14% | `IntShr.u128` |
| 2.06% | `Jump` |

Top fall-through pairs:

| share | pair |
|---|---|
| 4.42% | `Move(16) -> Return` |
| 4.20% | `Move8 -> Move(16)` |
| 4.00% | `Move(16) -> CallIndirect` |
| 3.58% | `Move8 -> Return` |
| 3.56% | `Move8 -> CallIndirect` |
| 3.00% | `StoreImm1 -> StoreImm8` |
| 2.93% | `IntBitAnd -> IntCast.u64->u8` |
| 2.30% | `Move(16) -> CallNative` |
| 2.14% | `IntShr.u128 -> IntCast.u64->u8` |
| 2.14% | `IntCast.u64->u8 -> Move8` |
| 2.13% | `IntCast.u64->u8 -> IntBitAnd` |
| 1.92% | `DeriveRefOffsetImm -> Move(16)` |
| 1.78% | `DeriveRefOffsetImm -> Move8` |
| 1.69% | `CallNative -> DeriveRefOffsetImm` |

### aave

| share | op |
|---|---|
| 24.13% | `Move(16)` |
| 9.35% | `Return` |
| 9.21% | `CallIndirect` |
| 5.98% | `ReadRefOffset` |
| 5.18% | `Move8` |
| 4.43% | `SlotBorrow` |
| 3.97% | `JumpZeroByte` |
| 3.96% | `JumpIntCmp` |
| 2.95% | `CallNative` |
| 2.64% | `StoreImm32` |
| 2.44% | `Jump` |
| 1.79% | `IntCast.u64->u8` |

| share | pair |
|---|---|
| 7.19% | `Move(16) -> Return` |
| 5.93% | `Move(16) -> Move(16)` |
| 4.90% | `Move(16) -> CallIndirect` |
| 2.00% | `Move8 -> Move(16)` |
| 1.76% | `ReadRefOffset -> Move(16)` |
| 1.74% | `Move(16) -> CallNative` |
| 1.71% | `SlotBorrow -> CallIndirect` |
| 1.11% | `Move(16) -> SlotBorrow` |

### Reading it

Moves dominate both workloads. clob spends 30.2% of all dispatches on `Move8`
plus `Move(16)`; aave spends 29.3%. Neither workload does anything with those
bytes — a move copies a value from one frame slot to another.

`Move(16)` alone is about 506 ops per transaction in both workloads, and 16
bytes is the width of a reference: a fat pointer, `(base, offset)`. So the
16-byte moves are references being marshalled, and the dominant successors say
where they are going. `Move(16) -> CallIndirect` and `SlotBorrow -> CallIndirect`
are argument setup. `Move(16) -> Return` is the return-value shuffle.

Every one of those 506 moves went through `std::ptr::copy`, which for a
non-constant size is a call to the platform `memmove`. At roughly 25 cycles of
call and dispatch overhead that is about 12,650 cycles, or 2.8 us per
transaction. The blame analysis measured 2.766 us/txn of memmove reachable from
`InterpreterContext::run`. The two numbers agree, which is a good sign the
model is right.

That gives two independent levers:

1. **Make each move cheap.** A width-specialized copy inlines 8- and 16-byte
   moves into two instructions and never calls out. This is E13 and it does not
   need the specializer to change.
2. **Do not move at all.** Better copy propagation and slot allocation would let
   a borrow or a computed value land directly in the slot its consumer reads,
   removing the op entirely. Removing 10 to 15% of all dispatches is worth about
   5 us/txn. This is the larger lever and it is not done.

## E12: size the module read-set once instead of growing it from empty

`under.py 'mono_move_loader'` on aave charges 10.39 us/txn to the loader,
against legacy's whole loader bucket of 1.52 us. Self costs inside it:

| us/txn | symbol |
|---|---|
| 1.629 | `load_function` |
| 1.156 | `meter_once` |
| 0.969 | `hashbrown::RawTable::reserve_rehash` |
| 0.891 | `charge_non_read_set_slots` |
| 0.673 | `HashMap::insert` |
| 0.459 | `get_loaded` |

The top six are all read-set machinery, 5.78 us of the 10.39. Everything below
them is a long tail under 0.25 us — verifier, specializer, bcs, LZ4 — which is
one-time compilation amortized across the block and not worth attacking.

The `reserve_rehash` is not one big allocation. The read-set starts empty every
transaction and hashbrown grows geometrically, so a transaction touching ~50
modules rehashes at 3, 7, 14 and 28 entries and moves about 52 elements on the
way. That is roughly 0.97 us, which matches.

The fix is two lines. `ModuleReadSet::new` starts at `INITIAL_CAPACITY = 48`,
and `record_loaded_and_charge_slots` reserves
`slots.len().saturating_sub(read_set.len())` rather than `slots.len()`. The
saturating form matters: a dependency slice averages two or three entries and
a warm read-set already holds them all, so reserving `slots.len()`
unconditionally would double the table on every call once the read-set is
larger than the slice.

Note this leaves `ModuleReadSet::default()` giving a zero-capacity map, which
`new()` no longer does. Nothing on the production path calls `default()`.

## E13: inline the moves the interpreter actually makes

`copy_bytes` in `core/src/memory.rs` dispatches on size and handles 4, 8, 16
and 32 bytes with unaligned loads and stores, falling back to `std::ptr::copy`
otherwise. Each specialized arm reads its whole source into registers before
writing, so it stays correct when source and destination overlap — which
matters, because the return-value shuffle can move results within one frame
region.

Five call sites switch to it, chosen from the histogram: `Move`, `ReadRef`,
`WriteRef`, `ReadRefOffset`, `WriteRefOffset`. Between them they cover the
16-byte moves, the 24% figure on aave, and the 5.98% of aave dispatches that
are `ReadRefOffset`.

`Move8` already had its own path and is untouched.

## E14: size the serialization buffer for the values it actually sees

`value_utils::serialize` allocated a `Vec` from empty and grew it. Its callers
are table keys and native arguments, all small. Starting at
`Vec::with_capacity(32)` avoids the growth for everything that fits. The blame
analysis charged 0.208 us/txn to `RawVec` growth under `value_utils::serialize`
on clob.

An exact-size variant is available — `fixed_serialized_size` returns `Some(n)`
for fixed-width layouts — but calling it costs a layout walk per serialize, so
a fixed 32 bytes is the better trade until the layout size is memoized.

## What E12 to E14 actually bought

The three changes shipped together, so a fresh profile of `bin-e13` attributes
them. Both numbers below are profiled us/txn, and the profiled thread total fell
from 74.89 to 71.13 on clob and 77.42 to 75.04 on aave.

E13 landed where it was aimed. `memmove` reached from `InterpreterContext::run`
went from 2.766 to **0.358** us/txn on clob, an 87% drop, and the whole
memmove-leaf bucket fell from 6.22 to 2.74. On aave the memory-traffic bucket
went 8.29 to 5.13. The work did not vanish, it moved: the interpreter bucket
rose on aave from 14.26 to 16.08 as the copies became inline instructions inside
`run` instead of calls out to the platform. Net of both, clob's dispatch-plus-copy
cost went from 16.26 to 14.75 real us/txn.

E12 landed too. `reserve_rehash` dropped out of the loader's top costs, and the
loader subtree on aave went from 10.39 to 9.09 us/txn.

The rest of the A/B win is E14 and second-order effects.

## The state read path is 20% of a transaction

`under.py 'BlockSTMSequentialProvider::get_resource'` gives the cold half of
global storage — everything mono does on a first touch of a key, including the
Block-STM view, the state view, and RocksDB:

| | clob | aave |
|---|---|---|
| inclusive | 13.63 us/txn (19.2%) | 15.28 us/txn (20.4%) |

For contrast, the warm half is cheap. `get_or_create_resource_entry` is 17.13
us/txn inclusive on clob, so only 3.5 us of it is the per-transaction `entries`
map — hashing and probing a key that is already there.

The cold path has no hot spot. It is 30-plus symbols, none above 0.6 us, in the
shape you would expect: layered-map node traversal, a DashMap probe, the
sharded state cache, RocksDB block iteration, LZ4 block decompression, hashing
`StructTag`, cloning `StateSlot`. That is real work and it is shared with the
legacy VM.

With one exception.

### 22% of the read path is a stopwatch

The largest single symbol under `get_resource` is `mach_absolute_time`:
**2.891 us/txn on clob and 3.671 on aave**, plus 0.140 of `mach_timebase_info`.
Every one of those samples comes from one line,
`storage/storage-interface/src/state_store/state_view/cached_state_view.rs:287`:

```rust
fn get_state_slot(&self, state_key: &StateKey) -> StateViewResult<StateSlot> {
    let _timer = TIMER.timer_with(&["get_state_value"]);
    COUNTER.inc_with(&["sv_total_get"]);
```

A `HistogramTimer` reads the clock on construction and again on drop. At roughly
40 ns a pair, 2.9 us/txn means mono issues on the order of 70 state-view reads
per transaction. The label lookup and the histogram observe are cheap by
comparison — `blame.py` charges only 0.214 us/txn to `inc_with` and
`timer_with` themselves.

The same profile on the legacy VM charges 7.018 us/txn (clob) and 7.749 (aave)
to `mach_absolute_time`. So legacy reads state about 2.4x more often than mono
does, and pays 2.4x more for the stopwatch. Both VMs pay it; mono's total is
10x smaller, so the same absolute cost is 7x more of mono's budget.

This is not a mono-move change to make. It is an aptos-core storage metric, and
removing it removes observability. It is recorded here because it sets a ceiling:
about 3 us/txn of mono's remaining budget is spent measuring, not executing.
E16 below measures exactly how much.

## E15: pre-size the resource-group blob (reverted, no measurable gain)

`merge_group` encodes a group's members with `bcs::to_bytes`, which starts from
an empty `Vec` and doubles. A group runs to several kilobytes on aave, so every
doubling copies the whole blob. The change computed a loose upper bound on the
encoding and allocated once.

It bought nothing the sweep could resolve: clob 14981 to 15018 TPS (+0.25%),
aave 14286 to 14338 (+0.36%), against a rep-to-rep spread of 1.7% and 2.1%. The
change is reverted.

The prediction was wrong because the attribution was wrong. The earlier reading
charged about 3.1 us/txn of aave's group write path to buffer growth. Re-running
`blame.py 'ralloc|finish_grow|memmove|realloc'` and reading the callers, not the
total, splits that:

| blamed on | aave us/txn | what it is |
|---|---|---|
| `bcs::ser::Serializer::serialize_bytes` | 1.092 | copying member values into the buffer |
| `StructTag::serialize` | 0.801 | copying tag identifiers into the buffer |
| `merge_group` itself | 0.765 | the reallocation |

Only the last row is removable, and pre-sizing spends part of it walking the map
to compute the bound. The first two are `extend_from_slice` on a buffer that
already has room — the bytes have to be copied somewhere, so they are the
irreducible cost of producing the blob.

Lesson for the next attribution: a `memmove` leaf under a serializer is usually
the serializer doing its job, not a `Vec` resizing. Blame by caller before
believing a growth number.

## E16: how much the state-view stopwatch actually costs

A measurement arm, not a proposed change. Same binary as E15 with the two
instrumentation lines in `CachedStateView::get_state_slot` commented out. The
source file is untouched in the tree; only the measurement binary carried it.

| | clob | aave |
|---|---|---|
| E15 | 15018 TPS (66.59 us/txn) | 14338 TPS (69.75 us/txn) |
| E16 | 15347 TPS (65.16 us/txn) | 14597 TPS (68.51 us/txn) |
| delta | **-1.43 us/txn**, +2.2% | **-1.24 us/txn**, +1.8% |

Legacy read 1952/1959/1953 on clob and 2089/2094/2099 on aave across the three
arms, so the control is flat to within 0.5% and the mono column is trustworthy.

The profile said 2.9 and 3.7 us/txn; the A/B says 1.4 and 1.2. The profile
over-states it by about 2x, which is the expected direction: `mach_absolute_time`
is a short leaf called at a fixed point in a hot path, exactly the shape that
collects more samples than its retired-cycle share. Take the A/B number.

So the honest figure is that **2% of a mono transaction is the storage layer
timing itself**. Worth raising with the storage owners as a node-wide cost — it
is 1.4 us/txn on a path both VMs run — but it is not mono's to change and it is
not on the path to 10x.

## E15 and E16 in the scoreboard

Neither is a kept change. E15 reverted; E16 is a measurement. The current tree
is still E12+E13+E14, which the three-arm sweep re-read as 7.67x clob and 6.84x
aave.

## E17: fuse runs of moves into one dispatch

The last interpreter experiment before the pivot below.

The op histogram says moves are 30.2% of clob dispatches and 29.3% of aave's.
They are not scattered: call arguments and return values are marshalled as a run
of adjacent frame-slot copies. `Move -> Return` pairs are 8.0% of clob
dispatches and 7.19% of aave's, `Move -> Call` pairs 9.86% and 6.64%.

Three new micro-ops carry more than one copy per dispatch:

| op | payload | emitted by |
|---|---|---|
| `Move2` | two `SlotCopy` | argument runs, return-value runs |
| `Move3` | three `SlotCopy` | the same, when the run is longer |
| `ReturnMove` | one copy, then return | the last copy before a `Ret` |

`SlotCopy` is `{dst, src, size}`, 12 bytes. `Move3` carries 36 of the 40 bytes a
`MicroOp` payload has, so the 48-byte `MicroOp` budget still holds. The
`micro_op_size` test covers it.

Fusing happens at emission, not in a post-pass. `translate.rs::emit` records
safe points and branch fixups against `out_buf.len()`, so an op removed after
emission would need every later pc renumbered. Emitting fused in the first place
needs no renumbering. `emit_parallel_copy` now returns an ordered `Vec<Copy>`
instead of emitting, and `emit_move_run` folds that order into `Move3`/`Move2`,
preserving it exactly. Overlap safety is unchanged: the ordering is the same one
the cycle-breaking pass already produced.

Gas is unaffected. `specializer/src/gas.rs::instrument` computes block costs from
the IR, not from micro-ops, so fusing does not change what a program is charged.

The verifier checks each carried copy exactly as it checks a `Move`.

Cost of landing it: 97 of the 253 differential tests carry `--print` listing
snapshots, and every one of them changed. That is the expected shape of a
compiler change, not a correctness signal, and `UPBL=1` regenerated them. The
suite is 253/253 green.

Measured, four reps interleaved against E13:

| arm | clob legacy | clob mono | ratio | aave legacy | aave mono | ratio |
|---|---|---|---|---|---|---|
| E13 | 1908 | 14713 | 7.71x | 2049 | 14105 | 6.88x |
| E17 | 1913 | 15132 | **7.91x** | 2044 | 14208 | **6.95x** |

Legacy is flat to 0.3%, so the move is real: +2.85% on clob, +0.73% on aave, or
1.88 and 0.52 us/txn. Kept.

The split matches the histogram. clob runs longer argument runs, so more of its
moves fuse; aave's moves are more often isolated and stay single.

This is also the shape of the ceiling. Two years of interpreter work at this rate
would not close the gap, which is the point of the section below.

## The pivot: the VM is not the bottleneck

Direction from the user, and the reason E17 is the last interpreter experiment
in this log: the VM in isolation is already 10-20x once deserialization and
materialization are excluded. End to end the benchmarks read 7.67x and 6.84x.
The loss is outside the VM, so further dispatch work does not pay.

To find where, the same partition was run on the legacy profiles and lined up
against mono's. Legacy's profile carries much more sampling overhead than mono's
(deeper stacks cost more to unwind), so legacy buckets are scaled by
`measured/profiled` = 0.739 for clob and 0.728 for aave, and mono's by 0.938 and
0.933.

| bucket (us/txn) | clob legacy | clob mono | aave legacy | aave mono |
|---|---|---|---|---|
| legacy move vm | 222.9 | 0.32 | 157.5 | 0.38 |
| unclassified | 143.4 | 9.46 | 127.1 | 12.05 |
| allocator | 40.7 | 5.83 | 45.1 | 6.32 |
| hashing | 32.1 | 2.67 | 43.0 | 3.94 |
| block executor | 20.5 | 1.70 | 20.4 | 2.16 |
| memory traffic | 19.1 | 4.39 | 25.5 | 4.79 |
| materialize | 14.3 | 1.81 | 18.1 | 2.08 |
| hash probing | 14.3 | 4.18 | 17.5 | 3.23 |
| state key | 7.35 | 1.72 | 8.15 | 1.60 |
| thread sync | 6.35 | 1.29 | 6.07 | 0.84 |
| instrumentation | 5.39 | 3.05 | 6.02 | 3.88 |
| storage backend | 3.93 | 4.15 | 4.01 | 4.24 |
| loader | 1.23 | 3.67 | 2.28 | 5.86 |
| interpreter + natives | 0 | 19.8 | 0 | 16.5 |

Two readings matter.

The shared floor is small. `storage backend` is the only bucket where the two
VMs cost the same: about 4 us/txn of RocksDB, LayeredMap, and StateSlot work.
Everything else mono already does 3x to 30x cheaper. So Amdahl is not what caps
the ratio at 7.67x; the cap is just mono's own remaining 66.75 us/txn.

The loader is the one place mono is slower than legacy: 3.67 against 1.23 on
clob, 5.86 against 2.28 on aave. Mono builds a loader per transaction.

## Where the remaining time actually goes: a phase map

Inclusive cost of each phase, on the executor thread, scaled as above.

| phase | clob legacy | clob mono | ratio | aave legacy | aave mono | ratio |
|---|---|---|---|---|---|---|
| whole sequential loop | 524.7 | 65.2 | 8.0x | 474.2 | 68.8 | 6.9x |
| one user transaction | 487.0 | 51.4 | 9.5x | 433.6 | 52.9 | 8.2x |
| prologue | 45.7 | ~3.0 | ~15x | 43.4 | ~2.8 | ~15x |
| epilogue | 62.1 | 2.13 | 29x | 53.4 | 1.99 | 27x |
| produce the output | 39.0 | 9.32 | **4.2x** | 46.7 | 11.52 | **4.0x** |
| storage read path | n/a | 16.5 | n/a | n/a | 18.3 | n/a |

Producing the output is where mono's advantage collapses. Everything else runs
at 8x or better; materialization runs at 4x. It is 14% of a clob transaction and
16.5% of an aave one.

The storage read path (`BlockSTMSequentialProvider` plus
`get_or_create_resource_entry`) is 25% and 26% of a mono transaction. It has no
single hot symbol. Under `get_resource` nothing except the stopwatch exceeds
0.6 us/txn across 30-odd symbols.

Breaking the two down by leaf:

| item | clob | aave | note |
|---|---|---|---|
| state-view stopwatch | 3.00 | 3.47 | profiled; A/B says 1.43 and 1.24 |
| RocksDB + LayeredMap + StateSlot | 4.15 | 4.24 | shared with legacy |
| hash probing in the read set | 2.78 | 1.55 | per-transaction `HashMap` |
| loader inside the payload | 2.49 | 2.34 | per-transaction loader |
| allocator under materialize | 1.74 | 2.86 | |
| memmove under materialize | 1.17 | 1.19 | |
| `StructTag`/`TypeTag` re-encode | 0.22 | 0.69 | group blob, every write |
| SipHash in the write set | 0.28 | 0.37 | `std::collections::HashMap` |
| `StructTag::hash` | 0.18 | 0.34 | group member maps |
| `StateKeyInner::cmp` | 0.35 | - | sorting the write set |

## Answering "where does the hashing come from"

The question from the original goal message. Blaming every keccak, sha3, and
SipHash leaf on its caller:

| caller | clob | aave | kind |
|---|---|---|---|
| `mono_move_natives::hash` | - | 1.49 | the workload's own `sha3_256` calls |
| `AuthenticationKey::object_address_from_object` | 0.33 | 1.10 | SHA3 per derived object address |
| `SessionId::hash` | 0.25 | 0.45 | SHA3 once per transaction, for GUIDs |
| `AccountAuthenticator::authentication_proof` | 0.30 | 0.24 | SHA3 in the prologue |
| `StructTag::hash` | 0.26 | 0.32 | map keys |
| `BlockSTMSequentialProvider::get_resource` | 0.18 | 0.23 | new `StateKey` crypto hashes |
| `dashmap::hash_u64` | 0.16 | 0.10 | interner probes |

So it is not one thing. About half is Aptos semantics that both VMs pay: object
address derivation, the session id, the auth key. On aave a third is the
workload calling `sha3_256` itself, which is real work. The mono-specific part is
the last three rows, roughly 0.6 and 0.65 us/txn, and it comes from keying maps
on `StructTag` and `StateKey` instead of on the interned type ids mono already
has.

## The budget after the pivot

10x needs clob at 51.2 us/txn (from 66.75) and aave at 47.9 (from 69.99), so
15.5 and 22.1 us/txn have to come out. The interpreter is off the table. The two
phases named above hold 27.5 and 32.0 us/txn between them, 41% and 43% of a
transaction. Halving them covers 13.7 and 16.0 of the budget, which is most of
clob's and two-thirds of aave's.

## An exclusive partition: 61% of a transaction is not interpretation

The phase table above measures inclusive cost, so the interpreter swallows every
storage read a Move instruction triggers. Partitioning the same profile so the
*innermost* region wins gives the honest split. clob, mono, profiled us/txn on a
71.13 us/txn thread:

| region | us/txn | share |
|---|---|---|
| interpretation proper (unmatched) | 27.57 | 38.8% |
| natives, including reads they trigger | 12.31 | 17.3% |
| read-write set, including reads it triggers | 10.70 | 15.0% |
| materialize | 9.94 | 14.0% |
| loader | 8.12 | 11.4% |
| outside the transaction boundary | 1.61 | 2.3% |
| GC and heap | 0.88 | 1.2% |

Dispatching micro-ops is 39% of the transaction. The other 61% is storage,
output, and loading. That is the pivot restated as a number.

`CachedStateView::get_state_slot` is 9.37 us/txn inclusive, always nested inside
`natives` or `rws`, and splits two ways:

| caller | us/txn |
|---|---|
| `BlockSTMSequentialProvider::get_resource` | 6.24 |
| `BlockSTMSequentialProvider::read_group_from_storage` | 3.12 |

A third of every state-view read is a resource group. clob creates order objects,
each with a fresh `ObjectGroup` slot, so each creation asks storage whether a
group that cannot exist exists.

## The three cross-cutting costs

The same profile, grouped by what the work is rather than where it happens.

| cost | us/txn | share |
|---|---|---|
| jemalloc (malloc, sdallocx, ralloc, cache fill, extent) | 5.66 | 8.0% |
| hash maps (find, insert, make_hash, rehash, SipHash) | 4.64 | 6.5% |
| `memmove` + `memset` | 4.12 | 5.8% |
| `mach_absolute_time` | 2.92 | 4.1% |

Nothing else on the thread except `InterpreterContext::run` (15.45 us/txn self)
exceeds 1.4. There is no single hot symbol left to fix. Every remaining
microsecond comes from allocating, hashing, copying, or reading the clock, spread
across a hundred call sites.

96% of the clock reads are one line: the `TIMER.timer_with` plus `COUNTER.inc_with`
pair at the top of `CachedStateView::get_state_slot`. It is 2.81 us/txn profiled
and 1.43 real, or 30% of the whole state-view cost, on a metric nobody reads
per-call.

## Correction: the per-transaction heap is already cheap here

The parallel branch measured +50% at one thread from pooling session heaps and
stacks, against a baseline that mmap'd 10 MiB per transaction. That win is not
available on this branch and must not be double-counted: E2 already replaced the
fixed 10 MiB heap with a 256 KiB heap that grows, so `mmap` plus `munmap` is
0.20 us/txn here, 0.3% of the transaction. Pooling the remaining 1 MiB stack is
worth at most a few tenths.

## E18: stop cloning the resource group out of the block cache

Three changes on the output path, all aimed at the 9.94 us/txn of materialize.

`merge_group` read the group's stored members with `provider.group_members`,
which deep-clones the whole `BTreeMap<StructTag, Bytes>` out of the block's
`UnsyncMap`. It then mutates the clone and the caller inserts it straight back,
so the clone was pure waste. Every `StructTag` in it costs three allocations. Now
`AptosDataProvider` has a `take_group_members` whose default still clones, and
`BlockSTMSequentialProvider` overrides it with `UnsyncMap::take_group`, a
`remove`. Materialization is the only caller, and it is the only caller that may
take, because it puts the assembled group back.

Second, `group_ops` and `MaterializedGroups` were `std::collections::HashMap`, so
every lookup ran SipHash over a `StateKey` that already carries a crypto digest,
or over a `StructTag`'s strings. They now use `ahash`, the same choice E7 made
for the per-block maps.

Third, `ResourceReadWriteSet::new` returned `Self::default()`, so the read set
started with no capacity and rehashed several times inside every transaction. It
now reserves 64 entries and 16 journal slots, and `Default` goes through `new`
so the two cannot drift. The write vector reserves 16.

Measured, four reps interleaved against E17:

| arm | clob legacy | clob mono | ratio | aave legacy | aave mono | ratio |
|---|---|---|---|---|---|---|
| E17 | 1939 | 15137 | 7.81x | 2082 | 14273 | 6.86x |
| E18 | 1938 | 15522 | **8.01x** | 2075 | 14747 | **7.11x** |

Legacy is flat to 0.3%. clob gains 2.54%, or 1.65 us/txn; aave gains 3.32%, or
2.25 us/txn. Kept. This is the first arm over 8x on clob.

aave gains more than clob, which fits. aave writes more group members per
transaction, so it clones more `StructTag`s per group and hashes more of them.

## E19: drop writes that did not change the value (reverted)

MonoMove marks a resource written as soon as anything mutably borrows it. The
mutable borrow deep-copies the value into the transaction's heap, and the drain
emits that copy whether or not a single byte changed. Commit `22b195ee` recorded
what this costs on `order-book-no-matches1-market`: about 1 180 bytes per
transaction of state MonoMove writes and the legacy VM does not.

The fix is already spelled out by the shape of the data. `StorageRead::ExternalHeap`
holds the pre-transaction pointer, set once at first touch and never updated, and
`StorageWrite::LocalHeap` holds the copy. `value_utils::compare` walks two values
of the same interned type structurally and early-exits on the first difference.
So `ResourceReadWriteSet::drop_unchanged_writes` compares the two, turns the
write back into `NotModified` when they are equal, and leaves it alone when the
comparison errors — function values are the only case it cannot walk. It runs
once, in `InterpreterContext::finish`, after execution can no longer mutate
anything, so both consumers of the write set see the same filtered result: the
`WriteSet` and the block's `UnsyncMap`.

All 1 108 tests pass, including the differential and legacy-comparison suites.
No `.exp` baseline moved, which is the first hint.

Measured, four reps interleaved:

| arm | clob legacy | clob mono | ratio | aave legacy | aave mono | ratio |
|---|---|---|---|---|---|---|
| E18 | 1934 | 15694 | 8.12x | 2075 | 15137 | 7.29x |
| E19 | 1927 | 15478 | 8.03x | 2071 | 14648 | 7.07x |

Legacy is flat to 0.4%. clob loses 1.4%, aave loses 3.0%. Reverted.

The comparison found nothing to drop. Both workloads mutably borrow a resource
only when they mean to change it, so the walk is pure overhead, and it costs
more on aave because aave's resources are larger. That also explains the flat
baselines: a walk that never finds an equal pair saves nothing downstream.

Two things worth keeping from this. First, the 1 180 bytes per transaction in
`22b195ee` are not no-op writes; they are somewhere else, and finding them is a
separate question from making the write set smaller. Second, the general lesson
from E9 applies again, one level up: a filter only pays when the thing it
filters out actually occurs. Measure the hit rate before paying for the test.

## The plumbing budget

After E19 the obvious single-change wins are gone, so this is a re-scope of
where the remaining time is. Classify every profile sample by its leaf symbol
into "plumbing" — allocator, hash tables, `memcpy`/`memset`, clock reads,
SHA3, drop glue, `BTreeMap`, thread-local access — versus everything else.
`plumb.py` does this CPU-weighted on the largest `txn_executor` thread.

| class | clob mono | aave mono | clob legacy |
|---|---|---|---|
| allocator | 7.49 (10.5%) | 8.45 (11.3%) | 73.02 (10.2%) |
| hash tables | 6.30 (8.9%) | 5.73 (7.6%) | 60.35 (8.4%) |
| memcpy/memset | 5.61 (7.9%) | 6.19 (8.3%) | 45.26 (6.3%) |
| clock reads | 3.06 (4.3%) | 3.71 (5.0%) | 7.37 (1.0%) |
| SHA3 | 1.10 (1.5%) | 3.50 (4.7%) | 2.43 (0.3%) |
| drop glue | 0.93 (1.3%) | 1.41 (1.9%) | 35.56 (4.9%) |
| BTreeMap | 1.18 (1.7%) | 0.77 (1.0%) | 22.53 (3.1%) |
| thread-local | 0.76 (1.1%) | 0.76 (1.0%) | 9.20 (1.3%) |
| **total** | **26.42 (37.2%)** | **30.52 (40.7%)** | **255.73 (35.5%)** |
| thread total | 71.13 | 75.04 | 719.59 |

All figures are profiled us/txn; multiply by the arm's scale factor for real
time (0.938 mono clob, 0.933 mono aave, 0.739 legacy clob).

The last column is the point. MonoMove cut the thread from 719.59 to 71.13
us/txn, but the plumbing share is 35.5% before and 37.2% after. Interpretation
got 10-20x faster and the allocate-hash-copy-clock work around it came along
unchanged as a fraction. That is why the end-to-end ratio sits at 8.12x while
the VM measures far higher in isolation.

It also sets the budget. clob needs 51.7 real us/txn for 10x, which is 55.1
profiled, a cut of 16.0 from a 26.42 plumbing budget — 61% of it. aave needs
51.7 profiled from 75.04, a cut of 23.3 from 30.52 — 76%. So clob is reachable
by attacking plumbing alone and aave is not quite.

Where each class comes from on clob, from `blame.py`:

- **hash tables (6.30).** `get_or_create_resource_entry` 2.07, `StateKey`
  interning in `TwoKeyRegistry` 0.89, `BlockSTMSequentialProvider::get_resource`
  0.50, `Loader::charge_non_read_set_slots` 0.41, `UnsyncMap::set_base_value`
  and `write` 0.62. A cold table read hashes the same key three times: the
  transaction's `entries` map, the block's `UnsyncMap`, and the `StateKey`
  registry.
- **allocator (7.49).** One `Vec<u8>` per table-item key in
  `table::handle_and_key`, cloned again for the `entries` insert and again for
  `UnsyncMap::set_base_value`; one exact-size `Bytes` per write in
  `drain_write_set`; `bcs::to_bytes` growth in `merge_group`.
- **clock reads (3.06).** 2.89 of it is `CachedStateView::get_state_slot`, and
  the mechanism is `aptos-metrics-core`: `maybe_flush()` calls `Instant::now()`
  on *every* counter increment, so the function pays five clock reads on a
  memorized hit and seven on a miss. This is 31% of the whole state-read path.
- **SHA3 (1.10 clob, 3.50 aave).** `StateKey::crypto_hash_ref()` for the
  hot-state lookup, plus object address derivation on aave.

E20 onward work this list.

## E20: how much the state-view metrics actually cost

A ceiling measurement, not a shippable change: delete the six `COUNTER.inc_with`
calls and the per-call `TIMER.timer_with(&["get_state_value"])` from
`CachedStateView::get_state_slot` and `get_unmemorized`, leaving the
`prime_state_cache` timer alone. This is E16 repeated after eight arms of other
work, and this time with the mechanism understood.

The mechanism is in `aptos-metrics-core`. Every `ThreadLocal*` metric ends its
update with `maybe_flush()`, which reads `Instant::now()` to see whether a second
has passed since the last flush. So the cost of a counter increment is not the
increment; it is a `mach_absolute_time` call. `get_state_slot` increments two
counters and starts one timer on a memorized hit, which is five clock reads, and
three counters plus the timer on a miss, which is seven.

| arm | clob legacy | clob mono | ratio | aave legacy | aave mono | ratio |
|---|---|---|---|---|---|---|
| E18 | 1930 | 15559 | 8.06x | 2058 | 14795 | 7.19x |
| E20 | 1925 | 15741 | **8.18x** | 2040 | 15032 | **7.37x** |

Mono gains 1.17% on clob and 1.60% on aave. Legacy does not move, which is the
expected asymmetry: legacy pays the same absolute clock cost on a thread five
times longer, so it is 1.0% of legacy and 4.3% of mono.

The gain is about a third of what the profile implied, which is the usual
factor. Short hot leaves attract samples out of proportion to their time, and
`mach_absolute_time` is the shortest hot leaf there is.

Kept as the ceiling for E21, which recovers part of this without giving up the
counters.

## E21: stop reading the clock on every metric update

E20 says the counters are worth about 1.5%, but deleting them is not an option.
The cost is not the counter, it is `maybe_flush()` reading the clock to decide
whether a second has passed. So keep the counters and read the clock less.

`FlushDeadline` in `aptos-metrics-core` replaces the bare `last_flush: Instant`
in all three `ThreadLocal*` types. It holds the same deadline plus a countdown.
When a check finds the deadline far away, the next 31 updates skip the check
outright; when a check finds the metric due, the countdown drops back to 1 so
the next update checks again. A cold metric therefore behaves exactly as before
and a hot one reads the clock a thirty-second as often. The price is that a
flush can be up to 31 updates late, which for a metric being updated that
frequently is well under the one-second interval it is trying to honour.

This also fixes a latent bug. The old code assigned `last_flush = now`
unconditionally, outside the interval test, so a counter updated more than once
a second pushed its own deadline forward on every update and never flushed at
all. `FlushDeadline` only advances `last_flush` when it actually flushes.

| arm | clob legacy | clob mono | ratio | aave legacy | aave mono | ratio |
|---|---|---|---|---|---|---|
| E18 | 1943 | 15660 | 8.06x | 2080 | 14749 | 7.09x |
| E20 (ceiling) | 1939 | 15814 | 8.16x | 2076 | 15122 | 7.29x |
| E21 | 1943 | 15876 | **8.17x** | 2091 | 14970 | **7.16x** |

On clob E21 reaches the ceiling: 8.17x against E20's 8.16x, so the counters cost
nothing measurable once the clock reads are strided. On aave it recovers about
half, 7.16x against 7.29x.

The aave gap is the `ThreadLocalHistogramTimer`. Its `new` reads the clock and
its `Drop` reads it again through `elapsed()`, and neither is a flush check, so
no stride removes them. Closing the rest of aave's gap means dropping the
per-call `TIMER.timer_with(&["get_state_value"])` histogram from
`CachedStateView::get_state_slot`, which is an observability decision rather
than a performance one.

Kept. It is a change to shared `aptos-metrics-core` code, so it speeds up every
thread-local metric in the tree, not just this path.

## Where E21 leaves the transaction

Fresh profiles of `bin-e21` on both workloads, largest `txn_executor` thread,
CPU-weighted. Thread total 73.54 us/txn on clob and 72.00 on aave. Regions
overlap — natives call into the read-write set, which calls into the state view
— so the column does not sum to the total.

| region | clob us/txn | clob % | aave us/txn | aave % |
|---|---|---|---|---|
| resource read-write set | 16.20 | 22.0 | 18.55 | 25.8 |
| all natives | 12.32 | 16.8 | 13.14 | 18.3 |
| materialize | 10.96 | 14.9 | 11.45 | 15.9 |
| table natives | 10.69 | 14.5 | 6.91 | 9.6 |
| state view | 10.55 | 14.3 | 10.56 | 14.7 |
| loader | 8.16 | 11.1 | 9.13 | 12.7 |

Tables are 87% of all native cost on clob, which is what the workload is: an
AVL queue whose every node lives in a table item.

The `plumb.py` leaf classes confirm E21 did what it claimed. Clock reads fell
from 3.06 to 1.59 us/txn on clob and from 3.71 to 2.35 on aave. Nothing else
moved.

## The table native path allocates a key it immediately throws away

Excluding everything under the state view and the block-executor provider — so,
the part of a table access that is not a storage read — clob spends 5.41 us/txn
inside table natives. Self cost:

| leaf | us/txn |
|---|---|
| `borrow_box` | 0.791 |
| `ProductionNativeContext::table_borrow` | 0.696 |
| `malloc` | 0.453 |
| native wrapper | 0.375 |
| `hashbrown::make_hash` | 0.284 |
| `sdallocx` (cold) | 0.288 |
| `get_or_create_resource_entry` | 0.244 |
| `RootPool::alloc` | 0.236 |
| `handle_and_key` | 0.197 |
| serialize | 0.216 |

The glue self-cost is the surprise. `handle_and_key` BCS-serializes the key
argument into a fresh `Vec<u8>`, `InMemoryStorageKey::table_item` moves it into
a ~72-byte enum, the lookup borrows that enum, and then — on a hit, which is
almost always — the whole thing is freed. Every table access pays one malloc and
one free for a key that the map already holds a copy of.

`ResourceReadWriteSet` excluding the state view is 6.06 us/txn on clob, of which
2.18 is hash-table machinery: `make_hash` 0.510, two `RawTable::find` sites
0.668, `reserve_rehash` 0.306, `get_inner` 0.230, memcmp 0.216, plus the key and
`StructTag` hashes at 0.254. `reserve_rehash` at 0.306 means the read set
outgrows its 64 pre-sized slots on this workload.

Materialize is 10.96 us/txn on clob and splits three ways: 2.02 allocator,
1.36 memmove, and 1.22 re-encoding `StructTag`s that were decoded from the group
blob moments earlier (`merge_group` 0.487, `StructTag::serialize` 0.356,
`StructTag::hash` 0.211, `identifier::is_valid` 0.165). The last one is the
clearest waste, but removing it means keying groups on interned type ids instead
of `StructTag`, which is a data-structure change rather than a local fix.

## The storage floor is already below legacy's

Both VMs read the same state through the same `CachedStateView`. Comparing the
two clob profiles directly:

| region | mono us/txn | legacy us/txn |
|---|---|---|
| `CachedStateView` (inclusive) | 10.55 | 13.40 |
| RocksDB, `pread`, layered map, hot state | 6.70 | 10.85 |
| thread total | 73.54 | 719.59 |

Mono reads storage *less* than legacy does, so there is no mono-specific storage
overhead left to remove. What changed is the denominator: the same 10.55 us/txn
is 1.9% of a legacy transaction and 14.3% of a mono one. Shrinking it further
means changing the storage layer for both VMs, which is a different project.

Inside that floor, `read_group_from_storage` is 4.28 us/txn on clob, and its
leaves are `pread`, `rocksdb::BlockIter::BinarySeek` and
`layered_map::NodeRef::get_strong` — genuine cold reads of a resource group's
blob, one per group the block first touches. The unsync map caches the decoded
group afterwards (`fetch_group_member` inserts it, materialize takes it and puts
the merged version back), so the cost is first-touch, not per transaction.

## What each block is actually worth toward 10x

Speedup is legacy over mono with legacy fixed, so reaching 10x from 8.17x means
mono must shed 18.3% of its time. On the profiled clob thread that is 13.5
us/txn out of 73.54. Blocks, largest first:

| block | us/txn | share |
|---|---|---|
| interpretation and everything unattributed | ~32.4 | 44.1% |
| materialize | 10.96 | 14.9% |
| state view and DB (legacy pays more) | 10.55 | 14.3% |
| loader | 8.16 | 11.1% |
| read-write set, excluding the state view | 6.06 | 8.2% |
| table natives, excluding storage | 5.41 | 7.4% |

Deleting materialize outright takes clob to 9.6x. Materialize plus the read-write
set reaches 10.6x; materialize plus the loader reaches 11.0x. Nothing smaller
gets there. Aave is further out: it needs 28.4% off 72.00 us/txn, which is
materialize plus the loader plus the read-write set, all of them, to zero.

So the remaining gap is not a list of 1% items. It is one structural cost.

## The structural cost: a legacy write set per transaction

Materialize exists to turn mono's read-write set into a `WriteSet` of
`(StateKey, WriteOp)` with BCS bytes, because that is what `TransactionOutput`
carries. Its 10.96 us/txn on clob has no dominant leaf — 2.02 allocator, 1.36
memmove, 1.22 re-encoding `StructTag`s, 0.95 building the `BTreeMap` and sorting
`StateKey`s, 0.66 actual value serialization, the rest spread thinner. It is the
format conversion itself that costs, not any one step of it.

The conversion is also repeated. A clob block writes the same AVL nodes from
many transactions, and every one of those transactions serializes the node again
even though only the last version reaches storage. The unsync map already holds
the live mono value (`MonoValue::Write { ptr, pin }`), so the bytes are the only
thing being rebuilt.

Deferring serialization to the end of the block would serialize each key once
instead of once per writing transaction.

## Correction: the write set cannot be deferred

`DoLedgerUpdate::assemble_transaction_infos` computes
`write_set_hash = CryptoHash::hash(txn_output.write_set())` for every committed
transaction, and `WriteSet` derives `BCSCryptoHash`, so the hash covers every
value's bytes. That hash becomes `TransactionInfo::state_change_hash` and goes
into the transaction accumulator. Each transaction therefore needs the bytes of
its own version of every key it wrote, not just the block's last version.

So end-of-block serialization is out. Materialize's ~11 us/txn is load-bearing.

Two things survive from the idea. `assemble_transaction_infos` runs
`into_par_iter()` while the sequential block executor does not, so moving the
serialization from the executor into the ledger-update phase would still be a
wall-clock win on a multicore host even with the CPU work unchanged. And the
bytes are produced once by `serialize_into`, copied into `Bytes`, then
BCS-encoded a second time by the hash, so the format contract itself costs more
than one pass.

## E22: recycle the table-item key buffer (reverted)

The table profile says a warm table access mallocs a `Vec<u8>` for the
serialized key and frees it a few hundred nanoseconds later, and that
`reserve_rehash` costs 0.306 us/txn. So: park a spare key buffer on
`ResourceReadWriteSet`, hand it to `bcs_serialize_arg`, and take it back once
the lookup is done; and raise `INITIAL_ENTRY_CAPACITY` from 64 to 128 so a
table-heavy transaction never rehashes. Recycling is sound because
`get_or_create_resource_entry` goes through `entry_ref`, so the map always clones
its own copy of the key.

| arm | clob legacy | clob mono | ratio | aave legacy | aave mono | ratio |
|---|---|---|---|---|---|---|
| E21 | 1942 | 15625 | 8.05x | 2079 | 14951 | 7.19x |
| E22 | 1938 | 15694 | 8.10x | 2072 | 14554 | 7.02x |

Clob gains 0.44%, which is inside the noise floor. Aave loses 2.7%, which is
not: E21's aave number is 14970 and 14951 across two sweeps, so 14554 is a real
drop. The likely cause is the capacity bump rather than the recycling. An aave
transaction touches far fewer than 64 keys, so doubling the map only buys it a
larger allocation to zero and a sparser table to probe.

Reverted whole. Splitting it — keeping the recycling, dropping the capacity bump
— would at best land on clob's +0.44%, and the budget above says items of that
size are not the problem. The unsafe reborrow and the closure-wrapped
`table_borrow` are not worth carrying for a number a sweep cannot resolve.

This is the third allocation-shaving experiment in a row (E15, E19, E22) to come
back null. Taken together they are evidence for the budget conclusion: the
allocator cost the profiles show is spread across many small sites, and removing
any one of them is unmeasurable.

## Per-block ratios: where mono's advantage is thinnest

The block table above says what each block costs mono. It does not say whether
mono is *good* at that block. Measuring the same regions on the legacy clob
profile answers that. Both columns are profiled us/txn, mono total 73.54 and
legacy total 719.59.

| block | mono | legacy | block ratio | mono % | drags total ratio to |
|---|---|---|---|---|---|
| state view (incl DB) | 10.55 | 13.40 | 1.27x | 14.3 | 11.21x without it |
| DB read only | 6.70 | 10.85 | 1.62x | 9.1 | |
| loader | 8.16 | 45.61 | 5.59x | 11.1 | 10.31x without it |
| materialize / change set | 10.96 | 70.86 | 6.47x | 14.9 | 10.37x without it |
| whole transaction | 73.54 | 719.59 | 9.78x | 100 | |

The last column is `(legacy - block) / (mono - block)`: what the overall ratio
would be if the block cost nothing in either VM. The state view drags the ratio
by 1.43 points, nearly three times what materialize or the loader drag.

That is the shape of the problem. Mono runs its own work at roughly 13x, and
then pays a storage cost that is close to legacy's in absolute terms. Legacy's
change-set machinery alone is 70.86 us/txn, which is more than mono's entire
transaction, and legacy's loader is 45.61. Mono is not losing to legacy in any
block. It is losing to its own storage floor.

Two things follow. There is no Amdahl wall at 10x -- if everything but the state
view went to zero the ratio would be 68x -- so the goal is reachable in
principle. But shrinking the state read path further means making storage faster
for both VMs, which improves the numerator too and so returns less ratio than
its size suggests.

## What is actually inside the state read path

Self cost under `CachedStateView`, mono clob, 10.55 us/txn total:

| leaf | us/txn | what it is |
|---|---|---|
| `mach_absolute_time` | 1.590 | the `get_state_value` histogram timer |
| `NodeRef::get_strong` | 1.292 | speculative-state layered map |
| rocksdb + `pread` + memcmp | ~1.75 | real DB |
| `StateSlot::clone` | 0.335 | |
| `ShardedStateCache::try_insert` | 0.207 | |
| `LocalHistogram::observe` + `inc_with_by` | 0.229 | the rest of the metric |

`mach_absolute_time` is the single largest leaf anywhere in a mono transaction.
E21 strided the flush checks but `ThreadLocalHistogramTimer` reads the clock in
`new` and again in `Drop`, and no stride removes those. At roughly 30 state
reads per transaction that is 60 clock reads.

The profile over-states it, and E20 already measured the truth: deleting the
state-view metrics outright bought 1.17% on clob and 1.60% on aave, and E21
recovered all of clob's and about half of aave's. So the headroom left in this
leaf is roughly zero on clob and 0.8% on aave. It stays the largest leaf in the
profile and is still not worth spending. Trust the ceiling arm over the profile:
this is the third time a short hot leaf has read about three times its worth.

## What is inside the loader's 8.16 us/txn

| leaf | us/txn | required per transaction? |
|---|---|---|
| `ModuleReadSet::meter_once` | 0.811 | yes, gas |
| `Loader::charge_non_read_set_slots` | 0.771 | yes, gas |
| `Loader::load_function` | 0.694 | no |
| `HashMap::insert` (read set) | 0.588 | yes, Block-STM validation |
| `LoadedModule::get_instantiated_function_ptr` | 0.463 | no |
| `ModuleReadSet::get_loaded` | 0.372 | no |
| verifier + specializer residue | 0.72 | no, first touch amortized |

Nothing here is a cache miss. Modules are already loaded once globally and the
lowered code is already cached. What repeats per transaction is the bookkeeping
around the cache: look the module up in this transaction's read set, check it has
been metered, charge its dependency slots, hash the type arguments to find the
instantiation.

That is the shape a call-site cache fits. Resolve a call site once, then on
every later execution replay a precomputed gas charge and a precomputed module
list instead of walking and re-hashing. Gas stays identical and the read set
stays complete, so it is not a shortcut -- it is memoizing a pure function of the
call site. The invalidation rule is module publishing, which mono does not
support yet (`for_each_module_write` is a no-op), so a block-scoped cache is
sound today and has a defined trigger for when publishing lands.

## Standing back: what 10x costs

Measured, not profiled. Clob mono is 63.0 us/txn and legacy 514.7, so 10x needs
mono at 51.5 -- shed 11.5 us/txn. Scaling the profile by 0.857 to measured time:

| candidate | measured us/txn | share of the 11.5 | status |
|---|---|---|---|
| materialize | 9.4 | 82% | diffuse; no leaf above 1.4 |
| state view total | 9.0 | 78% | 5.7 is irreducible DB |
| loader | 7.0 | 61% | bookkeeping around a warm cache |
| rws excluding state view | 5.2 | 45% | diffuse |
| state-view timer | 1.4 | 12% | already spent by E20/E21 |

No single item covers it, and the timer row is already banked. That leaves the
loader and materialize. A call-site cache taking most of the loader plus a
materialize rewrite taking half of its block is 4.5 + 4.7 = 9.2 us/txn, landing
around 9.4x on clob. Aave needs 28% rather than 18%, so it would fall further
short.

Stating it plainly: 10x sequential on these two workloads does not follow from
the changes now on the table. It needs the loader's per-transaction bookkeeping
gone, materialize roughly halved, and then something structural on the state read
path that neither VM pays today.

Everything smaller has now been tried. E15 (pre-size the group blob), E19 (drop
no-op writes) and E22 (recycle the table key) all returned null against a sweep
that resolves about 1.5%. The profiles say why: after E21 no leaf anywhere in a
mono transaction costs more than 1.6 us/txn, and that one is a clock read. The
cost is spread, so only changes that delete a whole category of work register.

## Storage, measured against legacy block by block

All numbers below are CPU-weighted inclusive us/txn on the largest
`txn_executor` thread, mono from the E21 profiles and legacy from the baseline
profiles. Legacy is valid to compare against because no legacy code changed
across E1 to E21.

| region | clob mono | clob legacy | ratio | aave mono | aave legacy | ratio |
|---|---|---|---|---|---|---|
| thread total | 73.54 | 719.59 | 9.78x | 72.00 | 660.69 | 9.18x |
| everything matching `group` | 8.35 | 43.41 | 5.20x | 13.64 | 83.94 | 6.15x |
| `CachedStateView` | 10.55 | 13.40 | 1.27x | 10.56 | 13.35 | 1.26x |
| RocksDB, `pread`, LZ4 | 5.92 | 5.65 | **0.95x** | 5.18 | 5.25 | 1.01x |
| layered map | 1.75 | 4.11 | 2.35x | 2.33 | 3.95 | 1.70x |
| `ShardedStateCache` | 0.81 | 0.51 | **0.63x** | 0.99 | 0.57 | **0.58x** |
| `StateSlot::clone` | 0.34 | 1.15 | 3.38x | 0.73 | 1.00 | 1.37x |

The disk is the same cost for both VMs. RocksDB is 5.92 us/txn in mono and 5.65
in legacy, so mono is very slightly worse. That block runs at 1.0x inside a
transaction that runs at 9.8x, and it is 8% of a mono transaction against 0.8%
of a legacy one. Nothing mono does can move it.

Two blocks where mono is absolutely worse than legacy: the sharded state cache
(0.81 against 0.51) and RocksDB itself. Both are small and neither has an
obvious cause beyond mono issuing a comparable number of reads with less other
work to hide them behind.

## Resource groups: what the assemble-per-transaction cost actually is

The whole group subsystem, mono against legacy:

| piece | clob | aave |
|---|---|---|
| `read_group_from_storage` | 4.28 | 5.75 |
| `merge_group` | 2.51 | 5.73 |
| `take_group` / `insert_group` / `fetch_group_member` | 0.20 | 0.48 |
| total matching `group` | 8.35 (11.4%) | 13.64 (18.9%) |
| same in legacy | 43.41 (6.0%) | 83.94 (12.7%) |

Removing groups from both VMs takes clob from 9.78x to 10.37x and aave from
9.18x to 9.88x, so groups are worth about 0.6 and 0.7 points of ratio. On aave
that is the largest single drag of any block.

### The read half is cold disk

`read_group_from_storage` leaves on aave: `mach_absolute_time` 1.386,
`NodeRef::get_strong` 0.856, `from_utf8` 0.833, BCS deserialize 0.368 + 0.292,
RocksDB `SkipListRep::Iterator::Seek` 0.261. The BCS decode is a small part; the
rest is a genuine cold read.

The unsync map does cache the decoded group for the rest of the block
(`fetch_group_member` inserts it, materialize takes it and puts the merged
version back at `mono_move/mod.rs:525`). It does not help here because both
workloads drive a distinct user address per transaction, so a 500-transaction
block gets roughly 500 first touches and no reuse. The cache is correct and
does nothing for these workloads.

### The write half re-encodes the whole blob, and so does legacy

`merge_group` leaves on aave: memmove 1.526, `serialize_newtype_struct` 1.032,
`finish_grow` 0.883, `BTreeMap::insert` 0.667, `merge_group` self 0.594,
allocator 1.323 across four symbols, `StateValue::compute_rapid_hash` 0.253.
That is a `BTreeMap<StructTag, Bytes>` being rebuilt and BCS-encoded from
scratch to change one member of a two-to-five kilobyte blob.

Legacy does exactly the same thing. `executor_utilities.rs:401 serialize_groups`
calls `bcs::to_bytes(&btree)` on the finalized group, once per writing
transaction. Its leaves under `materialize_output` on aave are the same shape:
memmove 2.830, `finish_grow` 1.464, `Vec::from_iter` 1.423, allocator 3.615.
Legacy pays 21.24 us/txn where mono pays 9.77, so mono is already 2.2x faster at
this. `serialize_groups` does not appear as a named frame because it is inlined.

### One write per block is not available

Three separate reasons, in order of how hard they are to move:

1. `do_ledger_update.rs:90` computes `CryptoHash::hash(txn_output.write_set())`
   and feeds it to `.state_change_hash(...)`, which goes into `TransactionInfo`
   and therefore into the ledger. Every transaction needs its own write set with
   its own bytes. Emitting a group as a delta would change the ledger format.
2. The DB-facing write is already once per block.
   `do_get_execution_output.rs:448 update_with_memorized_reads` folds every
   transaction's write set into one state update before the tree sees it. The
   per-block part of the idea is done.
3. `ResourceGroupSize` is part of the group write and gas is charged on it, so a
   transaction has to know the assembled size regardless. This one does not
   force a serialization, though: legacy V1 tracks the size incrementally with
   arithmetic (`resource_group_adapter.rs:324` and `:352`), serializing only the
   changed tag via `group_tagged_resource_size` (`:47`). Only reason 1 actually
   forces the blob.

What is left is not "write less often" but "assemble more cheaply". Mono
rebuilds the sorted map and re-encodes from scratch. Keeping the group as an
already-sorted `Vec<(StructTag, Bytes)>` alongside the blob it last produced
would let a single-member update splice — copy prefix, write the new member,
copy suffix — instead of re-running BCS over every member and every
`StructTag`. That deletes `BTreeMap::insert`, `StructTag::serialize` and most of
`serialize_newtype_struct`, roughly 2.2 of aave's 5.73, and leaves the memmove.

### Implementing `resource_group_write_set` is a parallel-path change

`MonoTxnOutput::resource_group_write_set()` returns `HashMap::new()` and
`for_each_resource_group_key_and_tags` is a no-op, so mono bypasses the block
executor's native group support and emits assembled blobs as plain resource
writes. Adopting the native path would keep members as per-tag deltas in the
multi-version map and move assembly into `materialize_output`.

That is the right shape, but it does not help this goal. Under the sequential
executor `materialize_output` runs on the same thread immediately after
execution, so moving work there moves it nowhere. The win is under the parallel
executor, where materialization is off the execution critical path, plus
finer-grained conflict detection on group members. Worth doing for the parallel
path; not a sequential 10x lever.

## What is left in the read path

`BlockSTMSequentialProvider::get_resource` is 13.89 us/txn on clob (18.9%) and
15.76 on aave (21.9%) — the single largest named region in a mono transaction.
Its self costs, largest first:

| leaf | clob | aave | verdict |
|---|---|---|---|
| `mach_absolute_time` | 1.590 | 2.325 | the `get_state_slot` histogram; E21 says worth 0 on clob, 0.8% on aave |
| `NodeRef::get_strong` | 1.292 | 0.960 | speculative layered map; legacy pays 2x more |
| `memset` / `memcmp` | 1.041 | 0.295 | RocksDB comparator and buffer clearing |
| RocksDB `BinarySeek`, `pread`, comparator, LZ4 | ~1.04 | ~1.37 | the floor |
| `StateSlot::clone` | 0.335 | 0.732 | one clone per read out of the cache |
| `ShardedStateCache::try_insert` | 0.207 | 0.608 | mono pays 1.6x legacy here |
| `reserve_rehash` + `HashMap::insert` | 0.369 | 0.660 | read-set growth |

Plus, outside the leaf table, `StateKey::resource` and
`StateKey::resource_group` construction at 1.35 and 1.34 us/txn. E8 already
flagged this and left it: the tag is cached now, but `StateKey::resource` still
takes a read lock on the global key registry and probes it on every call. This
is the cleanest remaining mono-specific item in the read path, because after E8
nothing else about it is mono's doing.

`struct_tag_of` residue after E8 is 1.40 on clob and 0.69 on aave. E7 and E8 did
land: on the pre-E7 profiles SipHash was 4.00 and 4.53 us/txn and tag rebuilding
was 3.31 and 3.97; now they are 0.33/0.48 and 1.40/0.69.

## Storage scoreboard

Ranked by what is actually recoverable, mono-specific only:

| item | clob | aave | recoverable |
|---|---|---|---|
| `merge_group` re-encode | 2.51 | 5.73 | about half, by splicing instead of re-encoding |
| `StateKey` registry probe | 1.35 | 1.34 | most, by caching the key per (address, type) |
| `get_state_slot` histogram | 1.59 | 2.32 | 0 on clob, ~1.0 on aave; observability decision |
| `StateSlot::clone` + `try_insert` | 0.54 | 1.34 | unclear |
| `read_group_from_storage` | 4.28 | 5.75 | 0, cold disk |
| RocksDB, `pread`, LZ4 | 5.92 | 5.18 | 0, and legacy pays the same |

Adding the recoverable column gives roughly 3 profiled us/txn on clob and 5 on
aave. Scaling to measured (0.857) and discounting the profile's known 2-3x
overstatement of short hot leaves puts the realistic total at 1.5 to 2 measured
us/txn. The budget for 10x is 11.5 on clob and about 20 on aave.

So storage is 19% to 22% of a mono transaction and almost all of it is a floor
both VMs share. There is roughly 2 measured us/txn of mono-specific fat in it,
not 11. Storage is the largest phase but it is not where the missing 10x is.

## The read path's fixed overhead, traced to source

Two subagents mapped the code behind the leaf table above. What they found that
the profile alone did not show:

**Every state read takes a DashMap shard lock, and a miss takes two.**
`ShardedStateCache` is `[DashMap<StateKey, StateSlot>; 16]`
(`cached_state_view.rs:39`). A hit calls `get_cloned` (`:67`), taking a shard
read lock. A miss then calls `try_insert` (`:77`), which calls
`entry(state_key.clone())` — a second lock acquisition, a second hash, and a key
clone taken before the lock confirms the entry is vacant. On the sequential path
every one of these locks is uncontended, so the whole cost is pure overhead.
This is the likeliest explanation for mono paying 0.81/0.99 us/txn here against
legacy's 0.51/0.57: identical locks, less other work to hide them behind. A
single `entry()` covering both lookup and insert would halve it.

**The speculative map re-hashes a hash.** `LayeredMap::get_with_hasher`
(`layered-map/src/map/mod.rs:60`) runs `hash_builder.hash_one(key)` where `key`
is already a `&HashValue` — a 32-byte crypto hash, precomputed and cached on the
`StateKey`, pushed through ahash again to address the HAMT. The hot-state base
DashMap uses the `HashValue` directly and skips this. This sits inside the
`NodeRef::get_strong` region measured at 1.29 (clob) and 0.96 (aave).

**`get_strong` upgrades weak refs.** `node.rs:150-174`: nodes contributed by
older merged layers are stored as `Ref::Weak` so the layer can be dropped
(`node.rs:204`), so traversing them costs a `Weak::upgrade`, an atomic
compare-exchange, per level. Depth is a function of HAMT key density, not of
block height or layer count.

**There is no prefetch.** `prime_cache` (`cached_state_view.rs:180`) runs
*after* block execution, to populate the memorized cache for hot-state
promotion. The `prime_state_cache` flag is `false` on every normal execution
path and true only in `by_transaction_output`
(`do_get_execution_output.rs:259`), so the benchmark's replay path gets
`MakeHotOnly` and no pre-execution read-ahead at all.

**Clone counts per read.** Cold: `StateKey` cloned three times (into the
`StateSlot`, as the DashMap key, and again inside the `StateSlot` stored in the
map) and `Bytes` once. Hot-state hit: `Bytes` twice, once out of the hot DashMap
(`hot_state.rs:144`) and once into memorized. This is the `StateSlot::clone` at
0.34/0.73.

**The hot-state delta is walked even when empty.** `hot_state.rs:134-145`
traverses the delta `LayeredMap` trie on every hot-state lookup before falling
back to the base DashMap, so a key in neither pays the full HAMT walk plus the
DashMap probe.

## Correction: incremental group size, and why the stubs matter

Two things the group trace corrected.

First, `ResourceGroupSize` does not force a serialization. Legacy V1 keeps it as
`Combined { num_tagged_resources, all_tagged_resources_size }` and updates it
with `increment_size_for_add_tag` / `decrement_size_for_remove_tag`
(`resource_group_adapter.rs:352`, `:324`), serializing only the changed tag
through `group_tagged_resource_size` (`:47`). Mono has no separate size path —
`merge_group` (`txn_output.rs:229`) derives everything from the blob it just
built. That is fine for correctness and it is not what costs the time, but it
means mono has no cheap size available if a splice-based assembly ever wants
one.

Second, and more important than the performance question: **mono's stubbed
`resource_group_write_set` and `for_each_resource_group_key_and_tags` are a
correctness gap for parallel execution, not just a missed optimization.**
`process_resource_group_output_v2` (`executor.rs:225-265`) uses them to write
per-tag entries into `VersionedGroupData` and to remove stale per-tag entries
left by a prior incarnation. Without them, a re-executed transaction leaves
ghost writes in the multi-version map and later transactions read them.
`versioned_group_data.rs:72-91` confirms the map never holds an assembled blob
at any point — only `(group_key, tag)` entries plus sizes and a tag set. Mono's
`UnsyncMap`-only caching is sufficient for sequential and cannot serve versioned
reads.

Legacy's sequential path also runs materialization inline
(`executor.rs:2231-2246`), so the blob serialization blocks the next transaction
there exactly as it does in mono. Under the parallel executor it becomes
`TaskKind::PostCommitProcessing` (`executor.rs:1326`). That confirms the earlier
conclusion from both sides: implementing the stubs is a parallel-path win and a
parallel-path correctness requirement, and does nothing for sequential 10x.

---

# Appendix A: change index

Every experiment, what it touched, what it cost, and why it is or is not worth
relanding. Sizes are `git diff --stat` against the branch point. "Gain" is the
measured `inner block executor` TPS delta from the sweep that decided it, clob
first then aave.

The whole set was 837 insertions and 327 deletions across 42 files, plus 97
regenerated `.exp` baselines and two new files. It is all reverted.

## Kept experiments (11), in the order they landed

### E2 — stop allocating 11 MiB per transaction

Files: `runtime/src/types.rs`, `runtime/src/heap/mod.rs`,
`runtime/src/lib.rs`, `aptos-transaction-executor/src/providers.rs`.

Every transaction built a fresh `Heap` at `MAX_HEAP_SIZE = 10 MiB` plus a 1 MiB
stack, touched a few KiB of it, and dropped it. At 500 transactions per block
that is 5.5 GiB of `mmap`/`munmap` traffic per block for a working set of tens
of KiB. Changed the heap to start at `INITIAL_HEAP_SIZE` and grow, and pooled
the stack.

Gain: 5.82x → 6.47x, 5.38x → 6.13x. The single largest win in the whole set.

Trade-off: growth is now a runtime event, so a transaction that genuinely needs
10 MiB pays a copy it did not pay before. Measured as noise on both workloads.
The differential testsuite needed a `--heap-max` directive and a new
`vec_heap_growth.move` case to keep exercising the growth path deliberately;
that is the `testsuite/src/{engine,parser,runner}.rs` churn.

Relanding: yes, with the testsuite directive. This is a memory-footprint fix as
much as a speed fix, and 5 GiB per block is indefensible on its own.

### E3 — stop zeroing the interpreter stack

Files: `runtime/src/memory.rs`, `runtime/src/heap/mod.rs`.

The stack was zero-filled on allocation. The verifier already proves no frame
slot is read before it is written, so the zeroing is dead work.

Gain: about 3% on both.

Trade-off: real. Zeroed memory turns a verifier bug into a deterministic null
rather than a read of whatever the last transaction left there. This trades a
debugging property for 3%. Given that E2 pools the stack across transactions,
the leftover bytes are now another transaction's data, not fresh pages — the
information-flow surface is worse than it looks.

Relanding: only behind a debug-assertion that keeps the zeroing in test builds.
Not worth 3% otherwise.

### E4 — stop copying bytes that do not need copying

Files: `runtime/src/value_utils.rs`, `core/src/memory.rs`.

Value moves used a generic byte copy where the size was known at lowering time.
Specialized the common widths.

Gain: about 1.5% on both. Low risk, small payoff.

### E5 — inline cache for indirect calls

Files: `runtime/src/call_cache.rs` (new, 71 lines), `runtime/src/interpreter.rs`,
`runtime/src/lib.rs`.

Dynamic dispatch resolved through a hash lookup on every indirect call. Added a
per-transaction monomorphic inline cache keyed on the call site.

Gain: about 2% clob, 1% aave.

Trade-off: the cache is per-transaction, so it warms up 500 times per block and
never reaches steady state. A per-block cache would be worth more but has to be
invalidated on module publish, which is exactly the versioning problem the
epoch-sealed-code design exists to solve. This was the cheap half.

Relanding: fold into the loader/epoch-sealed work rather than landing alone.

### E7 — stop hashing storage keys with SipHash

Files: `core/src/storage/mod.rs`, `core/src/storage/resource_provider.rs`,
`loader/src/read_set.rs`, `aptos-transaction-executor/Cargo.toml`, `Cargo.lock`.

`StateKey` maps used the default `SipHash` hasher. Storage keys are already
hashed values, so cryptographic strength buys nothing here. Introduced
`type FastMap<K, V> = HashMap<K, V, ahash::RandomState>` and switched the
read set, the resource provider, and the write set to it.

Gain: about 4% clob, 5% aave. SipHash fell from 4.00/4.53 profiled us/txn to
0.33/0.48.

Trade-off: `ahash` is not HashDoS-resistant in the adversarial sense. These maps
are keyed by state keys the transaction itself derives, and the map lives for
one transaction, so an attacker who can force collisions can only slow down
their own transaction. The bound is the gas limit, not the block.

Relanding: **yes.** This is the cleanest win in the set — small diff, no
semantics change, and the security argument is solid. Recommend landing it on
its own.

### E8 — stop rebuilding storage tags

Files: `core/src/storage/resource_provider.rs`, `global-context/src/context.rs`,
`core/src/interner.rs`.

Every global-storage access rebuilt a `StructTag` from the interned type,
allocating an `Identifier` and a `Vec` each time. Added a
`DashMap<InternedType, Arc<StructTag>>` on the global context.

Gain: about 3% clob, 2% aave. `struct_tag_of` fell from 3.31/3.97 profiled
us/txn to 1.40/0.69.

Trade-off: the cache is on the global context, so it survives across blocks and
grows with the number of distinct instantiated types the chain has ever seen.
It needs an eviction policy or a maintenance-phase reset before it can ship —
an attacker can grow it with distinct generic instantiations. That is why it is
not on the reland-now list.

### E11 — stream the write set instead of building a map

Files: `aptos-transaction-executor/src/materialize/txn_output.rs`,
`aptos-transaction-executor/src/outcome.rs`,
`aptos-move/block-executor/src/mono_move/mod.rs`,
`aptos-move/block-executor/src/task.rs`.

Materialization built a `HashMap` of the write set, then immediately iterated it
once and dropped it. Replaced with a streaming callback
(`for_each_resource_key`, and the `&mut dyn FnMut` shape that sidesteps
rust-lang/rust#145188).

Gain: about 3% on both.

Trade-off: the callback form is harder to read than the map form, and it forced
a trait-signature change in `task.rs` that the legacy path also has to satisfy.
That is the reason the group stubs (`resource_group_write_set` returning an
empty map, `for_each_resource_group_key_and_tags` as a no-op) exist in mono's
sequential path — they are placeholders, and a parallel path cannot ship with
them.

### E12 — size the module read-set once

Files: `loader/src/read_set.rs`, `loader/src/loader.rs`, `loader/src/lib.rs`.

The module read set grew from empty, rehashing several times per transaction.
Pre-sized from the previous transaction's count.

Gain: about 1% on both.

### E13 — inline the moves the interpreter makes

Files: `runtime/src/interpreter.rs`, `core/src/instruction/mod.rs`.

Frame-to-frame moves went through a call. Inlined the common shapes.

Gain: about 1.5% on both. See the micro-op histogram section — a third of all
executed ops are moves, which is what motivated this and E17.

### E14 — size the serialization buffer

Files: `runtime/src/value_utils.rs`, `aptos-state-view-providers/src/lib.rs`.

BCS output buffers started empty and grew. Seeded from the value's known size.

Gain: about 1% on both.

### E17 — fuse runs of moves into one dispatch

Files: `specializer/src/lower/parallel_copy.rs`,
`specializer/src/lower/translate.rs`, `core/src/instruction/mod.rs`,
`runtime/src/interpreter.rs`, `runtime/src/verifier.rs`.

The largest interpreter change in the set, and the last one that mattered. The
micro-op histogram showed a third of executed ops are frame-to-frame moves,
mostly in runs at call boundaries. Added a fused multi-move op emitted by the
parallel-copy lowering, with matching verifier support.

Gain: 7.22x → 7.91x, 6.52x → 6.95x. Second-largest win after E2.

Trade-off: this is 161 lines across the specializer, the instruction set, the
interpreter and the verifier. A new micro-op is a new thing the verifier must
prove safe, and its frame-slot bounds check is the load-bearing part. That is
real review surface for 9%.

Relanding: yes, but as its own reviewed PR with the verifier argument written
out, not folded into anything else.

### E18 — stop cloning the resource group out of the block cache

Files: `aptos-move/mvhashmap/src/unsync_map.rs`, `aptos-move/mvhashmap/Cargo.toml`,
`aptos-move/block-executor/src/mono_move/provider.rs`.

Reading a cached resource group cloned the whole `BTreeMap` of members out of
the `UnsyncMap`. Returned a borrow instead.

Gain: about 3% clob, 3% aave. First run over 8x on clob.

Trade-off: `UnsyncMap` is the sequential-only cache, so the borrow is sound
here and would not be under the concurrent `MVHashMap`. The parallel path needs
a different answer.

### E21 — strided flush check in `aptos-metrics-core`

Files: `crates/aptos-metrics-core/src/thread_local.rs`.

E20 established the ceiling: deleting the state-view metrics entirely was worth
about 4%. The cost was not the counter increment, it was
`mach_absolute_time` — every thread-local metric update read the clock to decide
whether to flush. Changed the flush check to a strided counter so the clock is
read once every N updates instead of every update.

Gain: 7.91x → 8.17x, 6.95x → 7.16x. `mach_absolute_time` is still 1.59/2.33
profiled us/txn after this, so more is available.

Trade-off: flush timing becomes approximate — a low-rate metric can now sit
unflushed for longer than the interval. For counters that is fine. This is also
the only change in the set outside Move and the VM, so it affects every crate
that uses the metrics core.

Relanding: **yes**, and it helps the legacy VM too. Worth raising with whoever
owns `aptos-metrics-core`. Note that this is a general-purpose crate, so the
change needs their review of the flush-latency contract, not just a perf number.

## Reverted experiments (6): what was tried and why it did not work

Recording these matters more than the wins. Each one was a plausible idea that
measurement rejected.

### E1 — reuse the per-block 64 MiB arena

The premise was wrong. `RESOURCE_ARENA_BYTES = 64 MiB` is reserved, not
committed — the pages are never touched, so there is nothing to reuse.
Measurement showed no allocation traffic at all from the arena. The real 5 GiB
came from the per-transaction heap, which E2 fixed.

Lesson: measure the traffic before optimizing the allocation.

### E6 — share one root pool per transaction

Files touched: `core/src/root_pool.rs`, `core/src/native/context.rs`,
`runtime/src/native_context.rs`.

Sharing the GC root pool across the three VM runs in a transaction (prologue,
payload, epilogue) should have saved two setups. It cost 7.22x → 7.07x. The
shared pool is larger, so root scanning walks more slots, and that outweighs the
saved setup.

### E9 — memoize fully charged dependency slices

Cost 2.1% and 3.4%. The memo table lookup is not cheaper than recomputing the
slice, and it adds a per-transaction allocation.

### E15 — pre-size the resource-group blob

No measurable gain. The blob buffer growth was already amortized.

### E19 — drop writes that did not change the value

Cost 1.4% and 3.0%. Comparing the old and new value costs more than writing.
This also changes semantics — `state_change_hash` would differ — so it was
never landable as-is.

### E22 — recycle the table-item key buffer

The table native allocates a key buffer it immediately throws away. Recycling it
cost aave 2.7% (7.19x → 7.02x) and was flat on clob. The allocation is small
enough that the recycling bookkeeping dominates.

## Measurement-only work (no code kept)

- **E10** — state-view instrumentation, to find out how much time the read path
  takes. Answer: 20% of a transaction.
- **E16** — how much the state-view stopwatch itself costs. This is the
  measurement that led to E20 and E21.
- **E20** — the ceiling arm: delete the state-view metrics entirely and see what
  a transaction costs without them. 4%. This is what made E21 worth doing and
  bounded how much more is available.

The ceiling-arm technique is the most reusable thing here. Before optimizing a
cost, build an arm that removes it entirely and measure that. It bounds the
payoff before any real work happens, and twice in this log it showed the payoff
was smaller than the profile suggested.

---

# Appendix B: the four investigations

The experiments are the visible part. These four investigations are what
actually determined where the effort went, and they are the part worth reading
before anyone tries again.

## 1. Why 25x in criterion becomes 5.6x end to end

Asked first, answered in "The full partition" and "The pivot". The mono
interpreter really is 10-20x faster than legacy on the same Move code. That
speedup is diluted twice.

A transaction is not only interpretation. The exclusive partition put 61% of a
mono transaction outside the interpreter: storage reads, materialization, the
loader, the write set, metrics, and block-executor plumbing. Legacy pays most of
those too, at roughly the same absolute cost. So the ratio on the whole
transaction is bounded by how much of it is VM work, and on these workloads that
is under 40%.

The second dilution is the non-VM tail outside the block executor entirely —
about 34% of wall time in the e2e harness, which no VM change reaches.

Consequence: the interpreter was already fast enough that further interpreter
work had a small ceiling. This is the finding that redirected everything after
E17.

## 2. Where the hashing comes from

Asked directly by the user. Answered three times as the answer kept changing.

Original: `SipHash` on `StateKey`, in the read set, the resource provider and
the write set, at 4.00 (clob) / 4.53 (aave) profiled us/txn. E7 took it to
0.33 / 0.48.

What is left is not one hasher but three distinct things:
- `LayeredMap` re-hashing a value that is already a hash, in the state-view
  layer. Legacy pays this too.
- `StateKey` construction going through a registry probe — 1.35 / 1.34 us/txn.
  This is a hash lookup to intern a key that is then used once.
- DashMap shard selection in the global context caches.

None of these are the VM's hasher. They are the state layer's.

## 3. Memory traffic

Asked as "500 txn at 10 MB is 5 Gb which is huge". Confirmed exactly: 5.5 GiB of
`mmap`/`munmap` per 500-transaction block, from the per-transaction heap and
stack, for a live set of tens of KiB.

E2 fixed it by growing from `INITIAL_HEAP_SIZE` instead of reserving
`MAX_HEAP_SIZE`. Peak RSS on the benchmark went from 294 MiB to roughly flat.

The user's specific proposal — truncate the arena on freeze, GC into a linked
list of pages, keep only live data — was not needed for the heap, because
growing lazily already keeps the footprint proportional to live data. It remains
the right shape if the per-block arena ever becomes a real cost, but E1 showed
it is not one today: the 64 MiB arena is reserved and never touched.

## 4. Storage, and the resource-group question

Asked as "we assemble and write per block, even though we can have one write per
block of group and other writes are deltas".

The DB-facing write is **already** once per block —
`do_get_execution_output.rs:448` folds every transaction's write set into one
state update via `update_with_memorized_reads`.

What forces per-transaction blob assembly is `state_change_hash`
(`do_ledger_update.rs:90` hashes each transaction's write set). That is a ledger
commitment, not a storage inefficiency, and it cannot be deferred without
changing what the chain commits to.

Legacy does the same full re-serialization — `executor_utilities.rs:401`,
`bcs::to_bytes` on the whole finalized group per transaction. Mono is already
2.2x faster at it. Measured, groups cost mono 8.35 (clob) / 13.64 (aave)
profiled us/txn against legacy's 43.41 / 83.94. Removing groups entirely from
both VMs moves the ratio from 9.78x to 10.37x on clob and 9.18x to 9.88x on
aave, so groups are a *headwind* on the ratio but a small one.

The available win is cheaper assembly, not fewer writes: splice the changed tag
into the existing blob instead of re-encoding all members
(`txn_output.rs:229`, which already carries a `TODO(cleanup)` saying the same
thing). Worth about 2.2 of aave's 5.73 us/txn in `merge_group`.

One correction worth keeping: legacy tracks `ResourceGroupSize` incrementally
with arithmetic (`resource_group_adapter.rs:324` and `:352`), serializing only
the changed tag. Group size does not force a full serialization. Only
`state_change_hash` does.

Implementing `resource_group_write_set` properly is a **parallel-path
correctness requirement**, not a sequential speed lever. The stubs in
`mono_move/mod.rs` are fine for sequential and are not fine for Block-STM.

---

# Appendix C: what a real 10x would take

Recorded so the next attempt starts here instead of at E1.

Final state: clob 8.17x, aave 7.16x. The gap to 10x is 11.5 profiled us/txn on
clob and about 20 on aave.

What is left, ranked, with what it is actually worth:

| item | clob | aave | recoverable |
|---|---|---|---|
| `read_group_from_storage` | 4.28 | 5.75 | ~0 — cold disk, both VMs |
| RocksDB / pread / LZ4 | 5.92 | 5.18 | ~0 — both VMs, mono already at 0.95x |
| `merge_group` re-encode | 2.51 | 5.73 | ~half, by splicing |
| `mach_absolute_time` after E21 | 1.59 | 2.33 | most, on aave |
| `StateKey` registry probe | 1.35 | 1.34 | most |
| `NodeRef::get_strong` weak upgrades | 1.29 | 0.96 | unclear |
| `StateSlot::clone` + `try_insert` | 0.54 | 1.34 | unclear |
| loader, after the call-site cache | 8.16 | — | ~1.5 |

Summing the genuinely recoverable column gives roughly 3 profiled us/txn on clob
and 5 on aave — about 1.5 to 2 measured. Against a gap of 11.5 and 20.

**So 10x is not reachable by more experiments of this kind.** The remaining time
is either shared with legacy (disk, decompression, the state-view layer) or
structural (a legacy write set per transaction, a ledger hash per transaction, a
per-transaction loader).

The three things that would actually move it, none of which are small:

1. **Epoch-sealed code** — version code by epoch rather than per module, so the
   loader runs per epoch instead of per transaction. The loader is 8.16 us/txn
   and almost all of it is re-derivation.
2. **A native write set** — stop materializing a legacy `WriteSet` per
   transaction. This is the "structural cost" section. It requires changing what
   the executor consumes, and `state_change_hash` has to be computed from
   whatever replaces it.
3. **Attack the non-VM 34%** — outside the block executor entirely. Nothing in
   this log touches it, and it is the largest single block of e2e time.

A warning for whoever picks this up: the profile over-states short hot leaves by
2-3x. Every number above that came from a profile rather than a ceiling arm
should be treated as an upper bound. Twice in this log a profile said 4% and the
ceiling arm said 1.5%.
