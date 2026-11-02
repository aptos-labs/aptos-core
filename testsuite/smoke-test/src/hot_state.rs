// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Smoke tests for the hot state features.
//!
//! `HOTNESS_IN_EPILOGUE` (hot-state promotion in the block epilogue) is on in
//! every test. `TRANSACTION_INFO_V1` together with `HOT_STATE_ROOT_IN_TXN_INFO`
//! commits the hot state root to the ledger accumulator, so once they are enabled,
//! a node catching up at all implies its recomputed hot state roots matched -- that
//! is the verification oracle these tests lean on. `delete_on_restart` is false
//! everywhere so restarts reload the persisted hot state rather than wiping it.
//!
//! Fast sync restores the hot state snapshot only when the target commits a hot
//! state root, so the fast sync tests always target an epoch where both features
//! are on. Backup/restore is intentionally excluded since hot state does not
//! support it yet.

use crate::{
    smoke_test_environment::SwarmBuilder,
    state_sync::shorten_proposer_history,
    state_sync_utils,
    utils::{
        create_test_accounts, execute_transactions, execute_transactions_and_wait,
        first_validator_client, get_on_chain_resource, transfer_coins, wait_for_all_nodes,
        MAX_CATCH_UP_WAIT_SECS,
    },
};
use aptos::test::CliTestFramework;
use aptos_config::config::{BootstrappingMode, ContinuousSyncingMode, NodeConfig};
use aptos_forge::{LocalNode, LocalSwarm, Node, NodeExt, Swarm};
use aptos_genesis::builder::InitGenesisConfigFn;
use aptos_logger::info;
use aptos_rest_client::Client as RestClient;
use aptos_sdk::{transaction_builder::TransactionFactory, types::LocalAccount};
use aptos_types::on_chain_config::{FeatureFlag, Features};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

/// Small enough that both the main and the hot state snapshots take several chunks.
const SMALL_STATE_CHUNK_SIZE: u64 = 30;

/// Small enough that the load fills every shard and evicts, while the full hot
/// state (`NUM_STATE_SHARDS` times this) still takes several chunks.
const TINY_HOT_STATE_ITEMS_PER_SHARD: usize = 8;

fn persist_hot_state(config: &mut NodeConfig) {
    config.storage.hot_state_config.delete_on_restart = false;
}

fn use_fast_sync(config: &mut NodeConfig) {
    config.state_sync.state_sync_driver.bootstrapping_mode =
        BootstrappingMode::DownloadLatestStates;
}

/// Config for a node that serves snapshots to fast syncing peers.
fn snapshot_server_config(config: &mut NodeConfig) {
    persist_hot_state(config);
    config.state_sync.storage_service.max_state_chunk_size = SMALL_STATE_CHUNK_SIZE;
}

/// Shrinks the hot state so that items get evicted and refreshed every few blocks,
/// making later hot state roots depend on the LRU order and `hot_since_version`s.
/// These feed into the hot state root, so every node needs the same values.
fn use_tiny_hot_state(config: &mut NodeConfig) {
    config.storage.hot_state_config.max_items_per_shard = TINY_HOT_STATE_ITEMS_PER_SHARD;
    config.storage.hot_state_config.refresh_interval_versions = 10;
}

/// Asserts that the node restored a hot state snapshot of more than one chunk
/// through fast sync.
async fn verify_hot_state_fast_synced(node: &LocalNode) {
    let last_index = node
        .inspection_client()
        .get_node_metric_i64("aptos_state_sync_version{type=synced_hot_states}")
        .await
        .unwrap();
    assert!(
        last_index.is_some_and(|index| index >= SMALL_STATE_CHUNK_SIZE as i64),
        "{} did not restore a multi-chunk hot state snapshot, last index: {:?}",
        node.name(),
        last_index,
    );
}

/// Genesis features for the suite: `HOTNESS_IN_EPILOGUE` is always on;
/// `TRANSACTION_INFO_V1` plus `HOT_STATE_ROOT_IN_TXN_INFO` (which commits the hot
/// state root to the ledger accumulator) is optionally on from the start.
fn hot_state_genesis(enable_txn_info_v1: bool) -> InitGenesisConfigFn {
    Arc::new(move |genesis_config| {
        let mut features = Features::default();
        features.enable(FeatureFlag::HOTNESS_IN_EPILOGUE);
        if enable_txn_info_v1 {
            features.enable(FeatureFlag::TRANSACTION_INFO_V1);
            features.enable(FeatureFlag::HOT_STATE_ROOT_IN_TXN_INFO);
        } else {
            features.disable(FeatureFlag::TRANSACTION_INFO_V1);
            features.disable(FeatureFlag::HOT_STATE_ROOT_IN_TXN_INFO);
        }
        genesis_config.initial_features_override = Some(features);
    })
}

/// Submits `count` one-coin transfers from `sender` to `receiver`, waiting on each.
async fn generate_load(
    client: &RestClient,
    transaction_factory: &TransactionFactory,
    sender: &mut LocalAccount,
    receiver: &LocalAccount,
    count: usize,
) {
    for _ in 0..count {
        transfer_coins(client, transaction_factory, sender, receiver, 1).await;
    }
}

/// Enables `TRANSACTION_INFO_V1` and `HOT_STATE_ROOT_IN_TXN_INFO` through a governance
/// script submitted as root and waits for them to take effect at the next epoch. Both
/// are needed for the hot state root to be committed to the ledger accumulator.
async fn enable_txn_info_v1_via_governance(
    swarm: &mut LocalSwarm,
    cli: &mut CliTestFramework,
    validator_client: &RestClient,
) {
    info!("Enabling TRANSACTION_INFO_V1 + HOT_STATE_ROOT_IN_TXN_INFO via governance.");
    let root_addr = swarm.chain_info().root_account().address();
    let root_idx = cli.add_account_with_address_to_cli(swarm.root_key(), root_addr);
    let script = format!(
        r#"
script {{
    use aptos_framework::aptos_governance;

    fun main(core_resources: &signer) {{
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);
        aptos_governance::toggle_features(&framework_signer, vector[{}, {}], vector[]);
    }}
}}
"#,
        FeatureFlag::TRANSACTION_INFO_V1 as u64,
        FeatureFlag::HOT_STATE_ROOT_IN_TXN_INFO as u64,
    );
    cli.run_script(root_idx, &script)
        .await
        .expect("Failed to enable TRANSACTION_INFO_V1 via governance.");

    // The features apply at the next epoch; wait for them to land on chain.
    let deadline = Instant::now() + Duration::from_secs(MAX_CATCH_UP_WAIT_SECS);
    loop {
        let features = get_on_chain_resource::<Features>(validator_client).await;
        if features.is_enabled(FeatureFlag::TRANSACTION_INFO_V1)
            && features.is_enabled(FeatureFlag::HOT_STATE_ROOT_IN_TXN_INFO)
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "TRANSACTION_INFO_V1 was not enabled in time"
        );
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    info!("TRANSACTION_INFO_V1 + HOT_STATE_ROOT_IN_TXN_INFO are now enabled on chain.");
}

/// A fullnode rebuilds hot state from genesis using the given sync modes, with
/// `TRANSACTION_INFO_V1` on from genesis so the recomputed root is verified
/// against the ledger. Wipes and re-syncs a freshly attached VFN.
async fn run_hot_state_fullnode_sync(
    bootstrapping_mode: BootstrappingMode,
    continuous_syncing_mode: ContinuousSyncingMode,
) {
    let mut swarm = SwarmBuilder::new_local(1)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| persist_hot_state(config)))
        .with_init_genesis_config(hot_state_genesis(true))
        .build()
        .await;

    let mut vfn_config = NodeConfig::get_default_vfn_config();
    persist_hot_state(&mut vfn_config);
    vfn_config.state_sync.state_sync_driver.bootstrapping_mode = bootstrapping_mode;
    vfn_config
        .state_sync
        .state_sync_driver
        .continuous_syncing_mode = continuous_syncing_mode;

    let vfn_peer_id = state_sync_utils::create_fullnode(vfn_config, &mut swarm).await;
    state_sync_utils::test_fullnode_sync(vfn_peer_id, &mut swarm, true, true).await;
}

/// A validator restarts without wiping storage: it must reload hot state from
/// disk and rejoin without diverging. `TRANSACTION_INFO_V1` is on from genesis,
/// so a wrong reload would yield a mismatched hot state root and stall consensus.
#[tokio::test]
async fn test_hot_state_validator_restart() {
    // Four validators so the chain keeps progressing while one is down.
    let mut swarm = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| persist_hot_state(config)))
        .with_init_genesis_config(hot_state_genesis(true))
        .build()
        .await;

    let validator_client = first_validator_client(&swarm);

    // Build up some hot state.
    let (mut account_0, mut account_1) = create_test_accounts(&mut swarm).await;
    execute_transactions(
        &mut swarm,
        &validator_client,
        &mut account_0,
        &account_1,
        true,
    )
    .await;
    wait_for_all_nodes(&mut swarm).await;

    // Restart a different validator, preserving its storage.
    let restart_peer_id = swarm.validators().nth(1).unwrap().peer_id();
    swarm
        .validator_mut(restart_peer_id)
        .unwrap()
        .restart()
        .await
        .unwrap();
    wait_for_all_nodes(&mut swarm).await;

    // Keep producing and confirm everyone stays in sync.
    execute_transactions_and_wait(
        &mut swarm,
        &validator_client,
        &mut account_1,
        &account_0,
        true,
    )
    .await;
}

/// A fullnode bootstraps by re-executing every transaction from genesis, rebuilding
/// hot state from the block epilogues. `TRANSACTION_INFO_V1` is on from genesis.
#[tokio::test]
async fn test_hot_state_fullnode_execution_sync() {
    run_hot_state_fullnode_sync(
        BootstrappingMode::ExecuteTransactionsFromGenesis,
        ContinuousSyncingMode::ExecuteTransactions,
    )
    .await;
}

/// A fullnode bootstraps by applying transaction outputs from genesis, rebuilding
/// hot state from the recorded outputs. `TRANSACTION_INFO_V1` is on from genesis.
#[tokio::test]
async fn test_hot_state_fullnode_output_sync() {
    run_hot_state_fullnode_sync(
        BootstrappingMode::ApplyTransactionOutputsFromGenesis,
        ContinuousSyncingMode::ApplyTransactionOutputs,
    )
    .await;
}

/// A fullnode is restarted on both sides of the `TRANSACTION_INFO_V1` switch,
/// always preserving storage, so it reloads hot state from disk under V0 and again
/// under V1. The post-enable V1 root check verifies the reloaded-then-promoted
/// state.
#[tokio::test]
async fn test_hot_state_fullnode_restart_across_v1_boundary() {
    // Single validator + CLI for the governance script. V1 starts disabled.
    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(1)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| persist_hot_state(config)))
        .with_init_genesis_config(hot_state_genesis(false))
        .build_with_cli(0)
        .await;

    // Fullnode that reloads hot state from disk on restart (default sync mode).
    let mut vfn_config = NodeConfig::get_default_vfn_config();
    persist_hot_state(&mut vfn_config);
    let vfn_peer_id = state_sync_utils::create_fullnode(vfn_config, &mut swarm).await;

    let validator_client = first_validator_client(&swarm);
    let transaction_factory = swarm.chain_info().transaction_factory();
    // Load runs between user accounts so it doesn't contend on the root sequence
    // number with the governance script (which submits as root via the CLI).
    let (mut account_0, mut account_1) = create_test_accounts(&mut swarm).await;

    // Phase 1: build hot state under V0, then restart and reload it.
    info!("Generating pre-V1 load.");
    generate_load(
        &validator_client,
        &transaction_factory,
        &mut account_0,
        &account_1,
        10,
    )
    .await;
    // Force a reconfig so the hot state above is snapshotted to disk; otherwise the
    // restart below reloads an empty hot state and just replays write sets.
    aptos_forge::reconfig(
        &validator_client,
        &transaction_factory,
        swarm.chain_info().root_account,
    )
    .await;
    wait_for_all_nodes(&mut swarm).await;

    info!("Restarting fullnode before V1 (storage preserved).");
    swarm
        .fullnode_mut(vfn_peer_id)
        .unwrap()
        .restart()
        .await
        .unwrap();
    wait_for_all_nodes(&mut swarm).await;

    enable_txn_info_v1_via_governance(&mut swarm, &mut cli, &validator_client).await;

    // Phase 2: more load, now committing the hot state root via TransactionInfoV1.
    generate_load(
        &validator_client,
        &transaction_factory,
        &mut account_1,
        &account_0,
        10,
    )
    .await;
    wait_for_all_nodes(&mut swarm).await;

    info!("Restarting fullnode after V1 (storage preserved).");
    swarm
        .fullnode_mut(vfn_peer_id)
        .unwrap()
        .restart()
        .await
        .unwrap();
    wait_for_all_nodes(&mut swarm).await;

    // Final load to confirm the fullnode stays in sync.
    generate_load(
        &validator_client,
        &transaction_factory,
        &mut account_0,
        &account_1,
        5,
    )
    .await;
    wait_for_all_nodes(&mut swarm).await;
}

/// A fullnode bootstraps by applying transaction outputs across the V0 -> V1
/// boundary: load is generated under V0, `TRANSACTION_INFO_V1` is enabled via
/// governance, more load is generated under V1, then the fullnode is wiped and
/// re-bootstraps from genesis -- replaying both transaction-info formats while
/// rebuilding hot state.
#[tokio::test]
async fn test_hot_state_fullnode_sync_across_v1_boundary() {
    // Single validator + CLI for the governance script. V1 starts disabled.
    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(1)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| persist_hot_state(config)))
        .with_init_genesis_config(hot_state_genesis(false))
        .build_with_cli(0)
        .await;

    // Fullnode that syncs by applying transaction outputs.
    let mut vfn_config = NodeConfig::get_default_vfn_config();
    persist_hot_state(&mut vfn_config);
    vfn_config.state_sync.state_sync_driver.bootstrapping_mode =
        BootstrappingMode::ApplyTransactionOutputsFromGenesis;
    vfn_config
        .state_sync
        .state_sync_driver
        .continuous_syncing_mode = ContinuousSyncingMode::ApplyTransactionOutputs;
    let vfn_peer_id = state_sync_utils::create_fullnode(vfn_config, &mut swarm).await;

    let validator_client = first_validator_client(&swarm);
    let transaction_factory = swarm.chain_info().transaction_factory();
    // Load runs between user accounts so it doesn't contend on the root sequence
    // number with the governance script (which submits as root via the CLI).
    let (mut account_0, mut account_1) = create_test_accounts(&mut swarm).await;

    // Phase 1: build hot state while transaction infos are still V0.
    info!("Generating pre-V1 load.");
    generate_load(
        &validator_client,
        &transaction_factory,
        &mut account_0,
        &account_1,
        10,
    )
    .await;
    wait_for_all_nodes(&mut swarm).await;

    enable_txn_info_v1_via_governance(&mut swarm, &mut cli, &validator_client).await;

    // Phase 2: more load, now committing the hot state root via TransactionInfoV1.
    generate_load(
        &validator_client,
        &transaction_factory,
        &mut account_1,
        &account_0,
        10,
    )
    .await;
    wait_for_all_nodes(&mut swarm).await;

    // Wipe and re-bootstrap from genesis, replaying across the V0 -> V1 boundary.
    info!("Wiping fullnode storage and re-bootstrapping via apply-outputs.");
    state_sync_utils::stop_fullnode_and_delete_storage(&mut swarm, vfn_peer_id, true).await;
    swarm
        .fullnode_mut(vfn_peer_id)
        .unwrap()
        .restart()
        .await
        .unwrap();
    wait_for_all_nodes(&mut swarm).await;

    // Final load to confirm the fullnode stays in sync.
    generate_load(
        &validator_client,
        &transaction_factory,
        &mut account_0,
        &account_1,
        5,
    )
    .await;
    wait_for_all_nodes(&mut swarm).await;
}

/// A fresh fullnode fast syncs to a later epoch, then restarts without wiping
/// storage. No epoch ends in between, so its latest persisted snapshot is still the
/// fast sync target: it reloads the restored hot state from disk and replays the
/// write sets after it. The hot state is tiny, so later blocks evict and refresh
/// restored items, and the roots also depend on the restored LRU order and
/// `hot_since_version`s.
#[tokio::test]
async fn test_hot_state_fullnode_fast_sync() {
    let mut swarm = SwarmBuilder::new_local(1)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            snapshot_server_config(config);
            use_tiny_hot_state(config);
        }))
        .with_init_genesis_config(hot_state_genesis(true))
        .build()
        .await;

    // Build up hot state across a few epochs before the fullnode joins.
    let validator_client = first_validator_client(&swarm);
    let transaction_factory = swarm.chain_info().transaction_factory();
    let (mut account_0, mut account_1) = create_test_accounts(&mut swarm).await;
    execute_transactions(
        &mut swarm,
        &validator_client,
        &mut account_0,
        &account_1,
        true,
    )
    .await;

    let mut vfn_config = NodeConfig::get_default_vfn_config();
    snapshot_server_config(&mut vfn_config);
    use_tiny_hot_state(&mut vfn_config);
    use_fast_sync(&mut vfn_config);
    let vfn_peer_id = state_sync_utils::create_fullnode(vfn_config, &mut swarm).await;
    wait_for_all_nodes(&mut swarm).await;
    verify_hot_state_fast_synced(swarm.fullnode(vfn_peer_id).unwrap()).await;

    generate_load(
        &validator_client,
        &transaction_factory,
        &mut account_1,
        &account_0,
        10,
    )
    .await;
    wait_for_all_nodes(&mut swarm).await;

    info!("Restarting fullnode after fast sync (storage preserved).");
    swarm
        .fullnode_mut(vfn_peer_id)
        .unwrap()
        .restart()
        .await
        .unwrap();
    wait_for_all_nodes(&mut swarm).await;

    execute_transactions_and_wait(
        &mut swarm,
        &validator_client,
        &mut account_0,
        &account_1,
        true,
    )
    .await;

    // The counter restarts with the process, so it only counts evictions on top of
    // the reloaded hot state. It's thread local and flushed lazily, hence the polling.
    let fullnode = swarm.fullnode(vfn_peer_id).unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let num_evictions = fullnode
            .inspection_client()
            .get_node_metric_i64("aptos_storage_counter{name=hot_state_evict}")
            .await
            .unwrap();
        if num_evictions.is_some_and(|n| n > 0) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "The fullnode evicted no hot state items after the restart: {:?}",
            num_evictions,
        );
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

/// A validator is wiped and fast syncs to a later epoch. Another validator is then
/// stopped, so the chain only makes progress if the fast synced validator's votes,
/// which commit to its hot state roots, match the others.
#[tokio::test]
async fn test_hot_state_validator_fast_sync_and_participate() {
    let mut swarm = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| snapshot_server_config(config)))
        .with_init_genesis_config(Arc::new(|genesis_config| {
            hot_state_genesis(true)(genesis_config);
            shorten_proposer_history(genesis_config);
        }))
        .build()
        .await;

    let validator_peer_ids: Vec<_> = swarm.validators().map(|v| v.peer_id()).collect();
    let validator_client = first_validator_client(&swarm);
    let (mut account_0, mut account_1) = create_test_accounts(&mut swarm).await;
    execute_transactions_and_wait(
        &mut swarm,
        &validator_client,
        &mut account_0,
        &account_1,
        true,
    )
    .await;

    // Only switch the wiped validator to fast sync, so the others never fast sync to
    // genesis at startup.
    let fast_sync_peer_id = validator_peer_ids[1];
    let validator = swarm.validator_mut(fast_sync_peer_id).unwrap();
    validator.clear_storage().await.unwrap();
    use_fast_sync(validator.config_mut());
    let config_path = validator.config_path();
    validator.config_mut().save_to_path(&config_path).unwrap();

    execute_transactions(
        &mut swarm,
        &validator_client,
        &mut account_1,
        &account_0,
        true,
    )
    .await;

    swarm
        .validator_mut(fast_sync_peer_id)
        .unwrap()
        .start()
        .unwrap();
    wait_for_all_nodes(&mut swarm).await;
    verify_hot_state_fast_synced(swarm.validator(fast_sync_peer_id).unwrap()).await;

    // Leader reputation looks at the previous epoch, which the fast synced validator
    // lacks, so it only agrees on proposers after another epoch change.
    execute_transactions_and_wait(
        &mut swarm,
        &validator_client,
        &mut account_0,
        &account_1,
        true,
    )
    .await;

    swarm.validator_mut(validator_peer_ids[3]).unwrap().stop();
    execute_transactions(
        &mut swarm,
        &validator_client,
        &mut account_0,
        &account_1,
        true,
    )
    .await;
}
