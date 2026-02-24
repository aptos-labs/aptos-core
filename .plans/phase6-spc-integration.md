# Plan: Phase 6 — SPC Integration

## Goal

Replace the SPC stub in `SlotManager::run_spc()` with a real `DefaultStrongPCManager` spawn, using the generic network bridge for SPC messaging and a `output_tx` channel for reporting v_high back to SlotManager.

## Approach

Follow the same pattern as `EpochManager` (lines 2178-2222):
1. Create `aptos_channels::new_unbounded()` → `(spc_tx, spc_rx)`
2. Create `StrongConsensusNetworkBridge::new(consensus_network_client.clone())`
3. Create `StrongPrefixConsensusNetworkClient::new(bridge)`
4. Create `StrongNetworkSenderAdapter::new(author, network_client, spc_tx.clone(), validators)`
5. Create `DefaultStrongPCManager::new(...)` with output channel
6. Spawn `manager.run(spc_rx, close_rx)`
7. Store `spc_tx` for forwarding incoming messages, `output_rx` for receiving v_high

SPC sends directly on the network via its `StrongNetworkSenderAdapter` — no intermediate channel adapter needed. Self-sends go through `spc_tx`, same channel as forwarded network messages.

### Channel types

- **SPC messages in**: `aptos_channels::UnboundedSender/Receiver<(Author, StrongPrefixConsensusMsg)>` (futures mpsc with gauge — required by `StrongPrefixConsensusManager::run()`)
- **SPC output**: `tokio::sync::mpsc::UnboundedSender/Receiver<(u64, PrefixVector)>` — slot + v_high sent by strong manager on commit
- **SPC close**: `futures::channel::oneshot` with ack pattern (same as existing)

## Implementation Steps

### Step 1: Add `output_tx` to `StrongPrefixConsensusManager`

**File**: `consensus/prefix-consensus/src/strong_manager.rs`

- Add field: `output_tx: Option<tokio::sync::mpsc::UnboundedSender<(u64, PrefixVector)>>`
- Add constructor parameter (10th, after `validator_verifier`)
- Add helper:
  ```rust
  fn send_output(&self, v_high: &PrefixVector) {
      if let Some(tx) = &self.output_tx {
          let _ = tx.send((self.slot, v_high.clone()));
      }
  }
  ```
- Call at both commit paths:
  1. `handle_commit_decision` (line 555): after `self.protocol.set_committed(v_high)` → `self.send_output(&v_high)`
  2. `process_commit` (line 878): after successful `process_received_commit` → `self.send_output(self.protocol.v_high().unwrap())`

**File**: `consensus/src/epoch_manager.rs` — pass `None` as the 10th arg to `DefaultStrongPCManager::new()`

Compile + run 226 existing tests (they all pass `None` via their mock constructors).

### Step 2: Make SPC spawning pluggable via trait

SlotManager needs to spawn SPC instances, but tests shouldn't require real network bridges or multi-party crypto. Extract SPC creation into a trait:

```rust
pub trait SPCSpawner: Send + Sync {
    fn spawn_spc(
        &self,
        slot: u64,
        epoch: u64,
        author: Author,
        input_vector: PrefixVector,
        ranking: Vec<Author>,
        validator_signer: &ValidatorSigner,
        validator_verifier: &Arc<ValidatorVerifier>,
    ) -> SPCHandles;
}

pub struct SPCHandles {
    pub msg_tx: aptos_channels::UnboundedSender<(Author, StrongPrefixConsensusMsg)>,
    pub output_rx: tokio::sync::mpsc::UnboundedReceiver<(u64, PrefixVector)>,
    pub close_tx: futures::channel::oneshot::Sender<futures::channel::oneshot::Sender<()>>,
}
```

**Production impl** (`RealSPCSpawner`): holds `ConsensusNetworkClient`, creates bridge + adapter + `DefaultStrongPCManager`, spawns task, returns handles.

**Test impl** (`StubSPCSpawner`): spawns the existing lightweight task that immediately returns input vector as v_high.

SlotManager becomes: `SlotManager<NS: SubprotocolNetworkSender<SlotConsensusMsg>, SP: SPCSpawner>`

### Step 3: Update SlotManager fields and channel types

**File**: `consensus/src/prefix_consensus/slot_manager.rs`

- Change `spc_msg_tx` from `Option<tokio::sync::mpsc::UnboundedSender<...>>` to `Option<aptos_channels::UnboundedSender<(Author, StrongPrefixConsensusMsg)>>`
- Change `spc_output_rx` from `Option<tokio::sync::mpsc::UnboundedReceiver<SPCOutput>>` to `Option<tokio::sync::mpsc::UnboundedReceiver<(u64, PrefixVector)>>`
- Add `spc_close_tx: Option<futures::channel::oneshot::Sender<futures::channel::oneshot::Sender<()>>>`
- Add `spc_spawner: SP` field
- Remove `SPCOutput` struct (use tuple directly), or keep and construct from tuple

### Step 4: Replace `run_spc()` stub

```rust
fn run_spc(&mut self, slot: u64) {
    // ... prepare input (same as before) ...

    let handles = self.spc_spawner.spawn_spc(
        slot, self.epoch, self.author,
        input_vector,
        self.ranking_manager.current_ranking().to_vec(),
        &self.validator_signer,
        &self.validator_verifier,
    );

    self.spc_msg_tx = Some(handles.msg_tx);
    self.spc_output_rx = Some(handles.output_rx);
    self.spc_close_tx = Some(handles.close_tx);
}
```

### Step 5: Update `process_spc_message` for `aptos_channels` sender

`aptos_channels::UnboundedSender` uses `Sink` trait (async `.send()`). Change `process_spc_message` to async:
```rust
async fn process_spc_message(&mut self, author: Author, slot: u64, msg: StrongPrefixConsensusMsg) {
    if let Some(tx) = &mut self.spc_msg_tx {
        let _ = tx.send((author, msg)).await;
    }
}
```

### Step 6: Update `on_spc_v_high` for tuple channel

Receive `(slot, v_high)` tuple instead of `SPCOutput` struct. Also drop `spc_close_tx` on completion.

### Step 7: Update `on_spc_v_high` cleanup to signal SPC shutdown

After receiving v_high:
```rust
self.spc_msg_tx.take();   // drop sender → SPC's message_rx closes
self.spc_output_rx.take();
self.spc_close_tx.take();  // drop close channel (SPC already exited)
```

### Step 8: Update tests

- `StubSPCSpawner` replaces inline stub code — same behavior (immediate v_high return)
- `MockSlotNetworkSender` unchanged
- Existing 9 tests work with `StubSPCSpawner`
- `build_test_manager` updated to accept `StubSPCSpawner`

### Step 9: Implement `RealSPCSpawner`

Holds `ConsensusNetworkClient<NC>` (cloneable). In `spawn_spc`:
```rust
let (spc_tx, spc_rx) = aptos_channels::new_unbounded_test();
let (output_tx, output_rx) = tokio::sync::mpsc::unbounded_channel();
let (close_tx, close_rx) = futures::channel::oneshot::channel();

let bridge = StrongConsensusNetworkBridge::new(self.consensus_client.clone());
let network_client = StrongPrefixConsensusNetworkClient::new(bridge);
let network_sender = StrongNetworkSenderAdapter::new(author, network_client, spc_tx.clone(), validator_verifier.clone());

let manager = DefaultStrongPCManager::new(
    author, epoch, slot, ranking, input_vector,
    network_sender, validator_signer.clone(), validator_verifier.clone(),
    Some(output_tx),
);

tokio::spawn(manager.run(spc_rx, close_rx));

SPCHandles { msg_tx: spc_tx, output_rx, close_tx }
```

### Step 10: Update lib.rs exports and compile

Export `SPCSpawner`, `SPCHandles`, `RealSPCSpawner` (if needed by EpochManager).

### Step 11: Verify

- `cargo check -p aptos-prefix-consensus`
- `cargo check -p aptos-consensus`
- `cargo test -p aptos-prefix-consensus` (226 tests)
- `cargo test -p aptos-consensus -- prefix_consensus` (9 tests)
- `cargo test -p smoke-test -- prefix_consensus` (6 smoke tests)

## Files Modified

1. `consensus/prefix-consensus/src/strong_manager.rs` — Add `output_tx` field, `send_output()`, constructor param
2. `consensus/src/prefix_consensus/slot_manager.rs` — Add `SPCSpawner` trait, `SPCHandles`, `StubSPCSpawner`, `RealSPCSpawner`; replace stub; update channel types
3. `consensus/src/epoch_manager.rs` — Pass `None` for `output_tx`
4. `consensus/prefix-consensus/src/lib.rs` — Export new types if needed

## Open Questions

1. **`RealSPCSpawner` location**: Should it live in `slot_manager.rs` (alongside `SlotManager`) or in a separate file? It depends on `ConsensusNetworkClient` (from `aptos-consensus`) so it must be in the `aptos-consensus` crate, not `aptos-prefix-consensus`. Recommend keeping it in `slot_manager.rs`.

2. **`SPCSpawner` trait location**: The trait itself has no crate-specific dependencies (just `aptos_channels`, `tokio`, `futures`, and prefix-consensus types). It could live in `aptos-prefix-consensus` for reuse, or in `slot_manager.rs` for simplicity. Recommend `slot_manager.rs` since only SlotManager uses it.

3. **ValidatorSigner cloning**: `SPCSpawner::spawn_spc` takes `&ValidatorSigner`. The `RealSPCSpawner` needs to pass it to `DefaultStrongPCManager::new()` which takes owned. `ValidatorSigner` contains `Arc<PrivateKey>` so `.clone()` is cheap. This is fine.
