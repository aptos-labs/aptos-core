# Native Position — Framework Design

## Context

Exchange user and position data currently lives in Move resources. `UserPositions` stores positions in a `BigOrderedMap<Object<PerpMarket>, PerpPosition>` — a B+tree backed by table storage, where each tree node is a separate table entry (MVHashMap key). Accessing a single position requires B+tree traversal through multiple table reads + BCS deserialization per node. Any two TXs touching the same user's positions conflict because they share the BigOrderedMap root.

Goal: framework-level native store with per-entry MVHashMap storage — eliminating B+tree traversal and BCS overhead, reducing conflicts to only position lifecycle operations, and enabling native computation.

Deferred: market→accounts reverse index (requires `SetAggregator`), on-chain liquidation discovery.

## Storage Layer

### New StateKey Variant

```rust
// types/src/state_store/state_key/inner.rs
pub enum StateKeyInner {
    AccessPath(AccessPath),
    TableItem { handle: TableHandle, key: Vec<u8> },
    Raw(Vec<u8>),
    // Persisted: one entry per (exchange, account, market) position
    Position {
        exchange_id: u32,
        account: AccountAddress,
        market: AccountAddress,
    },
    // Virtual: per-(exchange, account) market set used for Block-STM conflict tracking.
    // Never persisted to DB, never a JMT leaf, never in state-sync or backup.
    // Derived at startup from the Position entries and maintained in memory.
    UserMarkets {
        exchange_id: u32,
        account: AccountAddress,
    },
}
```

### Dedicated RocksDB Database (position_db)

Exchange-store data lives in its own top-level RocksDB instance (`position_db`), NOT in `state_kv_db` / `hot_state_kv_db`. Single-tier: no hot/cold split. Rationale:
- Tuning profile is different from general state (small fixed-size values, high update rate, per-account prefix queries) — a separate DB avoids perturbing the main state tuning.
- Isolated pruning, backup, and migration cadence.
- Avoids interaction with the hot-state eviction policy.

Layout inside `position_db`:
- `position_value` CF: versioned Position values keyed by encoded StateKey bytes (see Multi-version storage for the full schema).
- `stale_position_value_index` CF: stale-value index driving the pruner.
- Metadata CF: `exchange_id` allocation registry, schema version, and bookkeeping (non-versioned singleton state).
- Prefix bloom filter + prefix extractor on `[tag=Position:1][exchange_id:4][account:32]` (37 bytes) for per-account scans.
- Tuned for small fixed-width values + frequent updates.
- Only Position entries persist — UserMarkets is derived in memory (see Derived State).

### Dedicated State Merkle Tree (position_merkle_db)

Authentication is also split out: Position keys are NOT inserted into the main `state_merkle_db`. Instead, a separate JMT (`position_merkle_db`) covers them. `UserMarkets` is virtual and has no JMT leaves at all. Rationale:
- Write amplification isolation: per-block Position leaf churn does not bloat the main state tree's versioned node cache or contend on its write batch.
- Independent pruning cadence: Position history can prune more aggressively than general state (or vice versa) without coupling.
- Backup / state-sync chunking can stream the two trees in parallel.

Isolation is **not total**: per-exchange position-count aggregators live in main state (see Scaling thresholds), so every block that creates or removes Positions also produces one aggregator materialization write per exchange in `state_merkle_db`. This is bounded (one write per exchange per block, not per TX) and doesn't contend across TXs thanks to delayed-field semantics — but it does mean `main_state_root` changes on every block that touches Positions.

Layout:
- `position_merkle_db` has the same physical schema as `state_merkle_db` — versioned JMT nodes keyed by `NodeKey`, stale-node index, etc. — reusing the existing `schema/jellyfish_merkle_node` and `schema/stale_node_index` schemas.
- Only `StateKeyInner::Position { .. }` entries become leaves. `UserMarkets` is filtered out at the commit boundary.
- Leaf value hash: `hash(encode(StateKey::Position { .. }) || state_value_bytes)`, identical to how `state_merkle_db` hashes main-state leaves.
- A single `JellyfishMerkleTree` implementation serves both trees; the DB handle parameterizes which one is used.

### Composite State Root

The block-level state root becomes a composition of the two subtrees:

```
state_root_hash = H("APTOS::StateRoot" || main_state_root || position_root)
```

- `main_state_root`: root of `state_merkle_db` (as today).
- `position_root`: root of `position_merkle_db`. When there have been no Position writes ever, this is a fixed constant `EMPTY_POSITION_ROOT` (the empty-JMT root).
- `state_root_hash` goes into `TransactionInfo` / `LedgerInfo` where the current single root goes today.
- State proofs for a Position key return: `(value, leaf_proof_in_position_tree, position_root, main_state_root)` — the verifier checks the JMT inclusion against `position_root`, then composes both roots and checks against the ledger-info-signed `state_root_hash`.
- Before the feature flag turns on: `state_root_hash = main_state_root` (legacy). Flip to composite form at the feature-flag activation version so pre-upgrade proofs stay valid against the old format.

### Composite-Root Activation

The switch from legacy to composite root format is versioned. Clients must know the activation version to verify proofs correctly.

- **Activation version** is the first `Version` at which the `NATIVE_EXCHANGE_STORE` feature flag is active. It is recorded on-chain in the feature-flag framework state (read via `aptos_framework::features::native_position_activation_version()`).
- **Clients** (light clients, SDK, bridges, indexers) query the activation version from their trusted full node at connection time and cache it. Proofs at `Version V` are verified against:
  - If `V < activation`: legacy format — `state_root_hash` is exactly `main_state_root`. Verifier uses the existing proof check.
  - If `V >= activation`: composite format — `state_root_hash = H("APTOS::StateRoot" || main_state_root || position_root)`. Verifier does an extra hash composition step before comparing to the ledger-info-signed root.
- **Proof payloads** change shape post-activation. The proof struct gains optional fields:
  ```rust
  pub struct StateProof {
      // ... existing fields ...
      pub position_root: Option<HashValue>,       // None iff version < activation
      pub activation_version: Version,            // for client-side sanity checks
  }
  ```
  Pre-activation proofs carry `position_root = None`. Post-activation proofs always carry `position_root = Some(..)`, even for non-Position keys (because composition needs both roots regardless of which subtree the key lives in).
- **Non-Position key proofs post-activation** carry `main_state_root` + inclusion proof in the main tree, plus `position_root` (uninspected, just included for composition). Verifier: compose → compare.
- **Position key proofs** always carry `position_root` + inclusion proof in the position tree, plus `main_state_root` (uninspected). Same composition step.
- **Domain separator `APTOS::StateRoot`** is immutable once activated. No re-derivation ever.
- **Re-orgs across activation** cannot happen by construction (feature flags advance monotonically at epoch boundaries), so there's no need to verify proofs against a superseded format.
- **SDK migration:** bump the major version of client SDKs at activation; old SDKs cannot verify post-activation proofs and must be upgraded. Indexers and bridges are responsible for their own SDK bumps before the activation version lands.

### Derived State (UserMarkets)

`StateKeyInner::UserMarkets { exchange_id, account }` is a virtual key used by Block-STM to track conflicts on the per-user market set without paying for DB writes or JMT updates on every position create/remove.

- **Never written to DB.** At commit time, the write-set is partitioned by key variant: Position writes flow into `position_db` + `position_merkle_db`; UserMarkets writes update only the in-memory derived index.
- **Never a JMT leaf.** State root commits to Positions only. A consumer wanting to prove "user A has markets {X, Y, Z}" must prove each Position[A][m] individually — there's no UserMarkets leaf to prove against.
- **Derived at startup.** During the in-memory cold-load, after Positions are loaded the loader walks them once to build `DashMap<(exchange_id, account), HashSet<market>>`. Linear in Position count, piggybacks on the scan we already do.
- **Maintained at commit.** Every Position create or delete in a block produces a paired UserMarkets write (add/remove market) through the normal change-set path. The commit applier routes Position writes to DB+JMT and UserMarkets writes to the in-memory index.
- **Block-STM conflict behavior unchanged.** UserMarkets is a first-class MVHashMap key, so read/write conflicts between "list my markets" and "create/remove a position" are caught with the same per-key dependency machinery as any other state.
- **Crash safety by construction.** If the in-memory view is ever wrong (bug, partial recovery, process restart), a full rebuild from Positions restores the invariant: `UserMarkets(A) = { m | Position[exchange, A, m] exists }`.

### Change-Set Routing

Because `UserMarkets` is virtual, leaking it into `WriteSet` would be a data-corruption bug: `WriteSet` is shipped over state sync, persisted to `ledger_db` as part of the transaction record, and replayed on other nodes — all of which would then try to durably apply a key that shouldn't exist on disk.

**Split `VMChangeSet` explicitly.** Add two new buckets alongside the existing `resource_write_set` / module / event / etc. — one for Position writes, one for UserMarkets writes:

```rust
pub struct VMChangeSet {
    resource_write_set: BTreeMap<StateKey, WriteOp>,     // existing — resources only (AccessPath keys)
    module_write_set: ...,
    aggregator_v1_write_set: ...,
    // ... other existing buckets ...
    position_write_set: BTreeMap<StateKey, WriteOp>,            // NEW — Position keys only
    position_ephemeral_writes: BTreeMap<StateKey, WriteOp>,  // NEW — UserMarkets keys only
}
```

Bucket invariants:
- Only AccessPath keys in `resource_write_set`.
- Only `StateKeyInner::Position` in `position_write_set`.
- Only `StateKeyInner::UserMarkets` in `position_ephemeral_writes`.

At the `VMChangeSet → TransactionOutput.write_set` materialization boundary (runs at TX-end):

1. `resource_write_set` + `position_write_set` + other existing persisted buckets → merged into the flat `WriteSet` inside `TransactionOutput`. Both resource and Position entries are serialized into the same list; they're differentiated later by `StateKeyInner` variant.
2. `position_ephemeral_writes` is filtered out. UserMarkets writes never enter `TransactionOutput` / `WriteSet` / ledger_db / state sync.

The bucket split exists to make this filter trivial (include-by-bucket rather than scan-and-strip-by-variant) and to give the VMChangeSet API type-safe iteration — no other purpose.

At commit time, the `PositionCommitter` reads `TransactionOutput.write_set` (flat) and dispatches by `StateKeyInner` variant:
- AccessPath → `state_kv_db` + `state_merkle_db` (as today)
- Position → `position_db` + `position_merkle_db` + in-memory Position map
- UserMarkets → **cannot appear** (was excluded at materialization; panics if seen)

A defensive `StateKeyInner` custom `Serialize` impl should reject `UserMarkets` too — if some code path accidentally tries to `bcs::to_bytes(&state_key)` on a UserMarkets variant, the filter-by-bucket invariant is no protection, and `derive(Serialize)` would silently emit bytes with variant index 4. Explicit rejection at the serde layer is belt-and-suspenders on top of the bucket filter.

`TransactionOutput` carries only the persisted `WriteSet`. How UserMarkets is rebuilt depends on replay mode:

- **Re-execution replay** (e.g., smoke tests, block-level catchup, governance simulation): TXs run end-to-end, natives produce UserMarkets `WriteOp`s into `position_ephemeral_writes`, commit applier updates the in-memory index. Same path as normal execution.
- **Apply-WriteSet replay** (state-sync's normal mode, and how fullnodes catch up via chunks): natives never run. Only Position writes arrive (UserMarkets was stripped at the source). The in-memory index is rebuilt from Positions by the startup load at the end of sync (or incrementally as chunks land — see Cold Start scenarios). UserMarkets correctness relies on the derivation invariant: `UserMarkets(A) = { m | Position[exchange, A, m] exists }`.

Both paths converge to the same in-memory state. Derivation from Positions is deterministic, so re-executed UserMarkets values equal derived-from-scan UserMarkets values, byte for byte.

An assertion in the WriteSet construction path catches any `StateKeyInner::UserMarkets` that reaches it: panic in debug, metric-and-skip in release. This is load-bearing — the invariant must be enforced, not assumed.

**State-sync interaction.** State-sync chunk streams carry Positions (shipped through ledger_db + state_merkle_db) but never UserMarkets. Receivers verify Position chunks against `position_root`, apply to `position_db` + `position_merkle_db`, and regenerate UserMarkets locally during the startup load.

**Commit applier** — concretely, this is the block-executor's commit phase (the code path leading into `StateStore::save_transactions` in aptos-core today). A new `PositionCommitter` component sits in that phase and runs in this order:
1. Reads the block's accumulated `position_write_set` — routes each `WriteOp` into the `position_db` + `position_merkle_db` batch.
2. Updates the in-memory `positions` DashMap from the same batch.
3. Reads the block's accumulated `position_ephemeral_writes` — applies each UserMarkets `WriteOp` to the in-memory `user_markets` DashMap.

Ordering matters defensively: Positions are applied before UserMarkets so that at no intermediate moment is the invariant `UserMarkets(A) ⊇ { m | Position[exchange, A, m] exists }` violated if instrumentation peeks mid-commit. (The commit itself is atomic from the outside, so external observers never see the intermediate state, but defensive ordering is free and makes debugging easier.)

RocksDB WAL / fsync mode matches the existing `state_kv_db` / `state_merkle_db` policy (WAL durable sync on commit) — `position_db` and `position_merkle_db` inherit the same durability guarantees.

Per-exchange position counters are NOT managed by the commit applier. They are `AggregatorV2` values in main state, incremented/decremented by the natives themselves (`create_position.try_add(1)`, `remove_position.sub(1)`) and committed through the normal main-state write path. This keeps the counters authenticated (they're part of `main_state_root`) and avoids a separate sidecar persistence for them.

The ephemeral-writes bucket never leaves this component — it's not attached to `TransactionOutput`, not shipped to state sync, not persisted.

### UserMarkets Value Encoding

`StateKeyInner::UserMarkets` entries in MVHashMap use `StateValue` (for type uniformity with the rest of MVHashMap), but with a custom payload:

```
[num_markets:u16 LE:2][market_1:32][market_2:32]...[market_N:32]
  total = 2 + 32*N bytes
  markets sorted ascending (lexicographic over 32-byte addresses)
```

Matches the old UserAccount V1 layout minus the version tag byte (no versioning needed since it never persists). The sorted invariant makes add/remove O(log N) via binary search and keeps MVHashMap read-your-writes semantics stable (same set → same bytes → same hash in MVHashMap's internal tracking).

Deletion of the entire entry (empty markets set) uses `WriteOp::Deletion`. At the commit applier this means **remove the key from the in-memory `user_markets` DashMap** — there is no RocksDB row to tombstone, no JMT leaf to delete, and no stale-index entry to emit. Creation of a UserMarkets entry for a user that previously had none uses `WriteOp::Creation`, which inserts into the DashMap. Modification (add or remove a market, set stays non-empty) uses `WriteOp::Modification`, which replaces the DashMap value.

### Read / Write Path

- **Reads (Position):** `LatestView` / `StorageAdapter` dispatches reads with `StateKeyInner::Position { .. }` to the in-memory Position map (populated at startup + maintained at commit). `position_db` is the durable backing store, not the hot read path. For proof-carrying reads, dispatch goes to `position_merkle_db`.
- **Reads (UserMarkets):** Always served from the in-memory derived index. Never touches disk.
- **Writes:** At commit time, the change set is partitioned by key variant.
  - `StateKeyInner::Position` writes → `position_db` + `position_merkle_db` + in-memory Position map. Both JMT updates (main + position) happen in parallel; their new roots compose into the block's state root.
  - `StateKeyInner::UserMarkets` writes → in-memory derived index only.
- **State sync:** two independent chunk streams (main state and Position state). Sync client verifies each against its respective root, then checks the composed root matches the signed `LedgerInfo`. UserMarkets is NOT shipped — it's rebuilt post-sync from the applied Positions.
- **Backup & restore (version-sync invariant):** a backup snapshot consists of `state_kv_db` + `state_merkle_db` + `position_db` + `position_merkle_db` dumps, and all four MUST be captured at the same `Version`. Otherwise restore produces an inconsistent state root that doesn't match any signed `LedgerInfo`, and the node will fail verification at the first block it tries to process post-restore. Backup tooling takes a consistent-snapshot barrier across all four DBs (same pattern as today's ledger_db + state_kv_db + state_merkle_db coordination, extended by one). The resulting snapshot archive records the captured Version as metadata so restore can refuse to mix backups from different points in time.
- **Crash consistency across 4 DBs.** Commit spans four separate RocksDB instances. The existing 3-DB coordination protocol (ledger_db first, then state DBs, with replay-from-ledger on restart for partial-commit recovery) extends naturally to the fourth. If a crash lands between `position_db` commit and `position_merkle_db` commit (or vice versa), restart replays the affected block from `ledger_db` — same fallback path as today's state DBs, just one more participant.

### Key Layout

Two variants. Only `Position` is persisted; `UserMarkets` is in-memory.

```
StateKeyInner::Position     (persisted)
  encoded: [StateKeyTag::Position:1][exchange_id:4 BE][account:32][market:32]   = 69 bytes

StateKeyInner::UserMarkets  (virtual, in-memory only)
  identity: (exchange_id: u32, account: AccountAddress)
  no physical encoding — MVHashMap keys by the struct directly
```

Each entry is its own MVHashMap key. Block-STM tracks reads/writes per entry. The commit boundary routes by variant (see Derived State above).

## Storage Format

End-to-end format for Position entries (UserMarkets is in-memory only, no wire format). Three layers: physical StateKey encoding, StateValue wrapping, and the compact value payload.

**Endianness convention:** little-endian in value payloads (matches Move VM `u64`/`u128` serialization); big-endian in the physical StateKey encoding (so RocksDB lexicographic order matches numeric order of `(exchange_id, account, market)`).

### Physical StateKey encoding (Position)

Produced by `StateKeyInner::encode()` and used as the RocksDB row key prefix, the MVHashMap key, and the preimage for the JMT leaf key hash.

```
[StateKeyTag::Position : 1 byte][exchange_id : 4 bytes BE][account : 32 bytes][market : 32 bytes]
= 69 bytes total
```

`StateKeyHash = SHA3-256(encoded_bytes)` — used as the JMT leaf key inside `position_merkle_db`.

UserMarkets has no physical encoding — MVHashMap keys by the `{exchange_id, account}` struct directly, and no part of the system ever serializes it.

### StateValue wrapping

Position values flow through the standard `StateValue` type so they interoperate with the existing change-set / WriteOp / JMT plumbing:

```rust
StateValue {
    data: Bytes,              // the compact payload below
    metadata: StateValueMetadata,
}
```

- `metadata` carries the storage-slot deposit + creation time, just like resource/table writes. On create: deposit = `PositionGasParameters::storage_slot.base`. On delete: deposit refunded. Deposit rate is independent of main-state rate (own gas schedule).
- Wire form is `PersistedStateValue::WithMetadata { data, metadata }` (BCS). This is what ends up in RocksDB.
- The JMT leaf value hash is `SHA3-256(bcs(PersistedStateValue))`, matching main state.

UserMarkets entries also use `StateValue` in MVHashMap — see UserMarkets Value Encoding above. The difference is that UserMarkets StateValues never reach `PersistedStateValue` / RocksDB / JMT; they are consumed by the commit applier and discarded.

### Compact value payload (Position)

Payload is the raw bytes placed in `StateValue.data`. NOT BCS — fixed-width, hand-rolled. First byte is a variant tag for forward-compat evolution.

```
PerpV1   tag=0x00:
  [tag:1][size:u64 LE:8][is_long:u8:1][entry_px_times_size_sum:u128 LE:16]
  [avg_entry_px:u64 LE:8][user_leverage:u8:1][is_isolated:u8:1]
  [funding_index:i128 LE two's-complement:16]
  [unrealized_funding_before:i64 LE two's-complement:8]
  [timestamp:u64 LE:8]
  total = 68 bytes

SpotV1   tag=0x01:
  [tag:1][size:u64 LE:8][is_long:u8:1][entry_px_times_size_sum:u128 LE:16]
  [avg_entry_px:u64 LE:8][timestamp:u64 LE:8]
  total = 42 bytes
```

`is_long`/`is_isolated` stored as 1 byte (0 or 1); any non-zero byte on read is an error (reject rather than coerce) so we catch corruption cheaply.

### Multi-version storage

Position data is versioned by the global transaction `Version` (monotonic `u64` across the chain), same as main state. There is no separate version counter. UserMarkets is not versioned — it's a derived view.

#### Versioned value CF (`position_value`)

Keyed by the encoded Position StateKey bytes (not hash) so prefix scans on `(exchange_id, account)` are possible for cold-start / admin tooling:

```
Key:   (encoded_state_key: Vec<u8>, !version: u64 BE)
       // encoded_state_key = [tag=Position:1][exchange_id:4 BE][account:32][market:32]
       // fixed length: 69 bytes
Value: Option<StateValue>   // Some = write, None = tombstone
```

Why bytes, not hash:
- Position key space is bounded and application-scoped — DoS pressure that motivated hash-keying for main state doesn't apply.
- Fixed width (69 bytes) → RocksDB prefix-extractor + bloom filter behave well.
- Enables per-account scans at load time and for ops/inspection without a secondary index.

RocksDB tuning:
- Prefix extractor length = 37 bytes (`[tag:1][exchange_id:4][account:32]`) — common prefix of all positions for one (exchange, account).
- Prefix bloom filter over that 37-byte prefix.
- Fixed-width keys enable `memtable_prefix_bloom_size_ratio` tuning.

Lookup semantics:
- Point read at version `V`: seek to `(encoded_key, !V)`, take the first row — newest write with `version <= V`. Same bit-inverted-version mechanic as `state_value_by_key_hash`, but the row key is the encoded Position bytes (69 B), not a hash (32 B); this is the deliberate trade for prefix scans.
- Prefix scan at version `V`: iterate the prefix, dedupe by `encoded_key` taking the first row per key.

**Prefix scans enabled (off-hot-path only):**

```
All positions for (exchange_id, account) at version V:
  seek prefix = [tag=Position][exchange_id][account]        (37 bytes)
  iterate until prefix no longer matches
  for each distinct encoded_key, take first row (newest <= V)

All positions for exchange_id (admin sweep):
  seek prefix = [tag=Position][exchange_id]                  (5 bytes)
```

Prefix scans are NOT used during block execution. Execution-time reads come from the in-memory Position map (see Read / Write Path). Prefix scans exist for:
- Cold-start bootstrap: the startup loader uses a full scan (not prefix) to populate the in-memory store; a per-(exchange, account) prefix scan is useful if a partial reload is ever needed.
- Off-chain indexer / admin tooling.
- Disaster-recovery validation.
- Inspection RPCs.

Within Block-STM execution, prefix-scanning for all of a user's positions is handled by reading the virtual `UserMarkets[account]` (served from the in-memory derived index) and doing N point reads into the Position map — each tracked by Block-STM per-entry. Do NOT prefix-scan from inside a native during execution: it would bypass MVHashMap versioning and break parallel-execution correctness.

#### Stale-value index (`stale_position_value_index`)

Drives the value pruner:

```
Key:   (stale_since_version: u64 BE, version: u64 BE, encoded_state_key: Vec<u8>)
Value: ()
```

- When a new write supersedes an older row, an index entry is emitted marking when the older row becomes stale.
- Pruner seeks by `stale_since_version <= pruning_horizon` and deletes both the index row and the corresponding `position_value` row.

#### Merkle-side versioning (`position_merkle_db`)

Reuses the existing JMT schemas (`jellyfish_merkle_node`, `stale_node_index`, `stale_node_index_cross_epoch`) in separate CFs inside the new DB:

```
jellyfish_merkle_node          Key: NodeKey(Version, NibblePath)   Value: Node
stale_node_index               Key: (stale_since_version, NodeKey)  Value: ()
```

- Block commit adds new JMT nodes for modified keys and marks superseded nodes stale.
- Pruner deletes stale nodes whose `stale_since_version <= pruning_horizon`.
- Subtree root at version `V` is obtainable by reading the root node at version `V` (JMT versions every node).

#### Read flow at a given version

```
Latest point read (hot path — Position and UserMarkets both):
  MVHashMap → per-block in-memory cache → in-memory Position map / UserMarkets index
  (never touches RocksDB during block execution — the in-memory stores are the source)

Versioned point read (state proofs, historical queries — Position only):
  1. Value:  seek position_value at (encoded_key, !V), take first row
  2. Proof:  JMT inclusion proof at version V from position_merkle_db
             (JMT leaf key = SHA3-256(encoded_key), as with main state)
  3. Return (value, proof, position_root@V, main_state_root@V)
  UserMarkets has no versioned read — not persisted.

Prefix read (cold-start / admin only — outside Block-STM):
  seek position_value at (prefix_bytes, 0)
  iterate rows; dedupe by encoded_key, emitting newest <= V per key
```

The JMT hashes the encoded Position StateKey to get leaf keys — the hash lives inside `position_merkle_db` only, not in `position_value`. Value CF and merkle CF use different keyings on purpose: value CF optimizes for prefix scans + fixed-width keys; merkle CF needs uniform distribution for tree balance.

#### Write flow at commit

For each Position key written in a block, ordered by TX version:
1. Write a row into `position_value` at the TX version (`Some(StateValue)` or `None`).
2. Emit a `stale_since_version` entry marking the previous latest row (if any) as stale since this TX.
3. Update the JMT in `position_merkle_db`: new leaf node at the TX version; mark the old leaf + traversed internal nodes as stale since this TX.
4. Update the in-memory Position map.
5. New `position_root` at the block's terminal version is read from the JMT root node at that version and composed into the block's `state_root_hash`.

For each UserMarkets key written in the same block:
1. Apply the write to the in-memory derived index only (add market to set, remove market from set, or remove empty entry).
2. No DB row, no stale index, no JMT update, no contribution to `state_root_hash`.

#### Pruning

Independent policy per DB:
- Value pruner runs over `stale_position_value_index`.
- Merkle pruner runs over `stale_node_index` in `position_merkle_db`.
- Pruning horizon can differ from main state (tighter, since positions churn much faster and historical queries are less common).
- Epoch-aligned pruning is preserved: a `stale_node_index_cross_epoch` CF keeps nodes that are stale mid-epoch but required for cross-epoch proofs until epoch end.

#### Latest-value cache

Not needed as a separate layer: the in-memory Position map already holds the latest value for every key. RocksDB is only touched at startup (load) and at commit (persist). The per-TX cache (described in Read/Write Path) sits in front of the in-memory map to absorb repeated reads of the same Position within a TX.

#### What's NOT versioned

- `exchange_id` allocation registry in `position_db`'s metadata CF is singleton state (no versioning needed — register is monotonic, capabilities are stored as Move resources which carry their own versioning via main state).
- UserMarkets: derived, rebuilt at startup.

### Tombstones / deletion

Position deletion produces a `WriteOp::Deletion` at the change-set layer. Down-stream effects:
- `position_db`: `position_value` writes a tombstone row (`None`) at the delete version.
- `position_merkle_db`: JMT deletes the leaf at that version.
- In-memory Position map: remove entry.
- Storage-slot deposit refunded via `StateValueMetadata` (same mechanic as main state).

UserMarkets deletion (market removed from user's set, or entire user's set emptied) applies only to the in-memory derived index — no tombstone needed since there's no row to tombstone.

No application-level tombstone byte is needed in the value payload — the StateValue-level deletion is sufficient.

### Forward-compat rules

- New `StateKeyInner` variants must be appended to the enum (BCS variant index is positional).
- **BCS variant index vs `StateKeyTag` byte are independent enums and diverge.** BCS indices follow declaration order: `AccessPath=0, TableItem=1, Raw=2, Position=3, UserMarkets=4`. Physical `StateKeyTag` bytes follow assignment: `AccessPath=0, TableItem=1, Raw=255, Position=2`, UserMarkets has none. The two numbering schemes are not meant to match and their values must be treated independently.
- New `StateKeyTag` byte values are immutable once assigned. `Position = 2`; existing `Raw = 255` stays put. `UserMarkets` has NO `StateKeyTag` assignment — it is virtual, never encoded to bytes, never physically stored. MVHashMap keys it by the struct directly via `Hash`/`Eq` derives. `StateKeyInner::encode()` returns `Err("cannot encode virtual StateKey variant UserMarkets")` on a UserMarkets variant — it is a programmer error to reach that path (the change-set router's assertion is the primary invariant), but the error return matches the existing `anyhow::Result` signature.
- New position-payload variant tags are additive (e.g., `PerpV2 = 0x02`). An older binary reading a newer payload MUST fail loudly — gate all variant decoding on the feature flag / gas-schedule version.

### Example bytes

Position key for `exchange_id=7`, `account=0xaa..aa`, `market=0xbb..bb`:
```
02 00000007 aa..aa bb..bb      (69 bytes)
```

PerpV1 payload, `size=1000`, `is_long=true`, `entry_px_times_size_sum=50_000_000_000`, `avg_entry_px=50_000_000`, `leverage=10`, `is_isolated=false`, `funding_index=0`, `unrealized_funding_before=0`, `timestamp=1700000000`:
```
00
e803000000000000                          (size)
01                                        (is_long)
00743ba40b000000 00000000 00000000        (entry_px_times_size_sum, 16B)
80f0fa0200000000                          (avg_entry_px)
0a                                        (leverage)
00                                        (is_isolated)
00000000000000000000000000000000          (funding_index, 16B)
0000000000000000                          (unrealized_funding_before)
0080cf6500000000                          (timestamp)
total: 68 bytes
```

## Access Control

```move
module aptos_experimental::native_position {

    /// No copy, no drop. Only created via register(); destroyed via unregister().
    struct ExchangeCapability has store {
        exchange_addr: address,
        exchange_id: u32,
    }

    /// Register an exchange. Idempotent per signer: calling twice with the same
    /// signer returns a cap with the same exchange_id (the native layer remembers
    /// the allocation). `initial_max` sets the per-exchange position-count ceiling
    /// at first registration; subsequent calls with a different max are ignored
    /// (use the governance ceiling-update native to change it later).
    public fun register(exchange: &signer, initial_max: u64): ExchangeCapability {
        let addr = signer::address_of(exchange);
        let exchange_id = native_register(addr, initial_max);  // idempotent
        ExchangeCapability { exchange_addr: addr, exchange_id }
    }

    /// Governance-only: update the per-exchange ceiling. Used for scaling up
    /// (e.g., sizing for migration) or down (e.g., squeezing a misbehaving tenant).
    public(friend) fun update_ceiling(exchange_id: u32, new_max: u64) {
        native_update_ceiling(exchange_id, new_max);
    }

    /// Destroy a capability. Framework-side the exchange_id remains allocated
    /// (so re-registering returns the same id), but native functions will reject
    /// any call that carries a cap with an id marked disabled — see deny().
    public fun unregister(cap: ExchangeCapability) {
        let ExchangeCapability { exchange_addr: _, exchange_id: _ } = cap;
    }

    /// Framework-side denial: governance can disable an exchange_id by calling
    /// this native (gated on aptos_governance signer). All future native calls
    /// carrying a cap for that exchange_id abort. Existing persisted Positions
    /// are untouched — disable is a lockout, not a wipe.
    public(friend) fun deny(exchange_id: u32) {
        native_deny(exchange_id);
    }

    // All public functions require &ExchangeCapability.
    // Native functions are module-private.
}
```

**Revocation model.**

- **Idempotent registration.** `register()` is deterministic per signer. A compromised exchange cannot grief a legitimate exchange by pre-registering its address, and a legitimate exchange that calls register twice doesn't orphan an exchange_id.
- **Capability is a permission, not a unique authority.** Because `register` is idempotent, two calls return two caps with the same `exchange_id`. Both are valid; anyone holding any cap for `exchange_id=N` can call natives on that namespace. The exchange is responsible for custody of its caps. A leaked cap is functionally equivalent to a leaked access credential. Callers wanting single-authority semantics should wrap the cap in a resource that enforces their own access rules (e.g., held behind a signer check).
- **Self-destroy via `unregister`.** Adds `drop`-like behavior explicitly. Used by the exchange itself during orderly shutdown / migration or to retire a leaked cap copy. The `exchange_id` stays allocated so a later `register` call from the same signer gets the same id back. Does NOT affect other caps for the same id — unregistering one cap of many leaves the rest valid.
- **Governance lockout via `deny`.** For the compromise case: `aptos_governance` can mark an `exchange_id` disabled in the metadata CF. Native functions holding a cap for that id abort, regardless of how many caps exist. Positions stay in place (not wiped) — recovery/winddown is an app-level decision. `deny` can be paired with a governance-issued `reenable` if the compromise is resolved.
- **Why not just `drop`?** Without `deny`, a compromised exchange that still holds a valid cap can keep operating until its positions are manually unwound. `deny` is the kill-switch and works even if all other cap copies are still outstanding.
- **Why not revoke on-chain permissionlessly?** Because "compromise" is not on-chain-observable. Governance is the right layer.

**Registry locations and atomicity.**

`register()` writes to two different DBs in a single atomic commit:
- `position_db`'s metadata CF: `exchange_id_allocations[addr] = new_id`, `next_exchange_id` bump.
- Main state (`state_kv_db` + `state_merkle_db`): `PositionCounters.counts[new_id] = AggregatorV2::new(max)`.

Both writes land atomically via the 4-DB commit coordination described in Change-Set Routing. A partial-commit crash would leave the registry inconsistent ("allocated in metadata but no aggregator" or vice versa) and `create_position` on that exchange would fail; restart replays the block from `ledger_db` and completes the commit.

**`exchange_id` registry is NOT authenticated.**

The metadata CF in `position_db` is not versioned and not in any JMT. Clients cannot prove "exchange_id 7 is allocated to address X at version V" via a state proof. Today this is fine — the authenticated boundary is the `ExchangeCapability` resource in main state, which *is* authenticated (it's a normal Move resource). Anyone holding a cap has proof-equivalent access. But if a future use case emerges that needs to verify allocation independently of holding the cap, a companion authenticated allocation record in main state would be needed.

## Data Model (Rust)

```rust
enum NativePosition {
    PerpV1 {
        size: u64,
        is_long: bool,
        entry_px_times_size_sum: u128,
        avg_entry_px: u64,
        user_leverage: u8,
        is_isolated: bool,
        funding_index: i128,
        unrealized_funding_before: i64,
        timestamp: u64,
    },
    SpotV1 {
        size: u64,
        is_long: bool,
        entry_px_times_size_sum: u128,
        avg_entry_px: u64,
        timestamp: u64,
    },
}

// UserMarkets is stored as a bare HashSet<AccountAddress> throughout
// (in-memory DashMap, per-TX cache, resolver return type). No wrapper
// struct — the set is the data. Derived from Position entries; lives
// only in the in-memory derived index.
```

```move
enum Position has copy, drop, store {
    PerpV1 {
        size: u64, is_long: bool, entry_px_times_size_sum: u128,
        avg_entry_px: u64, user_leverage: u8, is_isolated: bool,
        funding_index: i128, unrealized_funding_before: i64, timestamp: u64,
    },
    SpotV1 {
        size: u64, is_long: bool, entry_px_times_size_sum: u128,
        avg_entry_px: u64, timestamp: u64,
    },
}
```

## Read/Write Path

### NativePositionResolver

New resolver trait modeled after `TableResolver` (which lives in `third_party/move/extensions/move-table-extension/src/lib.rs`). The new trait is defined in `aptos-move/aptos-vm-types/src/resolver.rs` since it's aptos-specific and integrates with aptos's `ExecutorView` stack:

```rust
pub trait NativePositionResolver {
    fn read_position(
        &self, exchange_id: u32, account: AccountAddress, market: AccountAddress,
    ) -> PartialVMResult<Option<Bytes>>;

    fn read_user_markets(
        &self, exchange_id: u32, account: AccountAddress,
    ) -> PartialVMResult<HashSet<AccountAddress>>;
}
```

Implemented by `StorageAdapter` → `LatestView` → MVHashMap. Both methods:
- Register Block-STM dependencies on the corresponding StateKey ✅
- Read from the in-memory stores (never RocksDB during execution) ✅
- Return raw bytes / direct set — no BCS deserialization, no layout computation ✅

### NativePositionContext (Session Extension)

```rust
#[derive(Tid)]
pub struct NativePositionContext<'a> {
    resolver: &'a dyn NativePositionResolver,
}
```

Registered as a session extension via `make_aptos_extensions()` in `aptos-move/aptos-vm/src/move_vm_ext/session/mod.rs`, following the `NativeTableContext` / `NativeAggregatorContext` pattern. Native functions access it via `context.extensions().get::<NativePositionContext>()`.

### Per-TX Cache

```rust
struct PositionTxCache {
    positions: HashMap<PositionKey, Option<NativePosition>>,
    // PositionKey → position. None = known to not exist.
    user_markets: HashMap<UserMarketsKey, HashSet<AccountAddress>>,
    // UserMarketsKey → markets set. Absent = not yet read.
    dirty_positions: HashSet<PositionKey>,
    dirty_user_markets: HashSet<UserMarketsKey>,
}
```

Uses the same `PositionKey` / `UserMarketsKey` structs as `PositionInMemory` so there's one key type per entity across the stack. Keys include `exchange_id` so a TX that composes across exchanges (rare but possible) doesn't collide on shared `(account, market)` pairs.

### Read Flow

```
Position read (first in TX):
  1. Check per-TX cache → miss
  2. Resolver.read_position(exchange_id, account, market)
     → MVHashMap lookup (if in-block write) → in-memory Position map → raw bytes
     → Block-STM read dep registered on StateKey::Position
  3. Deserialize 68 bytes → NativePosition                    ~100ns
  4. Store in per-TX cache
  Total: ~100-500ns (no DB access)

Position read (cached):
  1. Check per-TX cache → hit                                 ~0.1μs

Cross-margin (all positions for account):
  1. Resolver.read_user_markets(exchange_id, A) → {X, Y, Z}   ~0.1μs (in-memory)
     → Block-STM read dep registered on StateKey::UserMarkets
  2. Read Position[A][X], [A][Y], [A][Z]                      ~0.1-0.5μs each
     → Block-STM read dep registered on each StateKey::Position
  3. Compute status in Rust                                    ~0.5μs
  Total: ~1-2μs for 3 positions
```

### Write Flow

```
Position modify:
  1. Update Position in per-TX cache, mark dirty              ~0.1μs
  UserMarkets NOT touched. No conflict with other markets or with cross-margin.

Position create (new market for user):
  1. Write Position[A][X] in cache                            ~0.1μs
  2. Read UserMarkets[A] from cache/resolver, add X, mark dirty  ~0.1μs
  Both marked dirty.

Position remove:
  1. Write Position[A][X] = None in cache (tombstone)         ~0.1μs
  2. Read UserMarkets[A], remove X, mark dirty                ~0.1μs
  If markets set becomes empty, may also drop the UserMarkets entry.

TX finalize:
  For each dirty Position:
    1. Serialize to compact binary                            ~0.1μs
    2. Construct StateKey::Position { exchange_id, account, market }
    3. Inject as WriteOp into change set                      → Block-STM write dep

  For each dirty UserMarkets:
    1. Build the updated HashSet (or delete marker)
    2. Construct StateKey::UserMarkets { exchange_id, account }
    3. Inject as WriteOp into change set                      → Block-STM write dep
       (commit applier will route this to the in-memory index only)
```

## Block-STM Conflict Behavior

No overlay chain. No version handles. Pure MVHashMap per-entry tracking over Position and UserMarkets keys.

| Scenario | Conflict? | Why |
|----------|-----------|-----|
| Modify position X + Modify position Y (same user) | **No** | Different Position keys; UserMarkets not touched |
| Modify position X + Read position Y (same user) | **No** | Different Position keys |
| Create position + Modify existing (same user) | **Yes** | Both touch UserMarkets[user] |
| Create position + Read cross-margin (same user) | **Yes** | Cross-margin reads UserMarkets[user] |
| Any operation on user A + Any operation on user B | **No** | Different users |

Strictly better than today where ALL operations on the same user conflict (shared BigOrderedMap root).

**Hotspot to flag**: the backstop liquidator account receives inventory from every backstop liquidation and loses positions on every ADL. Each of those ops either adds or removes a market in `UserMarkets[backstop_liquidator]`, so all backstop/ADL ops in a block serialize on that single virtual key. This is not worse than today (which also serializes on the backstop's BigOrderedMap root), but it's an exception to the "different accounts never conflict" rule — worth knowing when reasoning about parallelism of liquidation-heavy blocks. Because UserMarkets is in-memory only, the cost is MVHashMap contention rather than RocksDB write amplification. If it still becomes a bottleneck, mitigations: (a) split backstop inventory across multiple sub-accounts, or (b) replace UserMarkets-per-user with an aggregator-backed variant for this specific account.

## Move API

### User Queries

User existence is derived (a user "exists" iff they have at least one position). No explicit create_user.

```move
public fun user_exists(cap: &ExchangeCapability, account: address): bool { ... }
// Equivalent to `|UserMarkets[account]| > 0`.
public fun get_user_markets(cap: &ExchangeCapability, account: address): vector<address> { ... }
// Reads UserMarkets[account] (virtual, in-memory).
```

### Position CRUD

```move
public fun get_position(cap: &ExchangeCapability, account: address, market: address): Option<Position> { ... }
public fun get_position_info(cap: &ExchangeCapability, account: address, market: address): (bool, u64, bool, u8) { ... }
// returns (exists, size, is_long, leverage)
public fun has_position(cap: &ExchangeCapability, account: address, market: address): bool { ... }
public fun has_any_position(cap: &ExchangeCapability, account: address): bool { ... }

public fun create_position(cap: &ExchangeCapability, account: address, market: address, position: Position) { ... }
// Writes StateKey::Position; also emits a UserMarkets write to add `market` to the set.
public fun update_position(cap: &ExchangeCapability, account: address, market: address, position: Position) { ... }
// Writes StateKey::Position only. UserMarkets NOT touched.
public fun remove_position(cap: &ExchangeCapability, account: address, market: address) { ... }
// Writes StateKey::Position deletion; also emits a UserMarkets write to remove `market` from the set.
```

### Computation Natives

```move
/// PnL for one position
public fun compute_pnl(cap: &ExchangeCapability, account: address, market: address,
    mark_px: u64, size_multiplier: u64): i64 { ... }

/// Funding cost for one position
public fun compute_funding_cost(cap: &ExchangeCapability, account: address, market: address,
    current_funding_index: i128): (i64, i128) { ... }

/// Margin required for one position
public fun compute_margin_required(cap: &ExchangeCapability, account: address, market: address,
    mark_px: u64, size_multiplier: u64): u64 { ... }

/// Apply a trade: update position, compute realized PnL + funding.
/// Handles increase, decrease, flip. Updates position in-place.
public fun apply_trade(cap: &ExchangeCapability, account: address, market: address,
    trade_size: u64, trade_price: u64, trade_is_long: bool,
    current_funding_index: i128, current_timestamp: u64,
): (i64, i64) { ... }
// Returns (realized_pnl, realized_funding_cost)

/// Iterate all positions for an account. Returns parallel vectors of (market, position).
/// `include_isolated`: when false, excludes isolated positions — matches
/// `positions_to_liquidate`'s cross-only path in backstop liquidation.
public fun get_account_positions(
    cap: &ExchangeCapability, account: address, include_isolated: bool,
): (vector<address>, vector<Position>) { ... }

/// Cross-margin status: reads all positions for account (via UserMarkets set),
/// computes aggregate PnL/margin using provided market states.
public fun compute_cross_margin_status(
    cap: &ExchangeCapability, account: address,
    excluded_market: Option<address>,
    // Market state per market (parallel vectors, provided by Move caller)
    market_addrs: vector<address>,
    short_mark_pxs: vector<u64>,
    long_mark_pxs: vector<u64>,
    funding_indices: vector<i128>,
    size_multipliers: vector<u64>,
    haircut_bps: vector<u64>,
    max_leverages: vector<u8>,
): (i64, i64, u64, u64, u64) { ... }
// Returns (unrealized_pnl, haircutted_upnl, margin_max_lev, margin_free_col, total_notional)
```

## Cold Start

Exchange-store data is fully resident in memory during execution. RocksDB is durable backing storage only. Cold start is the process of populating the in-memory stores at node open so block execution can begin.

### In-memory layout

```rust
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct PositionKey {
    pub exchange_id: u32,
    pub account: AccountAddress,
    pub market: AccountAddress,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct UserMarketsKey {
    pub exchange_id: u32,
    pub account: AccountAddress,
}

pub struct PositionInMemory {
    // Persisted source of truth (loaded from position_db at startup,
    // maintained at block commit).
    positions: DashMap<PositionKey, StateValue>,

    // Derived (built at startup by walking `positions`, maintained at block commit).
    // Never persisted, never in JMT.
    user_markets: DashMap<UserMarketsKey, HashSet<AccountAddress>>,

    // Metadata (also durable in the metadata CF, loaded once at startup).
    exchange_id_allocations: DashMap<AccountAddress, u32>,
    next_exchange_id: AtomicU32,
}
```

All reads during block execution resolve through these maps. RocksDB is not touched.

### Load sequence on node open

```
1. Open position_db and position_merkle_db.
2. Load metadata CF: exchange_id_allocations + next_exchange_id.
3. Spawn N worker threads (≈ #cores).
4. Partition position_value by the fixed 37-byte per-account prefix;
   each worker gets a contiguous RocksDB range.
5. Each worker iterates its partition, picking up the latest non-tombstone row
   per encoded key, and inserts into `positions`.
6. Barrier: all workers finish populating `positions`.
7. Second pass (cheap): walk `positions` and build `user_markets` —
   for each (exchange_id, account, market), insert market into the
   corresponding HashSet.
8. Validate: count(positions) matches expected counters in metadata CF.
9. Mark the store READY. Only then accept execution requests.
```

Byte-keyed `position_value` (not hash-keyed) makes step 4 trivially partitionable: each worker reads a contiguous SST range with good locality.

### Sizing and load time

- Position row: 69 B key + 68 B value payload + overhead ≈ 200 B on disk, ≈ 120 B in the DashMap.
- 1M positions: ~0.1 GB on disk, ~0.15 GB resident. Load ~1-2 s.
- 10M positions: ~1 GB on disk, ~1.5 GB resident. Load ~10-20 s.
- 100M positions: ~10 GB on disk, ~15 GB resident. Load ~60-120 s.

The load is a one-time cost at process start. During the load window the node cannot produce or execute blocks.

### Scaling thresholds and overflow policy

The in-memory design is sized for a perp DEX workload, not a global general-purpose store. Limits are **per-exchange** so one tenant cannot DoS another out of the global quota:

- **Per-exchange design target:** up to **50M Positions per `exchange_id`**. At one active exchange, this corresponds to ~7.5 GB resident and ~30-60 s load time — operationally acceptable for validator restarts.
- **Per-exchange hard ceiling:** also 50M, enforced atomically via `AggregatorV2`. Storage layout:
  ```move
  struct PositionCounters has key {
      counts: Table<u32, AggregatorV2<u64>>,  // exchange_id → counter
  }
  ```
  One module-owned resource at `@aptos_experimental::position_counts`, holding a `Table` keyed by `exchange_id`. `register()` allocates a new table entry with `max = per_exchange_ceiling`. `create_position` calls `counts[exchange_id].try_add(1)` — the aggregator returns false if it would cross `max`, and the native aborts `E_EXCHANGE_POSITION_LIMIT`. `remove_position` calls `counts[exchange_id].sub(1)`. Under Block-STM, delayed-field semantics on `AggregatorV2` carry through the `Table` access — concurrent `try_add` / `sub` on the same exchange's counter don't conflict as long as the bound isn't hit. Different exchanges have different `Table` entries and never conflict.
  The counter lives in main state and is therefore authenticated through `main_state_root` (clients can verify the ceiling).
- **Per-exchange ceiling is governance-tunable.** Initial limit is 50M; governance can lower (to squeeze a misbehaving tenant) or raise (if validators are sized for more) by updating the aggregator's `max`. Requires a framework-internal governance native to mutate the bound — cheap to add.
- **Chain-wide soft warning:** if aggregate Positions across all exchanges cross **~100M**, revisit. At ~15 GB resident per node and ~60-120 s load time, we're at the edge of what's operationally comfortable. Mitigations: incremental-load-during-state-sync (removes the post-sync stall), memory-mapped snapshot files (faster restart), or sharding `exchange_id` ranges across validator subsets.
- **Beyond 100M aggregate:** the in-memory design needs to be revisited. Options: disk-backed reads with LRU caching, shard `exchange_id` ranges across validator subsets, or build a SetAggregator-backed derived index that doesn't need full residency.
- **Consistency check at startup.** On node open, compare the per-exchange aggregator value against the in-memory `positions` DashMap size for that exchange. Mismatch → abort startup with a diagnostic; indicates a commit-path bug (Position was written but aggregator wasn't incremented, or vice versa). This check is cheap (O(number of exchanges)) and catches divergence early.

### Startup latency caveats

- 60-120 s validator restart is acceptable for unscheduled crashes but noticeable for rolling upgrades. Mitigation: incremental load during state sync (deferred optimization — described in scenarios) brings the post-sync stall to near-zero.
- Consensus participation blocks until the store is marked READY. Validators rejoining after an upgrade window will miss the blocks committed during their load time. This is the same failure mode as any large-state restart today; no new consensus risk introduced.
- Fullnodes serving RPC can return "not ready" (HTTP 503) during the load window. Clients retry.

### Other cold-start scenarios

- **Fresh node via state sync.** State sync delivers two independent chunk streams: main state (verified against `main_state_root`) and Position state (verified against `position_root`). BOTH streams must complete before the startup load begins — the load partitions `position_value` by `[exchange_id][account]` prefix, so any missing range would produce a corrupt in-memory map. After both streams complete, run the startup load to populate memory, then begin execution. UserMarkets is NOT shipped by state sync; it's rebuilt locally from the applied Positions.
- **State-sync catch-up (fallen-behind node).** Same as above, or via transaction replay — writes land in both RocksDB and the in-memory stores during replay, so no extra load phase is needed after replay completes.
- **First block after feature-flag activation.** Zero Position entries exist; both in-memory stores are empty. First `register()` allocates `exchange_id=1` in the metadata CF. Positions materialize as users call `create_position`.
- **Migration from existing `UserPositions` (etna).** Orthogonal to cold start — see Migration section. TL;DR: dual-read + background sweep + gated cutover.

### Who needs the in-memory store?

All nodes that serve reads or execute blocks maintain the full in-memory store.

- **Validators:** yes. They execute blocks and need sub-microsecond Position reads in the hot path. Full in-memory residency.
- **VFNs (validator fullnodes) / execution-replay fullnodes:** yes. They re-execute blocks for state verification and must match validator performance envelope. Full residency.
- **Archive / query fullnodes (RPC-only, no re-execution):** yes. They serve RPC queries including `get_user_markets`, and UserMarkets is not in RocksDB. Serving these queries via prefix-scan fallback on every RPC call is too slow and coupling to disk I/O. All fullnodes perform the same startup load and maintain the same in-memory Position + UserMarkets maps. Memory cost is the same as validators: ~1.5 GB at 10M positions, ~15 GB at 100M.
- **Indexer fullnodes:** still need the in-memory store if they serve RPC; do NOT need it if they purely follow the WriteSet stream into their own DB and never answer UserMarkets queries. Indexer operators pick based on their query surface.

Fleet memory implication: every RPC-serving node pays the same in-memory cost. Illustrative fleet (numbers are placeholders — actual sizes TBD per operator): 100 validators + 20 VFNs + 200 archive fullnodes → ~320 nodes × in-memory footprint. This is a real operational cost and a design input for the scaling thresholds above.

No `in_memory_position` config flag exists — mode is inferred from node role and is always "in-memory" for any node that answers Position or UserMarkets RPC queries.

### Commit-time update order

```
Per block commit:
  1. Write to RocksDB (atomic with ledger commit):
       - Position WriteOps → position_value + stale index + JMT in position_merkle_db
       - UserMarkets WriteOps → nothing (in-memory only)
  2. Apply to in-memory stores:
       - Position WriteOps → positions DashMap
       - UserMarkets WriteOps → user_markets DashMap
```

RocksDB is canonical; in-memory is derived state. Crash between (1) and (2) is safe — restart rebuilds in-memory from RocksDB.

## Gas Schedule

Dedicated schedule, separate from `table`, `move_stdlib`, etc. Defined in a new file `aptos-move/aptos-gas-schedule/src/gas_schedule/position.rs`, wired into `NativeGasParameters` as `.position`. All params feature-flagged with `N..` version gating so older binaries don't see them.

**Framework constants:**
```
PositionGasParameters, "position", NativeGasParameters => .position
```

**Base I/O:**
All reads during block execution are in-memory (never RocksDB). Gas reflects in-memory cost + serialization, not disk I/O.
- `load.base` — per read (in-memory point lookup + Block-STM dep registration)
- `load.per_byte` — per byte deserialized from raw payload into `NativePosition`
- `load.failure` — not-found fast path (no deserialization)
- `write.base` — per dirty entry at session finalize
- `write.per_byte` — per byte serialized
- `delete.base` — for removals (tombstone write-op)

**Per-native:**
- `register.base` — exchange registration (idempotent; same cost either call)
- `unregister.base` — capability destroy (cheap — just frees the cap; exchange_id stays allocated)
- `deny.base` — governance lockout (governance-gated, charged to the governance proposal)
- `user_exists.base`, `has_position.base`, `has_any_position.base`
- `get_position.base`, `get_position.per_byte`
- `get_position_info.base` (cheaper than `get_position` since no full enum unpack)
- `get_user_markets.base`, `get_user_markets.per_market` (output scales with markets — in-memory read is cheap but gas must still scale)
- `get_account_positions.base`, `get_account_positions.per_position` (linear in N positions read)
- `create_position.base`, `create_position.per_byte` (Position write + UserMarkets in-memory update)
- `update_position.base`, `update_position.per_byte` (Position write only)
- `remove_position.base`, `remove_position.per_byte` (Position delete + UserMarkets in-memory update)
- `compute_pnl.base`
- `compute_funding_cost.base`
- `compute_margin_required.base`
- `apply_trade.base` (fixed-cost arithmetic, no iteration)
- `compute_cross_margin_status.base`, `compute_cross_margin_status.per_position` (linear in position count)

**Storage fees (apply to Position only — UserMarkets is in-memory, no storage deposit):**
- `storage_slot.base` — per-slot deposit when creating a new Position entry (mirrors existing per-slot deposit for resources). Refunded on delete via `StateValueMetadata`.
- `storage_byte.base` — per-byte storage fee (one-time, non-refundable) on Position write.
- Values may differ from `state_kv_db` rates since `position_db` has different tuning / retention; chosen by benchmark.

**Rollout note:** all params must be added with `{ N.. => "..." }` version gates so validators on gas-schedule version < N still parse the schedule. Native functions themselves gated by a feature flag (see Rollout section).

## PositionStatus

Not stored. Computed on demand by `compute_cross_margin_status`. With positions in per-TX cache, iterating 3-5 positions is ~0.5μs in Rust. No need for `CachedPositionStatuses` or aggregators for regular users. Backstop liquidator can also use compute-on-demand since native iteration is fast enough.

## Deferred

- **Market → accounts reverse index**: requires `SetAggregator` for concurrent append + snapshot enumeration within a block. Until then: DB-level prefix scan between blocks or off-chain keepers.
- **On-chain liquidation discovery**: depends on reverse index. When mark price updates, need to find all accounts on that market to check liquidation. Current approach: off-chain keepers submit liquidation TXs.
- **Collateral in native store**: collateral already uses Table + I64Aggregator (parallel-friendly). Can be added later if BCS deser overhead is a bottleneck.

## Migration

Etna is live today with `UserPositions: BigOrderedMap<Object<PerpMarket>, PerpPosition>` resources at every user address. The native position subsystem has no cutover magic — explicit migration is required. Plan: **dual-read + background sweep + gated cutover** (option D from the design review).

### Phases

**Phase 0 — framework lands, feature flag off.**
- `NATIVE_EXCHANGE_STORE` feature flag deployed but disabled.
- Natives and `position_db` exist in the binary; no TXs can call them yet.

**Phase 1 — flag on, etna unchanged.**
- Governance activates the feature flag.
- Etna still reads/writes `UserPositions`; no position state exists yet.
- Composite state root activates at this version (see Composite-Root Activation).

**Phase 2 — etna deploys dual-read module.**
Etna deploys a module version that:
- Calls `register(signer, initial_max = existing_position_count + growth_headroom)` as part of module init. The `initial_max` parameter on `register()` sets the per-exchange aggregator ceiling to fit etna's current position count plus headroom (e.g., current 45M + 25M headroom = 70M max). Governance can tune later via the ceiling-update native.
- The wrapper's single predicate is `exists<UserPositions>(account)`:
  - **Exists** → account is unmigrated (has legacy data).
  - **Absent** → account is on the native path (either already migrated, or fresh post-Phase-2).
  No separate `Migrated` marker is needed — destruction of `UserPositions` during `migrate_user` is itself the "migrated" signal, and fresh accounts (never having had `UserPositions`) are indistinguishable from migrated ones, which is exactly what we want.
- **Per-position reads (`get_position(A, M)`):** prefer native; if the account has `UserPositions`, fall back to the BigOrderedMap lookup. Correct either way.
- **Aggregate reads (`get_user_markets(A)`, `compute_cross_margin_status(A)`, `has_any_position(A)`):** *do NOT* fall through to native UserMarkets for unmigrated users — that returns empty. Instead, etna's wrapper branches on `exists<UserPositions>(account)`:
  - Not present → native path (`get_user_markets` on the native resolver).
  - Present → iterate `UserPositions[A]` keys (the BigOrderedMap provides them).
  This dual-path at the aggregate level is the load-bearing correctness property during Phase 2-3.
- **Writes:** if `!exists<UserPositions>(account)`, write to native. Otherwise abort with `E_MIGRATION_REQUIRED` — the user must first migrate in a dedicated TX. Writes **never** inline-migrate. Fresh accounts (no prior `UserPositions`) take the native path directly.
- **New accounts** created after Phase 2 go directly to native — they never allocate `UserPositions`, so their write path always succeeds under the predicate above.

Etna exposes a permissionless entry point `migrate_user(account)` that:
- No-op if `!exists<UserPositions>(account)`.
- Otherwise: reads **all** entries from `UserPositions[account]`, writes each to the native store, bumps the aggregator, emits a `UserMigrated { account, migrated_at_version, position_count }` event for observability, destroys `UserPositions[account]` — **all in a single TX, atomically**.

**Why single-TX is safe:** position count per user is bounded by the number of markets the user trades. Even power users are in the low hundreds, far below what a normal TX's gas budget can handle (rough upper bound: ~5M gas for 100 positions at ~50k gas each; well under the chain's per-TX limit). Position creation is gated by etna's `ExchangeCapability`, so adversarial inflation isn't possible. This assumption lets us skip any partial-migration state machinery.

**Why no inline migration:** the off-chain sweep (Phase 3) migrates users proactively, so most users never encounter `E_MIGRATION_REQUIRED`. The rare user who does (offline during the sweep, or signed up between sweep and their next TX) submits one extra TX. Removing the inline path keeps etna's wrapper simple (no position-count check, no two-path write logic, no gas-budget edge cases) and keeps trading TX gas predictable. Front-ends that want seamless UX can bundle `migrate_user` + trade into a multi-step TX client-side.

**Phase 3 — background sweep.**
- Off-chain tool iterates all accounts with `UserPositions` and calls `migrate_user` in batches.
- Progress tracked off-chain (state snapshots + counting `UserMigrated` events, or tracking accounts that still carry the `UserPositions` resource).
- Throughput tuned to avoid starving regular TX load.
- Duration is a function of user count and block-space headroom. Illustrative: 1M users × 1 TX per user ≈ 1M TXs; at realistic sustained rates this is hours. Measure on testnet with etna's actual user count before committing to a launch window; don't rely on the illustrative number.
- Accounts created between Phase 0 activation and Phase 1 end have `UserPositions` and are picked up by the same sweep — no special handling.

**Phase 4 — cutover with on-chain gate.**

Safety net, in this order:
1. Off-chain sweep confirms no `UserPositions` resources remain anywhere.
2. Governance sets an on-chain `MigrationComplete { finalized_at_version }` resource (writable only by governance). One global marker, not per-user.
3. Etna deploys the cutover module version. `init` asserts that `MigrationComplete` exists and aborts deployment otherwise.
4. The cutover module's per-user entry points include a runtime assertion: `assert!(!exists<UserPositions>(account), E_MIGRATION_REQUIRED)`. If the sweep missed an account, the TX that touches it aborts visibly rather than silently losing that account's old positions.
5. **`migrate_user` stays in the module permanently** as an escape hatch for any stuck account found post-cutover. Only the dual-read branches (per-position fallback, aggregate-read branching) are removed. This keeps stuck accounts recoverable without another governance cycle.
6. After a quiet period (e.g., 48h) with no `E_MIGRATION_REQUIRED` aborts in the wild, etna ships the dual-read-removed version. `UserPositions` and its associated constructors/lookups/mutations stay as dead code in etna's Move module (Move can't delete module types or friend-visible functions), and `migrate_user` stays callable forever as an escape hatch.

### Correctness properties

- **Migration is idempotent.** `migrate_user` on an account without `UserPositions` is a no-op. Safe to call multiple times from different batches.
- **No per-position read gap.** Per-position reads fall through to `UserPositions` when it exists.
- **No aggregate read gap.** Aggregate reads branch on `exists<UserPositions>(account)`; the unmigrated path iterates `UserPositions` keys directly, not the native UserMarkets (which is empty for that user). Without this branching, aggregate reads would silently return incomplete data for unmigrated users.
- **No double-spend.** `migrate_user` reads UserPositions, writes Positions, emits the event, destroys UserPositions — all in a single TX. Atomic by Move's execution model. A user is always either fully unmigrated (UserPositions exists, all positions there) or native-path (UserPositions absent, positions in native if any). No intermediate state is observable.
- **Cutover is gated.** `MigrationComplete` marker + runtime `UserPositions` assertion catch any account the off-chain sweep missed.
- **Fresh accounts are never blocked.** A new account never has `UserPositions`, so the write predicate lets it use the native path from day one.

### Validation before scale-up

Migration is forward-only — the plan does not include a rollback path. Once the Phase 3 sweep starts landing `migrate_user` TXs on real users, reversing is impractical at production scale. Mitigate by validating thoroughly before scale-up:

- Full end-to-end test of Phase 2-4 on testnet, including composite state root and proof verification by client SDKs.
- Canary migration: run the Phase 3 sweep on a small subset of real mainnet users (e.g., a few hundred, handpicked across size classes), let them trade for a few days, confirm no anomalies, then scale the sweep to the full user base.
- Post-cutover monitoring: track `E_MIGRATION_REQUIRED` aborts for the quiet period (Phase 4 step 6) before removing dual-read branches.

### Non-goals

- No automatic migration triggered by the framework. Etna (the application) is responsible for calling `migrate_user` and managing the cutover. The framework only provides the new storage.
- No framework-level coexistence shim for old `UserPositions` data. That lives in etna's Move code.
- **No rollback path.** Migration is forward-only; if a bug is discovered mid-migration, fix it forward rather than reversing.

## Key Design Decisions

1. **Per-position MVHashMap entries** (not overlay chain): positions are simple key-value, no complex data structure. Block-STM handles versioning automatically.
2. **UserMarkets is derived, in-memory only**: per-user market set is a virtual StateKey variant (`StateKeyInner::UserMarkets`), never persisted to DB, never in JMT. Rebuilt at node startup from the Position entries. Gives Block-STM the conflict-tracking key it needs without the DB write / JMT update / storage-deposit cost that explicit storage would incur.
3. **Raw byte format, not BCS**: compact fixed-size binary for positions (68 bytes PerpV1). No layout computation or type resolution on read.
4. **PositionStatus computed, not stored**: with positions resident in memory, iteration 3-5 positions is sub-microsecond. No pre-computation needed.
5. **Full in-memory residency**: Positions are fully loaded into RAM at node open. RocksDB is durable backing storage; execution never touches disk. Predictable sub-microsecond reads, no DB tail latency in the hot path.
6. **Dedicated RocksDB database `position_db`** (not a CF inside state_kv_db, not a hot/cold tier): single-tier, isolated tuning, fixed-width keys, tuned for small-value frequent updates. Byte-keyed (not hash-keyed) so startup load can partition by `[exchange_id][account]` prefix for parallel RocksDB range reads.
7. **Dedicated state merkle tree `position_merkle_db`**: Position keys are authenticated by their own JMT, not the main `state_merkle_db`. UserMarkets has no JMT leaf. Block state root becomes `H("APTOS::StateRoot" || main_state_root || position_root)`. Activated at the feature-flag version; legacy roots remain unchanged pre-activation.
8. **Dedicated gas schedule `position`**: separate from `table` and other native schedules so fee curves can be tuned independently of general state. UserMarkets reads/writes priced as in-memory ops (no storage deposit).
9. **ExchangeCapability access control**: `store` only (no copy, no drop). Only the registered exchange can access its positions.
10. **Market addresses as keys** (not short IDs): simpler, no mapping infrastructure. 32-byte keys are acceptable.

## Current Code State (as of 2026-04-20)

- `UserPositions`: `BigOrderedMap<Object<PerpMarket>, PerpPosition>` (B+tree in table storage)
- `PerpPosition` fields: size, entry_px_times_size_sum, avg_acquire_entry_px, user_leverage, is_long, is_isolated, funding_index_at_last_update (AccumulativeIndex), unrealized_funding_amount_before_last_update, timestamp
- `CachedPositionStatuses`: `BigOrderedMap<address, PositionStatusCache>` — backstop liquidator only
- `CollateralBalanceSheet`: Table-based with I64Aggregator for primary_balance — unchanged
- `clearinghouse_perp.move`: refactored, liquidation logic moved to `liquidation/liquidation.move`
- `dex_accounts_entry.move`: new entry point module wrapping dex_accounts

## Files to Modify

### aptos-core

**Storage layer — StateKey & registry:**
- `types/src/state_store/state_key/inner.rs` — Append `Position { exchange_id, account, market }` and `UserMarkets { exchange_id, account }` at the end of the enum (BCS wire compat). New `StateKeyTag` bytes.
- `types/src/state_store/state_key/registry.rs` — New sharded registry for `Position` entries. `UserMarkets` may need a simpler registry since it's Eq/Hash-keyed by the struct (no encoded-bytes interning).

**Storage layer — value DB (position_db):**
- `storage/aptosdb/src/position_db/` — New module (sibling of `state_kv_db`, `state_merkle_db`): `mod.rs`, `position_value` + `stale_position_value_index` + metadata CFs, schema, open/init/close, pruning, backup/restore hooks.
- `storage/aptosdb/src/db_options.rs` — New `position_db_column_families()` with prefix bloom filter on the 37-byte `[tag][exchange_id][account]` prefix.

**Storage layer — merkle DB (position_merkle_db):**
- `storage/aptosdb/src/position_merkle_db/` — New module paralleling `state_merkle_db`: `mod.rs`, JMT integration, stale-node-index tracking, batch commit. Only Position leaves.
- `storage/aptosdb/src/db_options.rs` — New `position_merkle_db_column_families()` for JMT node storage (reuse `JELLYFISH_MERKLE_NODE`, `STALE_NODE_INDEX` schemas).
- `storage/jellyfish-merkle/src/` — Parameterize or reuse the existing `JellyfishMerkleTree` on the new DB handle; no tree-logic changes expected.
- `types/src/proof/` — Extend state proofs with a composite form that carries both `main_state_root` and `position_root`, plus the inclusion path in whichever subtree the proved key lives.
- `types/src/transaction/` — `TransactionInfo` / state-root hashing helper updated to compose the two roots (gated by feature flag activation version).
- `crates/aptos-crypto/` (if needed) — Add domain-separated hash tag `APTOS::StateRoot` for the composition.

**In-memory store + commit-path routing:**
- `aptos-move/block-executor/src/` (or new crate) — `PositionInMemory` struct, startup loader, commit-path routing (Position → DB+JMT+in-memory; UserMarkets → in-memory only).
- `aptos-move/aptos-vm/src/move_vm_ext/session/mod.rs` — Session finalize splits Position and UserMarkets WriteOps into appropriate buckets.

**Storage layer — integration:**
- `storage/aptosdb/src/lib.rs` / `storage/aptosdb/src/db/mod.rs` — Wire both `position_db` and `position_merkle_db` alongside existing DBs at init time; pass handles down the read/write paths.
- `storage/storage-interface/src/` — Read-side interface additions so the VM layer can query `position_db` through the existing `DbReader`/`StateView` stack; proof-side interface for `position_merkle_db`. UserMarkets reads short-circuit to the in-memory store.
- `storage/backup/backup-cli/` — Backup + restore for both new DBs; state-snapshot stream partitioned into main + Position substreams. UserMarkets is rebuilt post-restore, not shipped.
- `storage/aptosdb/src/pruner/` — Pruner for both new DBs (independent epoch-based policies allowed).
- `state-sync/` — Two independent chunk streams (main state + Position state); client verifies each against its subtree root and composes for the signed `state_root_hash`.
- `storage/indexer/src/db_v2.rs`, `crates/aptos-rosetta/src/types/objects.rs`, `api/types/src/convert.rs`, `storage/aptosdb/src/ledger_counters/mod.rs` — Handle `StateKeyInner::Position` (and skip `StateKeyInner::UserMarkets` as a virtual variant that should never appear in persisted paths).

**Resolver layer:**
- `aptos-move/aptos-vm-types/src/resolver.rs` — Define `NativePositionResolver` trait
- `aptos-move/block-executor/src/view.rs` — Implement for `LatestView` (MVHashMap integration, captured_reads for Block-STM tracking)
- `aptos-move/aptos-vm/src/data_cache.rs` — Implement for `StorageAdapter`

**Native interface / session:**
- `aptos-move/aptos-native-interface/src/` — `NativePositionContext` extension type (Tid)
- `aptos-move/aptos-vm/src/move_vm_ext/session/mod.rs` — Register context in `make_aptos_extensions()`; finalize dirty writes into change set during session end

**Native functions crate:** (no `order-book-natives` crate exists today — either option works)
- Option A: create new crate `aptos-move/framework/position-natives/` — cleaner separation
- Option B: add to `aptos-move/framework/aptos-trading/` natives — co-locates with related trading natives
- Contents either way: `NativePosition`, per-TX cache (positions + user_markets), compact binary serialization, all native CRUD + computation functions

**Gas schedule:**
- `aptos-move/aptos-gas-schedule/src/gas_schedule/position.rs` — New `PositionGasParameters` (see Gas Schedule section). Wire into `NativeGasParameters` via `mod.rs`.
- `aptos-move/aptos-gas-schedule/src/gas_schedule/mod.rs` — Add `position` field to `NativeGasParameters`; expose version gates.

**Move module:**
- `aptos-move/framework/aptos-experimental/sources/native_position.move` — `ExchangeCapability`, `Position` enum, public API + native declarations, gated by a new `FeatureFlag::NATIVE_EXCHANGE_STORE`.
- `aptos-move/framework/aptos-experimental/sources/position_counts.move` — `PositionCounters { counts: Table<u32, AggregatorV2<u64>> }` resource at `@aptos_experimental::position_counts`, plus `update_ceiling(exchange_id, new_max)` governance native. Declares `friend aptos_experimental::native_position` so `register` / `create_position` / `remove_position` can call `allocate_counter(exchange_id, max)`, `try_add(exchange_id, 1)`, `sub(exchange_id, 1)` directly; `update_ceiling` is `public(friend)` restricted to the governance module.
- `aptos-move/framework/move-stdlib/sources/configs/features.move` — Add `NATIVE_EXCHANGE_STORE` feature flag constant.

**Feature-flag enforcement:**
- Every native function entry point (in the native functions crate above) must call `features::is_native_position_enabled(ctx)` first and abort `E_FEATURE_DISABLED` if off. This is a belt-and-suspenders check in case a TX reaches the native before the module's own flag assertion — prevents reads/writes against `position_db` during any window where the flag could be toggled off.

### etna (separate repo — verify paths in that tree, not here)

- `move/perp/sources/position_management/perp_positions.move` — Remove `UserPositions` (BigOrderedMap), use `native_position`
- `move/perp/sources/position_management/position_update.move` — Use native `apply_trade` + computations
- `move/perp/sources/collateral/accounts_collateral.move` — Use native `compute_cross_margin_status`
- `move/perp/sources/clearinghouse_perp.move` — Use natives at entry points (`settle_trade`, `validate_*`)
- `move/perp/sources/liquidation/liquidation.move` — Use natives for liquidation position reads
- `move/perp/sources/liquidation/backstop_liquidation.move` — `positions_to_liquidate` (cross iteration with isolated filter) → native; per-position reads of backstop liquidator → native
- `move/perp/sources/liquidation/adl.move` — Hot `get_position_size` / `get_position_is_long` loop → native (big win from per-TX cache); note `adl_tracker` stays as-is (separate per-market leverage-bucket structure, not replaced by native position subsystem)
- `move/accounts/sources/dex_accounts.move` — Create native user on subaccount creation
- `move/accounts/sources/dex_accounts_entry.move` — Entry points may need acquire/flush wrappers

## Expected Impact

Latencies below reflect the full-in-memory design: Position data is resident in RAM during execution, RocksDB is touched only at startup and commit.

| Operation | Before (Move + BigOrderedMap) | After (Native, in-memory) |
|-----------|-------------------------------|---------------------------|
| Position read (1st in TX) | ~30-60μs (B+tree traversal: multiple table reads + BCS deser per node) | ~100-500ns (MVHashMap → in-memory map + deserialize 68B) |
| Position read (cached in TX) | ~3-5μs (TxDataCache, still B+tree node lookup) | ~50-100ns (per-TX cache hit) |
| Position write | ~30-60μs (B+tree insert/update + BCS ser per node) | ~100-200ns (cache update) + 68B serialize at finalize |
| Cross-margin (3 positions) | ~150-200μs (B+tree iteration + BCS + Move interpreter) | ~1-2μs (UserMarkets + 3 Position reads + Rust compute) |
| apply_trade | ~100μs (Move interpreter + B+tree ops) | ~500ns-1μs (Rust) |
| Two TXs, same user, different markets | **Conflict** (shared BigOrderedMap root) | **No conflict** |
| Node restart | ~seconds (state_kv_db open) | +10-120s for Position load depending on scale (~10-20s at 10M positions, ~60-120s at 100M; see Sizing and load time) |
| Per-validator memory | baseline | +~1.5 GB at 10M positions; +~15 GB at 100M (see Sizing and load time) |

## Verification

1. Sequential benchmark, 200K TXs, concurrency=1
2. Compare VM-TX avg with baseline (2,159μs)
3. Zero aborts
4. Probe native operations for latency
5. Test parallel execution — verify correct conflict behavior, no false conflicts
