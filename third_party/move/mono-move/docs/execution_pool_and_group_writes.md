# Cross-block execution state and member-level group writes

Two changes that reach beyond one block's execution. Neither is built;
this records what each would take, what it would buy on the measured
workloads, and where it changes the protocol. Numbers are from the scaling
sweep (`scaling_log.md`), 16 threads, blocks of 1000, per transaction.

## 1. Execution pool: keep the multi-version map across blocks

### Today

Every block builds a fresh `MVHashMap` and `SchedulerV2`, reads every key it
touches from the state view the first time (memorized reads → pending
overlay → speculative delta → hot state → RocksDB), inserts it as a base
entry (`set_base_value`, `set_raw_base_values` for a group's members), and
drops the map at the end. Costs, MonoMove, transfer:

| | 2T | 16T |
|---|---|---|
| `set_raw_base_values` + `set_base_value` (worker CPU) | 5.3 | 7.3 |
| state view lookups behind them (`get_state_view`, memorized reads) | ~1 | ~1 |
| per-block map and scheduler construction and drop | 0.27 ms/block | 0.27 ms/block |

About 5% of MonoMove's worker CPU at 16 threads, the same absolute amount
on V1 (3% of its). The base values of a key the previous block wrote are
exactly the values that block produced; the state view finds them in its
overlay (B1) and the map inserts them again.

### Change

One map per fork, kept across blocks. Transaction indices are offset by the
number of transactions already in the pool, so a new block's transactions
sit above the previous block's entries and read them as the latest version
below, with no base insertion. Dependencies registered on an old entry by
committed transactions are inert (always below any new writer). The
scheduler stays per block; the executor maps its indices through the
offset. Flush the pool when the parent block is not the pool's tip (fork
switch, same rule as the pending-writes overlay), at a reconfiguration, and
when it exceeds a memory budget; prune a key's versions below the committed
watermark on access, keeping the latest.

### Hazards

- Delayed fields: a committed entry holds the exchanged value (with delayed
  field ids), not the materialized bytes; ids are allocated per block. The
  post-commit materialization has to write the materialized value back into
  the entry as a storage-shaped base, or the next block reads ids it cannot
  resolve. MonoMove has no delayed fields and is unaffected.
- Group sizes and tag supersets are per key and carry over; the group-size
  validation already compares per version.
- Module publishing goes through the module cache, which has its own
  cross-block rules; not touched.
- Memory: unbounded without pruning; the prune-on-access rule keeps one
  version per key plus the live block's.

### What it buys

5–6% of the execution stage on MonoMove, 3% on V1, and the per-block
construction cost. It is the natural host for the pending-writes overlay
(the map is the overlay) and would remove the 3 µs/txn overlay build B1
pays. It does not change what the commit stage does, which is the
end-to-end bottleneck in pipelined mode.

## 2. Member-level group writes

### Today

A transaction that changes one member of a resource group writes the whole
group blob: its output's `WriteSet` has the group's `StateKey` mapped to the
BCS map of every member. The block executor assembles that blob after the
transaction commits, from the members' latest values in the group map
(`finalize_group` on V1, 9.4 µs/txn; `group_members_at` plus serialization
on MonoMove, 4.9), once per transaction that touched the group. The state
tree, the ledger's write-set column, state sync and the API all see the
blob.

Output bytes per transaction by kind of write (`aptos_processed_txns_output_bytes_by_kind`,
key plus value bytes; events are their BCS size):

| 16T | resource | group blobs | events | total (BCS) |
|---|---|---|---|---|
| no-op, V1 (metered) | 230 | 1070 | 106 | 1481 |
| no-op, MonoMove (unmetered) | 230 | 0 | 106 | 352 |
| transfer, V1 | 230 | 1399 | 305 | 2037 |
| transfer, MonoMove | 230 | 661 | 305 | 1221 |
| account generation, V1 | 230 | 1399 | 106 | 1839 |

A metered no-op writes 1070 bytes of group blobs for its fee alone: the
sender's store group (`ObjectCore` plus `FungibleStore`, about 330 bytes
with the member tags and key) and, by the remaining size, the APT
metadata object's group at `0xa`, whose `ConcurrentSupply` member changes
with every burn and drags the `Metadata` member along. The members that
change are a few dozen bytes each. Member-level writes would cut V1's
write set by 60–70% on these workloads; the unmetered MonoMove numbers
show the shape (its transfer carries two store blobs, 661 bytes, for two
balance changes of 8 bytes each).

### Change

The transaction output carries the member write (`group key`, `member tag`,
`bytes`) instead of the blob. The blob is assembled once per group key per
block, from the block's base blob and the last write to each member, where
the block's state update is built (`TransactionsToKeep::index` /
`State::update`), which already keeps only the last write per key in a block.

### What it buys

- The per-transaction assembly inside Block-STM: 9.4 µs/txn of V1 worker
  CPU, 4.9 of MonoMove's (2–3% of each).
- Smaller write sets: a member instead of the blob in every transaction's
  output, so less to hash, serialize, store in the ledger and ship to
  state-syncing nodes. A sender's and receiver's store group is written by
  every transfer; each blob is the store plus the object core, written
  twice (once per side) per transfer. The exact share needs the by-kind
  counter from the next run.
- Commit-stage bytes: the state tree still stores one blob per group key
  per block (the last), unchanged; the ledger write-set column shrinks by
  the 60–70% above, and so does what state sync ships and what the
  transaction-info hash covers. The commit stage is the end-to-end bound in
  pipelined mode (`scaling_log.md`), so this is where the saving would show
  as TPS.

### Protocol change

- `TransactionInfo::state_change_hash` is the hash of the write set, so
  the format of the write set is consensus-visible: a new `WriteOp` or
  `StateKey` shape for a member write, gated on a feature flag that every
  node observes at the same version.
- State at a version in the middle of a block is no longer a blob per
  group in the write set; anything that reconstructs state from write sets
  (state sync's apply path, the replay tools, `get_state_value_by_version`
  over the ledger) has to apply member writes onto the previous blob, with
  the same assembly code as the executor.
- The API already presents group writes as per-resource changes, so its
  shape does not change; the indexer's write-set parsing does.
- Gas: IO gas for a group write is charged on the group size estimate, not
  the serialized output; unchanged.

### Prototype scope

There is no flag-only prototype: a transaction's blob must reflect the
group as of that transaction, so assembling once per block changes every
writing transaction's output and therefore its hash. The change is the
format change. What is measurable before it: the assembly CPU (above) and
the share of output bytes that are group blobs (the by-kind counter), which
bound what the change can buy inside the block executor and on the commit
path respectively.
