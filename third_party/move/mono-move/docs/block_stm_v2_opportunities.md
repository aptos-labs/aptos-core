# Block-STM v2: where the time goes and what to change

Companion to `scaling_log.md`. Measurements are from the scaling sweep
(c7a.16xlarge, blocks of 1000, 16 threads unless stated), per transaction.
MonoMove is the VM unless stated; V1 numbers are given where the point is
that the cost is shared.

## What the workloads look like to the scheduler

| workload | aborts/txn at 16T | why |
|---|---|---|
| no-op, transfer, account generation, 1 per sender | 0.03–0.04 | independent; the aborts are group-slot and store-creation collisions |
| the same, 5 per sender (the e2e harness shape) | 2.0 (both VMs) | a chain of five through the sender's account and store; cascade 4+3+2+1 per chain |
| order book, one market | 6.1–6.3 (both VMs) | every transaction read-modify-writes the market; serial chain, 140 µs per link |

On the independent workloads v2 pays only bookkeeping. On the chained ones
it pays re-executions, and on the serial one it also pays 14 idle workers.

### Bookkeeping, independent workloads (transfer, MonoMove, worker CPU µs/txn)

| | 2T | 16T |
|---|---|---|
| worker CPU total | 115 | 167 |
| mvhashmap group map (`set_raw_base_values`, `group_members_at`, `write_v2`, reads) | 10.8 | 16.6 |
| mvhashmap data map (`write_v2`, `set_base_value`) | 2.6 | 3.9 |
| of which DashMap shard locks and entry mutexes | 7.3 | 9.9 |
| read-dependency registration | 0.8 | 2.0 |
| scheduler `next_task` | 0.4 | 1.6 |
| kernel (IPIs, page faults, futex) | 15 | 28 |
| allocator | 10 | 16 |

Two of the DashMap costs are per block, not per transaction: `set_raw_base_values`
(5.9) inserts every member of a group the first time the block reads it,
`set_base_value` (1.4) does the same for plain resources. V1 pays the same
plus `finalize_group` (9.4) because it rebuilds every written group's blob per
transaction.

### Phases, order book, MonoMove, 16T (worker CPU µs/txn)

| phase | 2T | 16T |
|---|---|---|
| execute_vm | 211 | 754 |
| scheduler (spinning in `next_task`) | 2.5 | 929 |
| execute_apply | 14.5 | 93 |
| materialize | 35 | 56 |
| commit_hooks | 4.8 | 52 |

Throughput is identical (7.1k) at 2 and 16 threads: the chain bounds it, and
the other 14 workers spend 929 µs/txn polling the scheduler and 540 µs/txn
executing incarnations that are thrown away. V1: 6967 µs/txn spinning.

## D1. Predictive stalls on abort (prototype on the branch, `BLOCK_STM_PREDICTIVE_STALL=1`)

v2 does not mark an aborted incarnation's writes (no ESTIMATE as in v1); a
reader that re-executes reads them and is aborted again when the writer
re-executes. The stall mechanism only learns a dependency after it has been
invalidated once by an incarnation above 0, so on a chain of five the second
wave of re-executions is 3+2+1 wasted incarnations per chain, 60% of the
aborts.

Two variants that cannot work, found by measurement (0.00 stalls on every
workload):

- "Hold the aborted reader for the next writer between the aborting writer
  and the reader at that key." There never is one: a write at `i` only
  splits off readers registered on the entry at or below `i`, and an entry
  between `i` and the reader would itself have invalidated the reader when
  it was written. The only exception is a write that passed value
  validation (V1 only).
- "A writer aborted while it ran stalls the readers its applied writes
  invalidated." Both VMs stop an aborted incarnation through the kill
  switch, so it applies nothing and invalidates nobody.

The variant that matches the cascade: when a transaction is aborted, the
readers currently registered on its *existing* entries (the ones its last
incarnation wrote, found through its recorded write keys and
`VersionedData::readers_of`) are recorded as its dependencies and stalled
until it has executed again. Those entries are about to be replaced, so a
reader that re-executes against them is wasted. The record is made inside
the abort loop in index order, right after the transaction's own
`finish_abort`, so none of its readers (all above it) can be scheduled
before the stall is in place. A wrong prediction (the next incarnation
writes the same value) costs a delay, not correctness.

Expected: chains 2.0 → 1.4 aborts/txn without sender hints (the first wave
and the second wave's first abort remain), order book 6.3 → about 2.
Measured: see `scaling_log.md`.

## D2. Kill switch for MonoMove (done, measured)

16% of VM CPU on the order book, no wall-clock change, zero interrupts on
chains: aborts arrive after the incarnation completed. Keep it (it is free),
but it is not a throughput lever.

## D3. Pre-execution dependency hints (next prototype)

The first wave of a chain is unavoidable today: txns `i..i+4` of one sender
start together and all read the base value. But the chain is known before
execution: same sender, consecutive sequence numbers, and the sender's
`Account` and primary APT store are read-modify-written by every one of
them. Give the scheduler a `pred: Option<TxnIndex>` per transaction (the
previous transaction of the same sender in the block, from one pass over
the block), and do not put a transaction with a `pred` into the initial
execution queue; insert it when `pred` finishes incarnation 0. That is the
same shape as the existing `executed_once_max_idx` deferral, per sender
instead of global, and it needs no change to the "incarnation 0 is never
stalled" invariant since the transaction is simply not scheduled yet.

Expected with D1: chains to about 0 aborts/txn, recovering the 25–31% the
e2e harness loses at 16T on both VMs. Orders of the same market have
different senders, so the order book is untouched; it needs D1.

Generalization: the hint is a set of keys the transaction will RMW for
sure; the sender keys are the first instance, an entry function's declared
"hot objects" (market, pool) could be another, supplied by the VM from the
transaction payload without executing it.

## D4. Validation by behaviour (read kinds)

v2 invalidates every registered reader of an entry when the value changes
(`eq_value`, bytes for V1, always "changed" for MonoMove). Reads that did not
depend on the whole value are invalidated for nothing:

- **exists**: `exists<T>(addr)` should only be invalidated when the slot
  flips between present and absent. Needs a kind bit per registered reader
  (`RegisteredReadDependencies` has the TODO) and, at invalidation time,
  `still_valid = exists(old) == exists(new)` for those readers. V1 already
  has the three-level read kind (exists / metadata / value) in its captured
  reads; v2's push validation collapses it to value.
- **metadata**: groups already do this (`ONLY_COMPARE_METADATA`).
- **field projection**: a reader that touched fields `{a, b}` of a struct is
  still valid when only field `c` changed. MonoMove can get the field set
  at lowering time: for each `borrow_global` site, the field paths reachable
  from the reference, conservatively the whole value when the reference
  escapes to a call without a summary. Zero runtime cost, one bitmask per
  read. Validation compares the projected byte ranges (MonoMove values are
  flat for pointer-free fields, so this is a few `memcmp`s with the layout).
- **value equality for MonoMove** (`eq_value` returns false today):
  `value_cmp::equals` with layouts from the global context (carry an
  `Arc<GlobalContext>` in `MonoValue::Write`). Worth at most the gap
  between V1 and MonoMove abort counts, 2–4% on these workloads.

Where it pays: transactions that read configuration of a hot object while
others mutate its state (FA metadata and supply, pool fee parameters and
reserves, collection max supply and current supply). None of the four
sweep workloads has that shape; every conflict in them is a true RMW. Worth
building a workload for before building the mechanism.

## D5. Commutative writes

The limit of D4: a write that is `old ⊕ delta` with no read of the value
has no dependency at all. V1 has this as delayed fields (aggregators), which
is why metered V1 does not serialize on the APT supply and metered MonoMove
does (4 aborts/txn). Out of scope per the brief, but it is the single
largest item for metered MonoMove.

## D6. Scheduling window

With D1 the cascade is gone but the first wave is still one wasted
incarnation per transaction on a serial chain, and 14 workers spin. An
adaptive window (`min_never_scheduled_idx - next_to_commit_idx <= W`, with
`W` shrinking when the abort rate is high and growing back when it is low)
bounds the waste; on the independent workloads it must stay at `num_txns`.
It does not raise the chain bound (7.1k TPS on the order book); it only
returns CPU to the rest of the node.

## D7. Idle workers

`next_task` returning `NextTask` is a spin: 929 µs/txn (MonoMove) and
6967 µs/txn (V1) of worker CPU on the order book at 16T. Park after N
consecutive empty polls and wake on `finish_execution` / `end_commit`
(v2's "notify waiting workers" TODO). Matters most in pipelined mode, where
the commit stage is the bottleneck and these cores are taken from it.

## D8. Bookkeeping

- `set_raw_base_values` and `set_base_value` (7.3 µs/txn) go away if the
  mvhashmap is kept across blocks with committed values as the base (the
  execution-pool design in `scaling_log.md`); short of that, the group base
  insertion can skip members the block never reads.
- Read-dependency registration under the entry mutex: 2 µs/txn, +150% from
  2T to 16T. A lock-free registration (per-entry append-only list with a
  reader-side CAS, drained under the lock only by the writer) removes the
  reader-side lock; small in absolute terms.
- `materialize` inflates +76% from 2T to 16T on no-op (5.4 → 9.5) for work
  that is embarrassingly parallel (per-transaction output conversion):
  allocator and memory-bandwidth effects. Not scheduler work; left for the
  materialization pass.
- Group writes as members (B4): V1's `finalize_group` is 9.4 µs/txn, MonoMove
  pays `group_members_at` 4.9 to assemble the group blob per transaction.
  Writing only the changed member and assembling the blob once per group
  per block at commit removes both. Protocol change: the write set of a
  transaction no longer contains the group blob, so the transaction output
  hash changes unless the blob is assembled before the output is hashed,
  which is at `materialize` (post-commit, once per transaction) — the saving
  is then only the speculative assembly, not the final one. Needs a
  write-up before code.

## Order of attack

1. D1 (done, measuring) and D3 (next): both VMs, the e2e harness shape, no
   protocol change.
2. D7: cheap, returns CPU on every conflict-heavy block.
3. D6: after D1/D3, to see what waste remains.
4. D4 exists-kind: small and general; the rest of D4 after a workload that
   needs it exists.
5. D8 as part of the execution-pool / mvhashmap-reuse work.
