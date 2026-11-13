# Delayed Fields for MonoMove — Exploration and Proposal

> **Status:** brainstorm. Nothing here is implemented.
> Part 1 surveys how delayed fields work in the V1 VM.
> Part 2 proposes a different mechanism for MonoMove: *behavioural validation* of small closures replayed at commit.

## TL;DR

- V1 delayed fields replace a value with an identifier inside the resource's bytes, and patch the identifier back at commit.
  Almost every ugly part of V1 follows from that one choice: size-preserving encodings, value/ID exchange passes, reads that turn into writes, banned equality and serialization.
- MonoMove does not need identifiers.
  The proposal is a single primitive, `execute(work)`: `work` is a closure that *semantically runs once, at commit, against committed state*.
  The speculative run only predicts what the closure's **observable behaviour** will be.
  At commit the closure is replayed and its observables are compared. If they match, only the closure's writes are patched. If not, the transaction is re-executed, as V1 does today.
- No taint tracking is needed, because Move closures cannot capture references.
  Every channel from the closure to the rest of the transaction is visible at the VM boundary: return value, abort, gas, global storage slots, events.
- The unit of delay is the **storage slot** (resource, group member or table item), which is already the unit of MonoMove's read-write set.
  A slot touched only inside closures is validated behaviourally; a slot touched anywhere else is validated exactly, as today.
- The three hot framework resources are entirely made of aggregators, so slot granularity covers them with no framework change (§2.7).
  The NFT mint path, where a counter flows into another resource and an event, needs a framework refactor to stay parallel (§2.8).

---

## Part 1 — How V1 delayed fields work

### 1.1 Life of a delayed field

1. **Layout tagging.**
   `0x1::aggregator_v2::{Aggregator, AggregatorSnapshot, DerivedStringSnapshot}` are matched by name and wrapped in `MoveTypeLayout::Native` (`third_party/move/move-core/types/src/value.rs`, `IdentifierMappingKind`).
2. **Value → ID exchange on first read.**
   The first read of a resource in a block deserializes it, replaces each delayed value with a fresh `DelayedFieldID` from a block-wide counter, registers the base value in `VersionedDelayedFields`, and re-serializes (`aptos-move/block-executor/src/view.rs`, `replace_values_with_identifiers`).
   From then on the resource's bytes hold IDs.
3. **Execution.**
   Natives operate on IDs. `try_add`/`try_sub` do not read the value: they take a *prediction* (the last committed value) and record a `DeltaHistory`.
   `read` records an exact value read.
4. **Validation, at commit only.**
   Regular Block-STM validation skips delayed fields (`aptos-move/block-executor/src/executor.rs:755`).
   The sequential commit hook (`validate_and_commit_delayed_fields`, `executor.rs:800`) checks each read against the committed base, then resolves deltas to values.
   On failure the transaction is re-executed inside the commit hook (`executor.rs:947`).
5. **Materialization, after commit.**
   Every written resource, every event, and every *read* resource containing a changed ID is deserialized, has its IDs replaced by values, and is serialized again (`aptos-move/block-executor/src/executor_utilities.rs`).

### 1.2 The validation predicate is already behavioural

`DeltaHistory::validate_against_base_value` (`aptos-move/aptos-aggregator/src/delta_math.rs:148`) accepts a base value iff:

```
base + max_achieved_positive_delta <= max      // every successful add still succeeds
base >= min_achieved_negative_delta            // every successful sub still succeeds
base >  max - min_overflow_positive_delta      // every failed add still fails
base <  max_underflow_negative_delta           // every failed sub still fails
```

This does not check that the transaction read the right value.
It checks that every `try_add`/`try_sub` *returns the same bool* on the real value.
It is a hand-compiled path condition for one fixed family of operations.
The proposal in Part 2 generalizes exactly this, from an interval over one integer to the recorded behaviour of an arbitrary small closure.

### 1.3 Ugly parts and why they exist

All of these come from storing an ID where the value should be.

| Constraint | Why |
|---|---|
| Serialized size must be preserved by the exchange. `DerivedStringSnapshot` carries a `padding` field and a 22-byte minimum width, on chain forever. | Gas, storage fees and group sizes are computed at execution time on bytes that hold IDs. |
| Reads become writes (`InPlaceDelayedFieldChange`, `ResourceGroupInPlaceDelayedFieldChange`). | A delta is applied through the ID without touching the resource, but the stored bytes embed the value. |
| Three or more deserialize/serialize round trips per resource. | IDs can be anywhere in a value and the layout is the only map. |
| `==` and comparison abort on delayed values; `bcs::to_bytes` aborts. | IDs are not deterministic. |
| Storable closures cannot capture delayed fields. | Captured values could not be exchanged. |
| At most 10 delayed fields per value; `stake::PendingTransactionFee` is sized around it. | A stand-in for charging for the exchange. |
| `AggregatorSnapshot` and `DerivedStringSnapshot` exist at all. | A value has to reach an event or another resource without being read. |
| Resource groups need synthetic metadata writes and a size re-check at materialization. | Group members are where the hot aggregators live. |
| Aggregator V1 is routed through the same machinery with a hardcoded layout and a legacy-fee flag. | Historical. |
| Mispredictions are only found at commit and are re-executed on the commit path. | No "soft failure" status in the scheduler. |

---

## Part 2 — Proposal: behavioural validation of closures

### 2.1 What MonoMove gives us

- **Values are raw memory and are never rewritten once published.**
  A write is a pointer into the writer's frozen heap, pinned for as long as anyone holds it (`MonoValue::Write`, `aptos-move/block-executor/src/mono_move/mod.rs:78`).
  A reader gets that pointer. There is no serialized form to exchange IDs in, and no reason to want one.
- **Reads are validated by version, not by value** (`version_matches`, `mod.rs:234`).
- **Session effects survive the session.**
  `finish()` (`runtime/src/interpreter.rs:1026`) evacuates the read-write set and every native extension's roots into a `FrozenHeap`.
  A closure object recorded in an extension therefore stays valid, at a stable address, until commit. Recording pointers is enough.
- **Gas is static per basic block.**
  A conditional jump carries `gas_taken` and `gas_fallthrough` (`core/src/instruction/mod.rs:704`).
  Two runs that take the same branches charge the same execution gas.
- **Closures cannot capture references** (`docs/closure_design.md`, "No Reference Capture").
- **BCS sizes are tracked on the write set** (`compute_write_sizes`, `runtime/src/global_storage.rs:554`), including group sizes. Sizes can be validated instead of being forced.
- **The hooks exist but are empty**: `MonoReads::validate_delayed_field_reads` (`mod.rs:288`), `MonoTxnOutput::delayed_field_change_set` (`mod.rs:487`), and the aggregator natives are plain sequential arithmetic (`natives/src/aggregator_v2.rs`).

### 2.2 The primitive

```move
module aptos_framework::delayed {
    /// Semantically identical to `work()`.
    public fun execute<R: copy + drop>(work: || R): R { work() }
}
```

On V1, and on MonoMove with the feature off, this is the whole implementation.
On MonoMove with the feature on, the specializer lowers a call to it into an intrinsic (`DelayedEnter; CallClosure; DelayedExit`).
It has to be an intrinsic: a MonoMove native cannot call back into Move (`NativeStatus` is only `Success`/`Abort`).

The meaning the VM gives it:

> `work` runs **once, at commit**, against the committed state just before this transaction.
> The speculative run produces a *prediction* of the closure's observables, and the rest of the transaction runs on that prediction.

Example, with no special types:

```move
fun increase_supply(metadata_address: address, amount: u128) {
    let ok = delayed::execute(|| {
        let supply = &mut Supply[metadata_address];
        if (supply.maximum - supply.current >= amount) { supply.current += amount; true } else { false }
    });
    assert!(ok, error::out_of_range(EMAX_SUPPLY_EXCEEDED));
}
```

`Supply` is a plain struct of integers. No `Aggregator`, no snapshot, no padding.

### 2.3 Observables: what must match

The speculative run and the commit replay must agree on everything the rest of the transaction, or the chain, can see:

| Observable | Why it is observable |
|---|---|
| Return value (bytes) | The transaction branches on it. |
| Abort code and location | It ends the session. |
| Gas charged inside the closure | It reaches the fee statement, and can cause out-of-gas. |
| Sequence of slot accesses: key, operation, existed or not, BCS size read | It defines the read and write sets, and IO gas is charged on size. |
| For each slot written: write kind (create/modify/delete) and BCS size | Storage fees, refunds, group sizes. |
| For each event emitted: type and BCS size | Gas, and the event's position in the list. |

What is **not** observable, and may differ: the bytes written to slots only closures touch, and the payload of events emitted inside a closure.
Those are taken from the replay.

This is the constant-time literature's definition of non-interference under a leakage model, with the delayed state as the secret.
The leakage trace is control flow (the program-counter model), accessed addresses (here at slot granularity) and operand-dependent cost (here gas).
The return value and abort code are declared public outputs, which the literature calls an output-sensitive policy.

**Trace equality versus observable equality.**
Because the closure is *re-executed* rather than patched symbolically, its internal trace does not have to match.
Only the table above does.
Recording branch outcomes is still worth doing: it is a cheap sufficient condition, it gives an early exit on the first divergent branch, and with static per-block gas it implies equal execution gas.
The recommendation is to compare the observables directly and to fold branch outcomes into a rolling digest for early exit.

### 2.4 Why no taint tracking is needed

A sanitizer-style design would let delayed values flow anywhere and flag the moment one reaches a branch or an address.
That needs shadow state for every value, which MonoMove's untyped, `memcpy`-moved memory does not have. V1's typed IDs are that shadow state.

Closures avoid the problem structurally.
A Move closure cannot capture a reference, and its captured values are moved in.
So the closure can affect the rest of the transaction only through its return value, an abort, gas, global storage, and events.
All five cross the VM boundary, where they can be recorded.

Global storage is the one channel that needs a rule, which is the next section.

### 2.5 Slot modes

Each slot a transaction touches is in one of two modes.

- **Behavioural**: touched only inside `execute` closures. Its version is *not* validated. It is validated by replay.
- **Exact**: touched outside a closure at least once. Validated by version, as today.

Rules:

1. A slot starts Behavioural if its first access is inside a closure, Exact otherwise.
2. Any access outside a closure makes it Exact, permanently for this incarnation.
3. At session end, build the graph of closures and the slots they touched.
   Every connected component that contains an Exact slot becomes Exact as a whole, and its closures are dropped from the log.
   This is always sound: exact reads guarantee identical behaviour.

Rule 3 exists because the replay runs closures back to back, without the code between them.
A closure may therefore only depend on slots that non-closure code never touches.

**Soundness sketch.**
By induction over the log.
Assume closures `1..k-1` replayed with the same observables as predicted.
Then all non-closure state before closure `k` is the same as in the speculative run: exact reads are validated, and every value that crossed out of an earlier closure matched.
So closure `k` is called with identical captured values.
The Behavioural slots it sees are the committed state plus the replayed effects of closures `1..k-1`, which is precisely what a sequential execution would give it.
If its observables match too, the induction continues. If not, the transaction is re-executed.

### 2.6 Speculative execution, commit, and Block-STM

**During execution.** A `DelayedLog` native extension records, per `execute`:

- a rooted pointer to the closure object (function and captured values),
- the observables of §2.3,
- the checkpoint epoch.

It participates in `checkpoint`/`rollback` like every extension (`runtime/src/interpreter.rs:985`).
An entry from a rolled-back session is kept as *validate-only*: its result still steered the transaction (it may be why the session aborted), but its effects must be undone during replay.
So the log is a sequence of `Call`, `Checkpoint` and `Rollback(n)`, and the replay mirrors it.

**At session end.** `finish()` classifies writes.
A slot written only by closures becomes a new write class, `Delayed`, next to `Creation`/`Modification`/`Deletion` (`runtime/src/global_storage.rs:140`).
It is published to the multi-version map like any other write, flagged.

**Reads against the multi-version map.**

- A *behavioural* read takes the latest entry below it as a prediction and records nothing to validate.
  It never needs to wait on a dependency: any plausible value will do, including an estimate.
- An *exact* read that lands on a `Delayed` entry records the entry's identity (its pointer).
  It is re-checked in the reader's own commit hook, because the writer may patch the entry when it commits.
  This is the equivalent of V1's `DelayedFieldRead::Value`, and it means a patch never has to invalidate anyone eagerly.

**In the commit hook**, for transaction `j`, where everything below `j` is committed and final:

1. Re-check exact reads that landed on `Delayed` entries.
2. **Fast path.** If every slot the closures read is still the same entry (same pointer) as during speculation, the prediction was exact. Done.
3. **Replay.** Open a small session over the committed state below `j` and walk the log.
   Compare observables as they are produced, and stop at the first mismatch.
4. On a match, for each `Delayed` slot whose replayed bytes differ from the speculative ones, replace `j`'s entry with the replayed value.
   The new value is a fresh allocation, so no thread ever sees memory change under it.
   Events emitted in closures are replaced at their recorded positions.
5. On a mismatch, re-execute `j` in the commit hook. `execute_txn_after_commit` (`executor.rs:837`) already does this for V1.

Materialization is unchanged: the committed entries hold final values, and they are serialized once, after commit, as today.
Group sizes need nothing extra, because a `Delayed` write is required to keep its BCS size.

This needs one new hook on `SingleTransactionExecutor`, called from `validate_and_commit_delayed_fields`, since the replay needs the executor and the data map, and the existing `validate_delayed_field_reads` only receives the delayed-field view.

**Bounding the commit path.**
The replay runs in the sequential commit hook, so it must be small.

- A closure has a budget in gas units, enforced during the speculative run.
  Since the replay must charge the same gas, the budget bounds the replay too.
- Over budget, the closure's slots become Exact and nothing is replayed. This is always sound.
- The fast path means replay is only paid under real contention.
- Replay is not charged. A premium on `execute` can pay for it.

### 2.7 The existing `aggregator_v2` API

Existing code borrows outside the native:

```move
let supply = &mut ConcurrentSupply[metadata_address];
assert!(supply.current.try_add(amount), ...);
```

Under §2.5 the outer borrow would make the slot Exact.
But the hot framework resources are made *only* of aggregators:

- `fungible_asset::ConcurrentSupply { current: Aggregator<u128> }`
- `fungible_asset::ConcurrentFungibleBalance { balance: Aggregator<u64> }`
- `collection::ConcurrentSupply { current_supply, total_minted: Aggregator<u64> }`
- the Aggregator V1 table item behind `CoinInfo.supply`, which is a bare `u128`

Aggregator fields are private and the types have no `copy`.
So for a resource type whose every leaf is an aggregator (call it *opaque*), non-framework code holding a reference still cannot observe the contents.

**Rule.** `exists`, `borrow_global` and `borrow_global_mut` of an opaque type do not make the slot Exact.
The `aggregator_v2` natives then act as built-in closures: the log entry is the native plus its arguments, and the replay calls the native again.
The reference argument is resolved to a slot by looking its base pointer up among the read-write set's entries of opaque types.

The few generic operations that could still observe or move the bytes make the slot Exact (or abort, where V1 aborts and parity requires it):
`==`, comparison, BCS, hashing and string natives, `move_from`, `mem::swap`/`replace`.
Whether a type is opaque is a static, per-type flag, and MonoMove is monomorphic, so these checks cost nothing for other types.

`read`, `snapshot` and `derive_string_concat` outside a closure make the slot Exact.
The first matches V1. The other two are a regression against V1, addressed next.

Aggregators that share a resource with ordinary fields, or sit in a container (`stake::PendingTransactionFee` holds a `BigOrderedMap<u64, Aggregator<u64>>`, written once per block), are simply Exact.
Correct, not parallel.

### 2.8 Values that flow elsewhere: the NFT mint path

`collection::increment_supply` bumps `total_minted`, emits a `Mint` event with a snapshot of it, and returns a snapshot that `token` turns into a name and stores in `TokenIdentifiers`.
The counter's value reaches an event and another resource without anyone branching on it.

In the closure model, a value may flow anywhere *inside* a closure.
So the source and its sinks have to be in one closure:

```move
delayed::execute(|| {
    let supply = &mut ConcurrentSupply[collection_addr];
    // bump, emit Mint { index }, then hand `index` to a continuation that writes TokenIdentifiers
});
```

The freshly created `TokenIdentifiers` slot is Behavioural (only the closure touched it), and the event payload is replaced at replay.
On V1 the same code runs as a plain call and the aggregators inside behave as they do now, so one framework serves both VMs.

Token names of different lengths ("#99" versus "#100") no longer need padding to keep sizes stable: a size change is an observable mismatch and costs one re-execution at each digit boundary.
Existing types keep their on-chain layout, padding included.

If refactoring the framework turns out to be unacceptable, the fallback is a placeholder that travels with the value, as in V1.
That brings back a traversal at materialization for types containing snapshots, though not the size or exchange constraints.
It should be a last resort.

### 2.9 What goes away, what is new

**Gone, relative to V1:** identifiers, value/ID exchange, `Native` layouts, width and padding rules for new code, reads becoming writes, materialization traversals, the equality and BCS bans inside closures, the 10-field limit, the closure-capture restriction, and the need for snapshot types in new code.

**New, and the risks:**

- A replay on the sequential commit path. Bounded by the budget, skipped by the fast path.
- Closures must be deterministic given their captures and the slots they touch.
  Natives callable inside a closure need an allowlist (no randomness; transaction-context natives are constants and fine).
- A transaction can force its own re-execution by writing a closure that mispredicts.
  This costs what an ordinary conflict costs today.
- A slot is parallel only if every hot access goes through closures. One exact read of a hot slot is a sequential dependency, as `read` is in V1.

### 2.10 Alternatives considered

1. **Port V1.** A `u64` ID fits the field, so it would work. It keeps every constraint in §1.3.
2. **Delayed regions inside a slot** (aggregators next to ordinary fields).
   Needs a way to find a cell's committed counterpart after the transaction restructured its copy of the resource, and to know whether a cell at a local address came from storage.
   Both point to a hidden per-cell tag in the in-memory layout. Worth revisiting only if mixed resources prove hot.
3. **Symbolic repair without replay** (record an operation log in SSA form and re-apply only the affected operations).
   More machinery than replaying a closure that is a handful of micro-ops.
4. **Closed-form guards** such as V1's intervals. Not needed while the closures are this small. Possible later as a specialization of hot built-in closures.

### 2.11 Related work

Only Forerunner was read in the primary source for this document. The rest is from abstracts and secondary sources and should be checked before being relied on.

- **Forerunner** (Chen et al., SOSP 2021). Speculates Ethereum transactions before the block and accepts the result if the real execution is *CD-equivalent*: same instruction sequence, same data dependencies for variable-location accesses. Checked by control and data guards. It needed explicit guards for gas. Its guards all run before any state write, so a failed guard has nothing to undo.
- **ParallelEVM** (Lin et al., EuroSys 2025). Moves the same idea to commit time inside a block: an SSA operation log, and a redo of only the operations that depend on conflicting slots, guarded by assertions on control flow and addresses.
- **RETCON** (Blundell et al., ISCA 2010). Hardware transactional memory that tracks symbolic values and re-applies them to final inputs at commit, with branch conditions kept as constraints.
- **MV3C** (Dashti et al., SIGMOD 2017). A transaction is a DAG of predicates with closures; repair re-runs only invalidated closures.
- **Escrow** (O'Neil, TODS 1986) and V1's `DeltaHistory`. Interval predicates over a hot counter.
- **Constant-time verification** (the program-counter model of Molnar et al., ct-verif by Almeida et al.). The source of the leakage-model vocabulary in §2.3. `ctgrind`-style tools are the taint alternative rejected in §2.4.
- **RapidLane** (Mitenkov et al.). The paper behind V1's delayed fields.

Common lessons: define equivalence as a trace of leaked observations and include cost in it; put guards before effects; keep full re-execution as the fallback.

### 2.12 Suggested order of work

1. Runtime: the `DelayedLog` extension, the `execute` intrinsic, slot modes, recording of observables. Testable in the unit-test backend with a hand-driven replay.
2. Block-STM: the `Delayed` write class, the commit hook, patching. Testable in `combinatorial_tests/mono_mock_executor.rs`.
3. Opaque slots and the `aggregator_v2` / Aggregator V1 natives as built-in closures. This makes APT supply and fungible-asset balances parallel with no framework change.
4. `0x1::delayed::execute` in the framework and the token mint refactor.
5. Optimizations: early (pre-commit) soft validation, in-place patching of flat slots, closed-form guards.

### 2.13 Open questions

- Should `execute` be public, or framework-only at first to limit what runs on the commit path?
- May a closure read an Exact slot that non-closure code only read and never wrote? It is sound and would relax rule 3 of §2.5.
- What is the budget, and is it per closure or per transaction?
- BlockSTMv2 invalidates readers on write. An exact reader of a `Delayed` entry must be re-checked at its own commit rather than invalidated when the entry is patched; confirm this fits its dependency tracking.
- V1 parity for the abort codes of `==` and `bcs::to_bytes` on aggregators: abort as V1 does, or make the slot Exact and return the value?
- Dispatchable fungible-asset hooks run arbitrary code. Allowed inside a closure under the budget, or excluded?
