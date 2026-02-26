# Phase 8: EpochManager Integration for SlotManager

## Context

SlotManager (Multi-Slot Prefix Consensus, Algorithm 4) is fully functional as a standalone component through Phases 1-7. It needs to be wired into the EpochManager so it starts automatically at epoch boundaries when the config flag is set — just like DAG and Jolteon are started today. Without this, SlotManager can only be tested via manual channel wiring in unit tests.

## Goal

Add `enable_prefix_consensus: bool` to local `ConsensusConfig`, create `start_new_epoch_with_slot_manager()` in EpochManager, route `SlotConsensusMsg` through `check_epoch()`, and handle shutdown — so that setting the config flag causes the validator to run prefix consensus instead of Jolteon/DAG.

## Files to Modify

1. `config/src/config/consensus_config.rs` — add config flag
2. `consensus/src/prefix_consensus/slot_manager.rs` — change channel types from tokio to futures
3. `consensus/src/epoch_manager.rs` — main integration (new fields, startup, routing, shutdown)

## Pre-Step: Channel Type Alignment

SlotManager was built with tokio channel types but the rest of the consensus codebase uses futures::channel types. Three mismatches must be fixed:

### Mismatch 1: execution_channel
- **SlotManager uses**: `tokio::sync::mpsc::UnboundedSender<OrderedBlocks>` (slot_manager.rs:48-50, line 248)
- **execution_client returns**: `futures::channel::mpsc::UnboundedSender<OrderedBlocks>` (execution_client.rs:56,88)
- **Fix**: Change SlotManager's `execution_channel` field to `futures::channel::mpsc::UnboundedSender<OrderedBlocks>`. Change `.send()` call (line 607) to `.unbounded_send()`.

### Mismatch 2: message_rx
- **SlotManager uses**: `tokio::sync::mpsc::UnboundedReceiver<(Author, SlotConsensusMsg)>` (slot_manager.rs:50, line 279)
- **EpochManager needs**: `aptos_channels::UnboundedSender`/`Receiver` (wraps `futures::channel::mpsc` with IntGauge — `crates/channel/src/lib.rs:154-163`)
- **Fix**: Change SlotManager's `start()` to accept `aptos_channels::UnboundedReceiver<(Author, SlotConsensusMsg)>`. The `tokio::select!` event loop must change from `message_rx.recv()` to `message_rx.select_next_some()` (or `message_rx.next()`) since `aptos_channels::UnboundedReceiver` implements `futures::Stream`, not tokio's `recv()`.

### Mismatch 3: close_rx
- **SlotManager uses**: `tokio::sync::oneshot` (slot_manager.rs:51, line 280)
- **EpochManager uses**: `futures::channel::oneshot` (epoch_manager.rs:109)
- **Fix**: Change SlotManager's `close_rx` to `futures::channel::oneshot::Receiver<futures::channel::oneshot::Sender<()>>`. The `.await` on a futures oneshot receiver works differently — it returns `Result<T, Canceled>` (same as tokio's, so the pattern is the same).

### Impact on SlotManager event loop
The `tokio::select!` macro works with both tokio and futures types via `.fuse()` or by using `futures::StreamExt::next()`. The key change in the event loop:

```rust
// Before (tokio):
msg = message_rx.recv() => { ... }
// After (futures Stream):
msg = message_rx.select_next_some() => { ... }
```

### Impact on SlotManager tests
Tests currently use `tokio::sync::mpsc::unbounded_channel()`. Change to `aptos_channels::new_unbounded_test()` and `futures::channel::oneshot::channel()`. Test assertions using `rx.recv().await` change to `rx.next().await`.

### Impact on SPCSpawner
`SPCSpawner::spawn_spc()` returns `SPCHandles` which contains `spc_output_rx: tokio::sync::mpsc::UnboundedReceiver<SPCOutput>`. The SPC output channel is internal to SlotManager (not crossing the EpochManager boundary), so it can remain tokio. However, for consistency, verify that `tokio::select!` still works when mixing futures streams (message_rx) with tokio receivers (spc_output_rx). It does — `tokio::select!` handles any Future.

## Implementation Steps

### Step 1: Add config flag (`consensus_config.rs`)

- Add `enable_prefix_consensus: bool` to `ConsensusConfig` struct (after line 109)
- Add `#[serde(default)]` on the field (for backwards-compatible deserialization)
- Add `enable_prefix_consensus: false` to `Default` impl
- Note: The struct has `#[serde(default, deny_unknown_fields)]` at struct level. Adding a new field with `#[serde(default)]` means old configs without this field will default to `false`. Configs with unknown extra fields would still fail, but that's the existing behavior for all fields.

### Step 2: Change SlotManager channel types (`slot_manager.rs`)

Change imports:
```rust
// Remove:
use tokio::sync::{mpsc::{UnboundedReceiver, UnboundedSender}, oneshot};
// Add:
use futures::channel::{mpsc::UnboundedSender as FuturesUnboundedSender, oneshot};
// Keep tokio oneshot for SPC internal channels if SPCHandles still uses it
```

Specific changes:
- `execution_channel` field: `tokio::sync::mpsc::UnboundedSender<OrderedBlocks>` → `FuturesUnboundedSender<OrderedBlocks>`
- `execution_channel.send(ordered)` (line 607) → `execution_channel.unbounded_send(ordered)`
- `start()` signature: `message_rx: UnboundedReceiver<...>` → `message_rx: aptos_channels::UnboundedReceiver<...>`
- `start()` signature: `close_rx: oneshot::Receiver<oneshot::Sender<()>>` → `close_rx: futures::channel::oneshot::Receiver<futures::channel::oneshot::Sender<()>>`
- Event loop: `message_rx.recv()` → `message_rx.select_next_some()` (using `futures::StreamExt`)
- Close signal: pattern stays the same (both oneshot types produce `Result<T, Canceled>`)
- Shutdown ack: `ack_tx.send(())` — same API for both oneshot implementations

Update tests to use `aptos_channels::new_unbounded_test()` and `futures::channel::oneshot::channel()`.

### Step 3: Add SlotManager channel fields to EpochManager (`epoch_manager.rs`)

Add after the existing strong prefix consensus fields (line ~197):
```rust
// Slot Manager (Multi-Slot Prefix Consensus) channels
slot_manager_tx: Option<aptos_channels::UnboundedSender<(Author, aptos_prefix_consensus::SlotConsensusMsg)>>,
slot_manager_close_tx: Option<oneshot::Sender<oneshot::Sender<()>>>,
```

Note: `oneshot` here is `futures::channel::oneshot` (already imported at epoch_manager.rs:109).

Initialize both to `None` in `EpochManager::new()` (line ~274-277).

### Step 4: Add `start_new_epoch_with_slot_manager()` method

Follow DAG startup pattern (`start_new_epoch_with_dag()`, lines 1453-1554).

**Signature** — same as DAG (all params needed for `execution_client.start_epoch()`):
```rust
async fn start_new_epoch_with_slot_manager(
    &mut self,
    epoch_state: Arc<EpochState>,
    loaded_consensus_key: Arc<PrivateKey>,
    onchain_consensus_config: OnChainConsensusConfig,
    on_chain_execution_config: OnChainExecutionConfig,
    onchain_randomness_config: OnChainRandomnessConfig,
    onchain_jwk_consensus_config: OnChainJWKConsensusConfig,
    network_sender: NetworkSender,
    payload_client: Arc<dyn PayloadClient>,
    payload_manager: Arc<dyn TPayloadManager>,
    rand_config: Option<RandConfig>,
    fast_rand_config: Option<RandConfig>,
    rand_msg_rx: aptos_channel::Receiver<AccountAddress, IncomingRandGenRequest>,
    secret_share_msg_rx: aptos_channel::Receiver<AccountAddress, IncomingSecretShareRequest>,
)
```

**Construction sequence** (following DAG order):

1. `let signer = Arc::new(ValidatorSigner::new(self.author, loaded_consensus_key.clone()));`
2. `let commit_signer = Arc::new(DagCommitSigner::new(signer.clone()));` (reuse from `consensus/src/dag/commit_signer.rs`)
3. `let highest_committed_round = self.storage.aptos_db().get_latest_ledger_info().expect("...").commit_info().round();`
4. Call `self.execution_client.start_epoch(loaded_consensus_key, epoch_state.clone(), commit_signer, payload_manager.clone(), &onchain_consensus_config, &on_chain_execution_config, &onchain_randomness_config, rand_config, fast_rand_config, rand_msg_rx, secret_share_msg_rx, highest_committed_round).await;`
5. `let execution_channel = self.execution_client.get_execution_channel().expect("execution channel must exist after start_epoch");`
6. `let parent_block_info = self.storage.aptos_db().get_latest_ledger_info().expect("...").commit_info().clone();`
7. Create slot manager channels:
   - `let (slot_tx, slot_rx) = aptos_channels::new_unbounded(&counters::OP_COUNTERS.gauge("slot_manager_channel_msgs"));`
   - `let (close_tx, close_rx) = futures::channel::oneshot::channel();`
8. Create network bridge:
   - `let bridge = SlotConsensusNetworkBridge::new(self.network_sender.clone());`
   - `let slot_network_client = aptos_prefix_consensus::SlotConsensusNetworkClient::new(bridge);`
   - `let slot_network_sender = aptos_prefix_consensus::SlotNetworkSenderAdapter::new(self.author, slot_network_client, slot_tx.clone(), epoch_state.verifier.clone());`
9. Create SPC spawner:
   - `let spc_spawner = RealSPCSpawner::new(self.author, epoch_state.epoch, loaded_consensus_key, epoch_state.verifier.clone(), self.network_sender.clone());`
   - Note: `RealSPCSpawner::new` takes `consensus_network_client: ConsensusNetworkClient<NC>` — that's `self.network_sender` (epoch_manager.rs:145), NOT the `NetworkSender` parameter.
10. Create ranking manager: `let ranking_manager = MultiSlotRankingManager::new(epoch_state.verifier.get_ordered_account_addresses());`
11. Create SlotManager:
    ```rust
    let slot_manager = SlotManager::new(
        self.author,
        epoch_state.epoch,
        ValidatorSigner::new(self.author, loaded_consensus_key),
        epoch_state.verifier.clone(),
        ranking_manager,
        execution_channel,
        payload_client,
        parent_block_info,
        slot_network_sender,
        spc_spawner,
    );
    ```
12. Store channels and spawn: `self.slot_manager_tx = Some(slot_tx);`, `self.slot_manager_close_tx = Some(close_tx);`, `tokio::spawn(slot_manager.start(slot_rx, close_rx));`

**Starting slot**: SlotManager::start() hardcodes `self.start_new_slot(1)`. For the prototype this is acceptable — on restart, state sync catches the node up and slot 1 is correct for a fresh epoch. Mid-epoch crash recovery is deferred (same as the existing design decision #6 in multi-slot-consensus.md).

### Step 5: Wire routing in `start_new_epoch()` (line 1323)

Change:
```rust
if consensus_config.is_dag_enabled() {
    self.start_new_epoch_with_dag(...).await
} else {
    self.start_new_epoch_with_jolteon(...).await
}
```

To:
```rust
if self.config.enable_prefix_consensus {
    self.start_new_epoch_with_slot_manager(...).await
} else if consensus_config.is_dag_enabled() {
    self.start_new_epoch_with_dag(...).await
} else {
    self.start_new_epoch_with_jolteon(...).await
}
```

Local config flag is checked **first**, before on-chain config. This matches multi-slot-consensus.md design decision #7.

### Step 6: Add `SlotConsensusMsg` routing in `check_epoch()` (before line ~1769 `_ =>`)

```rust
ConsensusMsg::SlotConsensusMsg(msg) => {
    let msg_epoch = msg.epoch();
    if msg_epoch == self.epoch() {
        if let Some(tx) = &mut self.slot_manager_tx {
            tx.send((peer_id, *msg)).await.map_err(|e| {
                anyhow::anyhow!("Failed to send to slot_manager_tx: {:?}", e)
            })?;
        } else {
            warn!(
                remote_peer = peer_id,
                epoch = msg_epoch,
                "[EpochManager] Received SlotConsensusMsg but slot manager not running"
            );
        }
    } else {
        warn!(
            remote_peer = peer_id,
            msg_epoch = msg_epoch,
            local_epoch = self.epoch(),
            "[EpochManager] SlotConsensusMsg epoch mismatch"
        );
    }
},
```

Note: `tx.send(...).await` is correct — `aptos_channels::UnboundedSender` implements `Sink` (from `futures::SinkExt`), so `.send()` is async. This matches the existing PC/SPC routing pattern at lines 1727, 1750.

Note: `ConsensusMsg::SlotConsensusMsg` variant already exists in `network_interface.rs:113` — no changes needed there. Network routing in `consensus/src/network.rs` already handles SlotConsensusMsg in the DirectSend dispatch.

### Step 7: Add shutdown handling in `shutdown_current_processor()` (after line ~712)

```rust
// Shutdown slot manager (multi-slot prefix consensus)
if let Some(close_tx) = self.slot_manager_close_tx.take() {
    let (ack_tx, ack_rx) = futures::channel::oneshot::channel();
    if close_tx.send(ack_tx).is_err() {
        warn!("[EpochManager] Slot manager already stopped");
    } else {
        if tokio::time::timeout(Duration::from_secs(5), ack_rx).await.is_err() {
            warn!("[EpochManager] Timeout waiting for slot manager shutdown");
        }
    }
}
self.slot_manager_tx = None;
```

Note: SlotManager's shutdown also tears down any running SPC task via `spc_close_tx` in its own shutdown handler (slot_manager.rs event loop close_rx branch). So the EpochManager only needs to signal SlotManager — the SPC cleanup cascades.

### Step 8: Imports

**epoch_manager.rs**:
```rust
use aptos_prefix_consensus::{
    SlotConsensusMsg, MultiSlotRankingManager,
    SlotConsensusNetworkClient, SlotNetworkSenderAdapter,
};
use crate::dag::commit_signer::DagCommitSigner;
use crate::network_interface::SlotConsensusNetworkBridge;
use crate::prefix_consensus::slot_manager::{SlotManager, RealSPCSpawner};
```

**slot_manager.rs**: Replace tokio channel imports with futures equivalents (detailed in Step 2).

## Resolved Decisions

1. **commit_signer**: Reuse `DagCommitSigner` (from `consensus/src/dag/commit_signer.rs`). Simple key signing, no SafetyRules needed.

2. **Recovery**: Skip `RecoveryData` / `PartialRecoveryData`. Use `self.storage.aptos_db().get_latest_ledger_info()` directly (same as DAG). No recovery manager fallback.

3. **Channel types**: Adapt SlotManager to use `futures::channel` types (matching execution pipeline and codebase convention), not the other way around.

## Verification

```bash
# 1. Compile check (both crates, since slot_manager.rs channel types change)
cargo check -p aptos-prefix-consensus
cargo check -p aptos-consensus

# 2. Unit tests (slot manager tests must pass with new channel types)
cargo test -p aptos-prefix-consensus
cargo test -p aptos-consensus -- slot_manager

# 3. Config test (ensure serde works with new field)
cargo test -p aptos-config

# 4. Existing tests unaffected (enable_prefix_consensus defaults to false)
cargo test -p aptos-consensus -- epoch_manager
```
