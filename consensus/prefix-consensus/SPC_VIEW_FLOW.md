# SPC View Progression Flow & Investigation Items

## Architecture

| Component | File |
|-----------|------|
| Pure state machine (no IO) | `strong_protocol.rs` |
| Async event loop | `strong_manager.rs` |
| Per-view state & ranking | `view_state.rs` |
| Certificate types | `certificates.rs` |
| 3-round inner PC per view | `inner_pc_impl.rs` / `inner_pc_trait.rs` |

## View 1 (Special)

- Input: **raw proposal hashes** (not certificates)
- Runs 3-round inner PC: Vote1 -> QC1 -> Vote2 -> QC2 -> Vote3 -> QC3
- Produces `(v_low, v_high)`

### View 1 Completion Decision (`strong_manager.rs:439-498`)

| Condition | Path | Action |
|-----------|------|--------|
| `v_low.len() == n` (all validators) | Fast path | v_high == v_low by Upper Bound property. Commit immediately, broadcast `StrongPCCommit` with empty certificate chain |
| `v_low.len() < n` | Normal path | Create `DirectCertificate` from View 1's QC3, call `propose_and_enter(view=2, cert)` |

## Views 2, 3, 4, ... (General Case)

### Entering a View (`strong_manager.rs:545-596`)

1. `propose_and_enter(next_view, cert)` broadcasts a `ViewProposal` with the certificate from the previous view
2. `enter_view(view)` creates a `ViewState`, then either:
   - Starts inner PC immediately if rank-0 party's certificate already arrived
   - Sets 1-second `VIEW_START_TIMEOUT` timer as fallback

### Building Inner PC Input (`view_state.rs:274-290`)

For views > 1, input is **certificate hashes** (not raw proposals):

```
For each party in ranking order:
  if party sent a certificate -> push cert_hash, STOP
  else -> push bot (HashValue::zero)
```

Only entries up to the first non-bot are included (entries after are provably unused).

### Three-Way Decision at View Completion (`strong_protocol.rs:238-264`)

| Condition | Decision | Action |
|-----------|----------|--------|
| `v_low` has non-bot entry | **Commit** | Trace certificate chain back to View 1, broadcast `StrongPCCommit` |
| `v_high` has non-bot entry | **DirectCert** | Create `DirectCertificate(view, QC3)`, call `propose_and_enter(view+1, cert)` |
| Both all-bot | **EmptyView** | Broadcast `EmptyViewMessage`, collect >1/3 stake, create `IndirectCertificate`, call `propose_and_enter(view+1, cert)` |

## Certificate Types (`certificates.rs`)

- **DirectCertificate**: `{view, QC3}` — created when v_high is non-empty. Proves something was decided.
- **IndirectCertificate**: `{empty_view, parent_view, parent_proof, messages}` — created when a view is entirely empty. Contains >1/3 stake `EmptyViewMessage`s, points back to highest known non-empty view.

## Trace-Back on Commit (`strong_protocol.rs:285-341`)

When v_low has a non-bot entry, trace the certificate chain back to View 1:

```
View 5 commits: v_low = [H(cert_v3), bot, bot]
  -> look up cert_v3 (DirectCert from View 3)
  -> cert_v3.v_high -> [H(cert_v2), ...]
  -> look up cert_v2 (DirectCert from View 2)
  -> cert_v2.v_high -> [H(cert_v1), ...]
  -> cert_v1 is from View 1 -> TERMINAL
  -> Final v_high = cert_v1.v_high (actual transaction hashes)
```

## Ranking Rotation (`view_state.rs:47-116`)

Each view rotates the ranking left by `(view-1) % n`:

```
View 1: [p1, p2, p3, p4]
View 2: [p2, p3, p4, p1]
View 3: [p3, p4, p1, p2]
View 4: [p4, p1, p2, p3]
```

Ensures leaderless progress: even if an adversary blocks the rank-0 party each view, eventually an honest party occupies rank 0.

## Event Loop (`strong_manager.rs:263-390`)

Biased `select!` priority order:

1. **Close signal** — shutdown
2. **View start timer** — fallback if rank-0 proposal doesn't arrive
3. **Priority messages** — `ViewProposal`, `EmptyViewMessage`, `StrongPCCommit`
4. **Regular messages** — Inner PC votes, fetch requests

Loop exits when `protocol.is_complete()` returns true.

## Full Flow Summary

```
View 1: raw inputs -> 3-round PC -> (v_low, v_high)
  |-- v_low full -> COMMIT immediately
  +-- else -> DirectCert -> View 2

View 2+: cert hashes -> 3-round PC -> (v_low, v_high)
  |-- v_low non-bot -> COMMIT (trace back to View 1)
  |-- v_high non-bot -> DirectCert -> View N+1
  +-- all-bot -> IndirectCert -> View N+1
```

---

## Investigation Results

### 1. DirectCert vs EmptyView Split

**Scenario**: After a view's inner PC completes, some nodes see a non-bot entry in v_high and create a `DirectCertificate`, while other nodes see all-bot and broadcast `EmptyViewMessage`.

**Answer: YES — this split CAN happen.** Different honest nodes can form different QC3s (from different vote subsets), yielding different v_high values. The root cause is Byzantine equivocation in Round 2.

#### How the split happens

The key insight: `v_high = min_common_extension({mcp_prefix values in QC3})`. If **all** mcp_prefix values in a QC3 are empty, v_high is empty (EmptyView). If **any** mcp_prefix is non-empty (and all are consistent), v_high is non-empty (DirectCert). Different nodes can form QC3s with different vote compositions.

**Concrete example** (n=4, f=1, validators A/B/C honest, D Byzantine):

**Round 1**: All honest nodes vote with the same `input_vector = [H_cert]`. Any QC1 from ≥3 votes yields `certified_prefix = [H_cert]` (minority threshold = 2, at least 2 honest votes carry `[H_cert]`). All honest Vote2s carry `certified_prefix = [H_cert]`. No split here.

**Round 2 — the split point**: D equivocates:
- Sends Vote2 with `certified_prefix = [H_cert]` to A
- Sends Vote2 with `certified_prefix = []` to B and C
- Adversary delays honest Vote2 delivery so each node forms QC2 before seeing all honest votes

Result:
- A forms QC2 from {A, B, D_good}: prefixes = `[H_cert], [H_cert], [H_cert]` → **mcp = `[H_cert]`**
- B forms QC2 from {A, B, D_bad}: prefixes = `[H_cert], [H_cert], []` → **mcp = `[]`** (because `max_common_prefix` returns `[]` when any vector has length 0 — `utils.rs:37`: `if min_len == 0 { return Vec::new(); }`)
- C forms QC2 from {A, C, D_bad}: same → **mcp = `[]`**

Vote3 mcp_prefix values: A=`[H_cert]`, B=`[]`, C=`[]`

**Round 3 — different QC3s**:
- **A** forms QC3 from {A, B, C}: mcp_prefixes = `[H_cert], [], []`
  - `all_consistent` → true (`[]` is prefix of `[H_cert]`)
  - `v_high = min_common_extension = [H_cert]` (longest vector) → **DirectCert**
- **B** forms QC3 from {B, C, D_empty}: mcp_prefixes = `[], [], []`
  - `v_high = min_common_extension = []` → **EmptyView**

#### Recovery mechanism — EmptyView nodes are NOT stuck

EmptyView nodes recover via the **catch-up mechanism** in `process_proposal` (`strong_manager.rs:1032-1050`):

1. DirectCert nodes (A) call `propose_and_enter(view+1, DirectCert)`, broadcasting a ViewProposal for the next view.

2. EmptyView nodes (B) broadcast EmptyViewMessage and start collecting. B may not gather enough (needs >1/3 stake = 2, has only 1 from itself). But the event loop keeps processing messages.

3. When A's ViewProposal arrives at B with `target_view > current_view`:
   ```
   // strong_manager.rs:1049-1050
   self.propose_and_enter(target_view, proposal.certificate).await;
   ```
   B **adopts A's DirectCert**, broadcasts it as its own proposal, and enters the next view. The IndirectCert path becomes moot.

#### Consequence

- **Safety**: Not affected. The Consistency Lemma guarantees that any v_low committed by one honest node is consistent with any v_high produced by another. The split is between DirectCert and EmptyView, not between conflicting commits.
- **Liveness**: EmptyView nodes catch up within one extra message round-trip (waiting for a ViewProposal from a DirectCert node).
- **Latency cost**: ~1 additional network round-trip per view where the adversary causes this split. Ranking rotation limits how many consecutive views the adversary can disrupt.

#### Key code references

| What | Where |
|------|-------|
| `max_common_prefix` returns `[]` if any vector empty | `utils.rs:37` |
| `min_common_extension` returns longest consistent vector | `utils.rs:82-100` |
| `qc2_certify` = `max_common_prefix` of certified_prefixes | `certify.rs:163-177` |
| `qc3_certify` produces v_low/v_high | `certify.rs:203-224` |
| Three-way decision (DirectCert vs EmptyView) | `strong_protocol.rs:246-264` |
| Catch-up: adopt future-view proposal | `strong_manager.rs:1032-1050` |

### 2. StrongPCCommit Flow — Stuck Nodes on Fast Path

**Scenario**: View 1 fast path fires (`v_low.len() == n`). The node broadcasts `StrongPCCommit` with an empty certificate chain and moves to the next slot. Other nodes may not have completed View 1 yet.

**Answer: Nodes do NOT get stuck.** The protocol handles this correctly through self-contained validation and message buffering.

#### How commit reception works

When a node receives `StrongPCCommit` (`strong_manager.rs:1105-1131`):

1. `process_commit` calls `protocol.process_received_commit(&commit, &verifier)` (`strong_protocol.rs:352-361`)
2. This delegates to `commit.verify(verifier)` (`certificates.rs:710-741`)
3. **Validation is self-contained** — the QC3 committing proof is inside the message. The receiver does NOT need its own View 1 output. For empty-chain (fast path), verify checks:
   - QC3 signatures valid (from the proof embedded in the message)
   - Empty chain → committing view == 1
   - `v_low.len() == n` (derived from the embedded QC3)
   - `v_high == v_low`
4. On success: sets `protocol.committed = true`, stores `v_high`
5. Event loop checks `protocol.is_complete()` → true → **breaks** (`strong_manager.rs:345-353`)

The inner PC is not a separate spawned task — it's in-memory state in `pc_states` that gets dropped with the manager when the event loop exits.

#### Cross-slot synchronization

If Node A advances to slot N+1 before Node B finishes slot N:

- **A** starts broadcasting slot N+1 proposals
- **B** is still running slot N's SPC. SlotManager routes slot N+1 messages to the **buffer** (`slot_manager.rs:1474-1493`):
  ```
  // SPC not yet spawned for this slot (current pre-spawn or future slot).
  self.spc_priority_buffer.entry(slot).or_default().push((author, msg));
  ```
- When B finishes slot N and starts slot N+1, it **drains the buffer** (`slot_manager.rs:798-821`)
- Messages for past slots (`slot < current_slot`) are dropped (`slot_manager.rs:1423-1430`), which is correct — the node has already committed that slot

#### Summary

| Concern | Status |
|---------|--------|
| Receiver needs own View 1 output? | No — validation uses the embedded QC3 |
| Inner PC aborted on commit? | Not needed — event loop breaks, state is dropped |
| Cross-slot message loss? | No — future-slot messages are buffered and drained |
| Fast node's proposals arrive early? | Buffered by slow node's SlotManager |

#### Key code references

| What | Where |
|------|-------|
| `process_commit` handler | `strong_manager.rs:1105-1131` |
| `process_received_commit` sets committed | `strong_protocol.rs:352-361` |
| Empty-chain fast-path validation | `certificates.rs:719-741` |
| Event loop break on `is_complete()` | `strong_manager.rs:345-353` |
| Future-slot message buffering | `slot_manager.rs:1474-1493` |
| Buffer drain on SPC start | `slot_manager.rs:798-821` |
| Past-slot message drop | `slot_manager.rs:1423-1430` |
