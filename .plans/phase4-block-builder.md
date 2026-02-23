# Phase 4: Block Builder + PrefixConsensusBlock Variant

## Goal

Add `BlockType::PrefixConsensusBlock` to the consensus-types crate and create a `build_block_from_v_high()` function in a `block_builder.rs` module in prefix-consensus. The `PipelinedBlock`/`OrderedBlocks` wrapping is deferred to Phase 5 (SlotManager) since `OrderedBlocks` lives in the `aptos-consensus` crate, not `consensus-types`.

## Context

### Where This Fits

When SPC outputs v_high for a slot, the SlotManager needs to:
1. Look up actual payloads from the payload_map (Phase 3)
2. **Build a `Block` containing the aggregated committed proposals** (this phase)
3. Wrap it in `PipelinedBlock` and `OrderedBlocks` (Phase 5 — SlotManager, in the `aptos-consensus` crate)
4. Send to the execution pipeline via `UnboundedSender<OrderedBlocks>`

This phase creates the `Block` construction machinery. The SlotManager (Phase 5) will call `build_block_from_v_high()` and handle the `PipelinedBlock`/`OrderedBlocks` wrapping itself.

### Why `build_ordered_blocks` Is Not in This Phase

`OrderedBlocks` is defined in `consensus/src/pipeline/buffer_manager.rs` (the `aptos-consensus` crate). The `block_builder.rs` module lives in `consensus/prefix-consensus/` (the `aptos-prefix-consensus` crate). Adding a dependency from `aptos-prefix-consensus` → `aptos-consensus` would create a heavyweight (and potentially circular) dependency. Instead:
- **Phase 4** (`aptos-prefix-consensus`): `build_block_from_v_high()` returns a `Block`
- **Phase 5** (`aptos-consensus`): SlotManager wraps the `Block` in `PipelinedBlock::new(block, vec![], StateComputeResult::new_dummy())`, constructs `OrderedBlocks` with `AggregateSignature::empty()`, and sends to the execution channel — following the DAG adapter pattern inline

This matches how DAG does it: the wrapping logic is inline in `adapter.rs`, not in a separate crate.

### DAG Pattern (What We Mirror)

DAG's `OrderedNotifierAdapter::send_ordered_nodes()` in `consensus/src/dag/adapter.rs:137-238`:

```rust
// 1. Aggregate payloads from all ordered nodes
let mut payload = Payload::empty(!anchor.payload().is_direct(), ...);
for node in &ordered_nodes {
    validator_txns.extend(node.validator_txns().clone());
    payload = payload.extend(node.payload().clone());
    node_digests.push(node.digest());
}

// 2. Build Block
let block = Arc::new(PipelinedBlock::new(
    Block::new_for_dag(epoch, round, block_timestamp, ...),
    vec![],
    StateComputeResult::new_dummy(),
));

// 3. Build ordering proof (placeholder — real BlockInfo computed after execution)
let block_info = block.block_info();  // uses StateComputeResult::new_dummy()
let blocks_to_send = OrderedBlocks {
    ordered_blocks: vec![block],
    ordered_proof: LedgerInfoWithSignatures::new(
        LedgerInfo::new(block_info, anchor.digest()),
        AggregateSignature::empty(),
    ),
};

// 4. Send to execution
self.executor_channel.unbounded_send(blocks_to_send)
```

Note: `block_info()` uses `StateComputeResult::new_dummy()` which produces `executed_state_id = ACCUMULATOR_PLACEHOLDER_HASH` and `version = 0`. This is a placeholder — the real `BlockInfo` is computed after execution. DAG does the same thing (`adapter.rs:200`).

### Existing BlockType Enum

From `consensus/consensus-types/src/block_data.rs:26-70`:

```rust
pub enum BlockType {
    Proposal { payload, author, failed_authors },
    NilBlock { failed_authors },
    Genesis,
    ProposalExt(ProposalExt),
    OptimisticProposal(OptBlockBody),
    #[serde(skip_deserializing)]
    DAGBlock { author, failed_authors, validator_txns, payload, node_digests, parent_block_id, parents_bitvec },
}
```

### Match Arms That Need Updating

**Exhaustive matches (compiler errors if not updated):**
- `validator_txns()` at `block_data.rs:178` — lists all 6 variants explicitly
- `failed_authors()` at `block_data.rs:225` — lists all 6 variants explicitly
- `validate_signature()` at `block.rs:426` — lists all 6 variants explicitly

**Wildcard matches (won't break but must update for correctness):**
- `author()` at `block_data.rs:137` — has `_ => None` fallback; add to `Proposal | DAGBlock` combined arm
- `payload()` at `block_data.rs:167` — has `_ => None` fallback; add to `Proposal | DAGBlock` combined arm
- `parent_id()` at `block_data.rs:156` — `if let DAGBlock` with else fallback to QC. **Critical**: without updating, fallback returns `self.quorum_cert.certified_block().id()` which is `BlockInfo::empty().id()` = `HashValue::zero()`, silently breaking block chain integrity

**Non-exhaustive checks elsewhere (verified as non-issues):**
A `grep -r "BlockType::" consensus/` was performed. Additional match sites found:
- `consensus/safety-rules/src/fuzzing_utils.rs` (lines 103, 118, 247) — constructs specific variants for fuzzing, does not match. Not affected.
- `consensus/src/round_manager.rs` (line 1132) — `matches!(..., BlockType::ProposalExt(_))` partial check. Not affected.
- Various test files that construct specific variants. Not affected.
- `block_data.rs:212,216,220` — `matches!` single-variant checks (`is_genesis_block`, `is_nil_block`, `is_opt_block`). Not affected.

### `previous_bitvec()` Handling

`Block::previous_bitvec()` at `block.rs:571-577` matches `DAGBlock` and `OptimisticProposal` explicitly, then uses a `_ =>` fallback that calls `self.quorum_cert().ledger_info().get_voters_bitvec().clone()`. `PrefixConsensusBlock` hits this fallback. The dummy QC has `AggregateSignature::new(BitVec::default(), None)`, so `get_voters_bitvec()` returns an empty `BitVec`.

This empty bitvec flows into `BlockMetadata`/`BlockMetadataExt` consumed by execution. **For the prototype, this is intentionally acceptable**: prefix consensus has no quorum certificate voters to report (the SPC proof is the actual guarantee). DAG avoids this by supplying its own `parents_bitvec`, but DAG's bitvec has different semantics (which nodes were present in the previous round). Prefix consensus has no equivalent concept — there are no rounds with voters, just slot proposals.

If the execution layer later requires a meaningful bitvec, we can add a field to `PrefixConsensusBlock` and match it explicitly. For now, the empty bitvec is correct for our use case.

### `new_block_metadata()` / `new_metadata_with_randomness()` Note

These methods in `block.rs:580-617` call `self.author()`, `self.previous_bitvec()`, and `self.block_data().failed_authors()`. For `PrefixConsensusBlock`:
- `author()` returns `Some(author)` ✓
- `previous_bitvec()` returns empty `BitVec` (see above) ✓
- `failed_authors()` returns `Some(&vec![])` ✓

All produce valid `BlockMetadata` for execution. The empty `failed_authors` and empty bitvec are correct for a leaderless protocol with no voter tracking.

### verify_well_formed() Analysis

`verify_well_formed()` at `block.rs:469-551` does NOT match on `BlockType` — it operates on the `Block` generically. Key checks against our dummy QC:

- `parent.round() < self.round()` — parent from dummy QC has round 0, our round is ≥ 1 ✓
- `parent.epoch() == self.epoch()` — parent from dummy QC has epoch 0, our epoch ≥ 1 ✗ **FAILS**
- `parent.has_reconfiguration()` — false for empty BlockInfo ✓
- `!self.quorum_cert().ends_epoch()` — false for empty QC ✓

**Problem**: The `parent.epoch() == self.epoch()` check will fail because `BlockInfo::empty()` has `epoch: 0` but our block has the real epoch. DAG blocks have the same issue — meaning `verify_well_formed()` is never called on DAG blocks. We must ensure it's never called on our blocks either. Since our blocks are local-only (never received from network, `validate_signature()` bails), this is safe — `verify_well_formed()` is only called on received proposals.

### is_reconfiguration_suffix() Note

`is_reconfiguration_suffix()` at `block_data.rs:422` checks `self.quorum_cert.certified_block().has_reconfiguration()`. With our dummy QC, this always returns `false`. The SlotManager will need an alternative mechanism to detect epoch-ending blocks. This is a Phase 5 concern, not Phase 4 — Phase 4 just builds the block; Phase 5 decides whether to advance or stop.

### Serde Index Stability

**Important**: The `PrefixConsensusBlock` variant MUST be added **after** `DAGBlock` (the current last position in the enum) to avoid shifting DAGBlock's BCS serde enum index. Adding it before DAGBlock would break existing DAGBlock deserialization on the network. Since both variants use `#[serde(skip_deserializing)]`, the new variant at the end is safe.

### v_high Semantics

In SPC, v_high is a **variable-length prefix** (length ≤ n). The inner PC's v_high is the `min_common_extension` of QC3 prefixes (`certify.rs:218`), which returns the longest consistent prefix — a vector of length 0 to n. The strong PC's v_high inherits this length (`certificates.rs:647`, `strong_protocol.rs:333`). The ranking manager correctly uses `v_high.len()` as `committed_prefix_length` (`slot_ranking.rs:49`).

The `build_block_from_v_high()` function uses `v_high.iter().zip(ranking.iter())`, which terminates at the shorter iterator. Entries beyond `v_high.len()` are the excluded validators whose proposals are not committed — they are silently and correctly skipped by `zip`. Within the v_high range, entries may still be `HashValue::zero()` (if a validator didn't send their proposal before the timeout), so the zero-check filter is also needed. The `zip` + zero-check approach handles both variable length and interior ⊥ entries correctly.

Note: `ProposalBuffer::build_input_vector()` produces full-length-n input vectors, but SPC's output v_high may be shorter.

### Payload::extend() Assumption

`Payload::extend()` at `common.rs:357-483` has an `(_, _) => unreachable!()` fallback at line 481 that panics on mismatched `Payload` variants. Since all proposals in the prototype use `Payload::DirectMempool`, and we start aggregation from `Payload::DirectMempool(vec![])`, all `extend()` calls are `DirectMempool + DirectMempool` which concatenates transaction vectors (lines 359-363). This is safe for the prototype. If future work introduces QuorumStore payloads, the initial empty payload and all proposals must use the same variant.

## Implementation Steps

### Step 1: Add `BlockType::PrefixConsensusBlock` variant

**Modified file**: `consensus/consensus-types/src/block_data.rs`

Add **after** `DAGBlock` (last position — preserves serde indices):

```rust
/// A virtual block constructed from Strong Prefix Consensus output.
/// Local-only (never sent over network), like DAGBlock.
#[serde(skip_deserializing)]
PrefixConsensusBlock {
    author: Author,
    failed_authors: Vec<(Round, Author)>,
    validator_txns: Vec<ValidatorTransaction>,
    payload: Payload,
    /// Authors of committed proposals, in ranking order (only non-⊥ entries)
    authors: Vec<Author>,
    /// Slot number in the multi-slot protocol
    slot: u64,
    /// Payload hashes of committed proposals, in ranking order (only non-⊥ entries)
    proposal_hashes: Vec<HashValue>,
    parent_block_id: HashValue,
},
```

### Step 2: Add `BlockData::new_for_prefix_consensus()` constructor

**Modified file**: `consensus/consensus-types/src/block_data.rs`

Add after `new_for_dag()`:

```rust
pub fn new_for_prefix_consensus(
    epoch: u64,
    round: Round,
    timestamp_usecs: u64,
    validator_txns: Vec<ValidatorTransaction>,
    payload: Payload,
    author: Author,
    failed_authors: Vec<(Round, Author)>,
    authors: Vec<Author>,
    slot: u64,
    proposal_hashes: Vec<HashValue>,
    parent_block_id: HashValue,
) -> Self {
    Self {
        epoch,
        round,
        timestamp_usecs,
        quorum_cert: QuorumCert::new(
            VoteData::new(BlockInfo::empty(), BlockInfo::empty()),
            LedgerInfoWithSignatures::new(
                LedgerInfo::new(BlockInfo::empty(), HashValue::zero()),
                AggregateSignature::new(BitVec::default(), None),
            ),
        ),
        block_type: BlockType::PrefixConsensusBlock {
            author,
            validator_txns,
            payload,
            failed_authors,
            authors,
            slot,
            proposal_hashes,
            parent_block_id,
        },
    }
}
```

### Step 3: Update all `BlockType` match arms in `block_data.rs`

**`author()`** (line 137) — add `PrefixConsensusBlock` to the combined arm:
```rust
BlockType::Proposal { author, .. }
| BlockType::DAGBlock { author, .. }
| BlockType::PrefixConsensusBlock { author, .. } => Some(*author),
```

**`parent_id()`** (line 156) — extend the `if let` to match both:
```rust
if let BlockType::DAGBlock { parent_block_id, .. }
    | BlockType::PrefixConsensusBlock { parent_block_id, .. } = self.block_type()
{
    *parent_block_id
} else {
    self.quorum_cert.certified_block().id()
}
```

**`payload()`** (line 167) — add to combined arm:
```rust
BlockType::Proposal { payload, .. }
| BlockType::DAGBlock { payload, .. }
| BlockType::PrefixConsensusBlock { payload, .. } => Some(payload),
```

**`validator_txns()`** (line 178) — add to DAGBlock arm:
```rust
BlockType::DAGBlock { validator_txns, .. }
| BlockType::PrefixConsensusBlock { validator_txns, .. } => Some(validator_txns),
```

**`failed_authors()`** (line 225) — add to combined arm:
```rust
BlockType::Proposal { failed_authors, .. }
| BlockType::NilBlock { failed_authors, .. }
| BlockType::DAGBlock { failed_authors, .. }
| BlockType::PrefixConsensusBlock { failed_authors, .. } => Some(failed_authors),
```

### Step 4: Add `Block::new_for_prefix_consensus()` to `block.rs`

**Modified file**: `consensus/consensus-types/src/block.rs`

Add after `new_for_dag()`:

```rust
pub fn new_for_prefix_consensus(
    epoch: u64,
    round: Round,
    timestamp_usecs: u64,
    validator_txns: Vec<ValidatorTransaction>,
    payload: Payload,
    author: Author,
    authors: Vec<Author>,
    slot: u64,
    proposal_hashes: Vec<HashValue>,
    parent_block_id: HashValue,
) -> Self {
    let block_data = BlockData::new_for_prefix_consensus(
        epoch,
        round,
        timestamp_usecs,
        validator_txns,
        payload,
        author,
        vec![], // failed_authors: no leader = no failed leader
        authors,
        slot,
        proposal_hashes,
        parent_block_id,
    );
    Self {
        id: block_data.hash(),
        block_data,
        signature: None,
    }
}
```

### Step 5: Update `validate_signature()` in `block.rs`

**Modified file**: `consensus/consensus-types/src/block.rs` (line 462)

Add arm after `DAGBlock`:
```rust
BlockType::PrefixConsensusBlock { .. } => {
    bail!("We should not accept PrefixConsensus block from others")
},
```

### Step 6: Create `block_builder.rs`

**New file**: `consensus/prefix-consensus/src/block_builder.rs`

Single public function (no `build_ordered_blocks` — that lives in Phase 5):

```rust
pub fn build_block_from_v_high(
    epoch: u64,
    round: Round,
    slot: u64,
    timestamp_usecs: u64,
    author: Author,
    ranking: &[Author],
    v_high: &PrefixVector,
    payload_map: &HashMap<HashValue, Payload>,
    parent_block_id: HashValue,
    validator_txns: Vec<ValidatorTransaction>,
) -> Block
```

Algorithm:
1. Create empty accumulators: `authors: Vec<Author>`, `proposal_hashes: Vec<HashValue>`, `aggregated_payload = Payload::DirectMempool(vec![])`
2. Iterate `v_high.iter().zip(ranking.iter())` (zip terminates at `v_high.len()`, correctly skipping excluded validators beyond the prefix)
3. For each `(hash, ranked_author)` where `*hash != HashValue::zero()`:
   - Look up `payload_map.get(hash)`
   - If found: push `*ranked_author` to `authors`, push `*hash` to `proposal_hashes`, `aggregated_payload = aggregated_payload.extend(payload.clone())`
   - If not found: skip entirely (do NOT include in authors/proposal_hashes — avoids inconsistency between block metadata and actual content). Log warning — this should not happen if payload resolution in Phase 7 succeeded.
4. Call `Block::new_for_prefix_consensus(epoch, round, timestamp_usecs, validator_txns, aggregated_payload, author, authors, slot, proposal_hashes, parent_block_id)`

### Step 7: Register module in `lib.rs`

**Modified file**: `consensus/prefix-consensus/src/lib.rs`

- Add `pub mod block_builder;`
- Add re-export: `pub use block_builder::build_block_from_v_high;`

### Step 8: Unit tests in `block_builder.rs`

- [ ] `test_build_block_all_non_bot` — 4 validators, all v_high entries non-⊥. Block has 4 authors, aggregated payload contains all transactions, 4 proposal_hashes.
- [ ] `test_build_block_partial` — 4 validators, 2 entries ⊥. Block has 2 authors, only present payloads aggregated.
- [ ] `test_build_block_empty_v_high` — all ⊥. Block has empty payload, no authors, no proposal_hashes. (The SlotManager in Phase 5 may skip block creation for empty v_high, but the builder should handle it gracefully.)
- [ ] `test_build_block_short_v_high` — v_high has length 2 with ranking of 4 validators. Only the first 2 ranked validators' proposals are considered; the remaining 2 are excluded by zip truncation.
- [ ] `test_build_block_ordering` — verify authors and proposal_hashes follow ranking order, not insertion order.
- [ ] `test_build_block_metadata` — verify epoch, round, slot, timestamp, author, parent_block_id are set correctly on the Block.

### Step 9: Verify compilation and tests

- [ ] `cargo check -p aptos-consensus-types` (BlockType changes)
- [ ] `cargo check -p aptos-prefix-consensus` (block_builder module)
- [ ] `cargo test -p aptos-prefix-consensus` — all existing 220 + new tests pass
- [ ] `cargo test -p aptos-consensus-types` — existing tests still pass

## Dependencies

- **Uses from Phase 3**: `PrefixVector`, `HashValue`, `Payload`, `Author`
- **Modifies external crate**: `consensus-types` (BlockType enum, Block constructors)
- **Consumed by Phase 5**: SlotManager calls `build_block_from_v_high()` and handles `PipelinedBlock`/`OrderedBlocks` wrapping inline

## Estimated Size

~300 LOC total:
- `BlockType::PrefixConsensusBlock` variant + constructor: ~40 LOC
- Match arm updates in `block_data.rs`: ~15 LOC
- `Block::new_for_prefix_consensus()` + `validate_signature()` arm: ~30 LOC
- `block_builder.rs` (`build_block_from_v_high` + tests): ~200 LOC

## Open Questions

None — all issues from the review have been resolved:

1. **`build_ordered_blocks` dependency** (blocking) — Resolved: deferred to Phase 5 (SlotManager in `aptos-consensus` crate), matching how DAG does it inline in `adapter.rs`.
2. **`previous_bitvec()` handling** — Resolved: documented that the empty bitvec from the dummy QC fallback is intentionally acceptable for the prototype.
3. **`verify_well_formed()` epoch check** — Documented: never called on local-only blocks (same as DAG).
4. **`is_reconfiguration_suffix()` always false** — Documented: Phase 5 concern.
5. **Payload variant mismatch** — Documented: all proposals use `DirectMempool` in the prototype.
6. **Serde index stability** — Documented: variant added after DAGBlock to preserve indices.
7. **v_high semantics** — Documented: variable-length prefix (length ≤ n). Implementation handles correctly via `zip` + zero-check.
