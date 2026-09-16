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
//! Fast sync downloads the hot snapshot directly (see
//! `test_hot_state_fullnode_fast_sync`), rather than rebuilding it by replaying
//! from genesis like the other tests here. Backup/restore is still excluded --
//! hot state does not support it yet.

use crate::{
    smoke_test_environment::SwarmBuilder,
    state_sync_utils,
    utils::{
        create_test_accounts, execute_transactions, execute_transactions_and_wait,
        first_validator_client, get_on_chain_resource, transfer_coins, wait_for_all_nodes,
        MAX_CATCH_UP_WAIT_SECS,
    },
};
use aptos::test::CliTestFramework;
use aptos_config::config::{
    BootstrappingMode, ContinuousSyncingMode, NodeConfig, NO_OP_STORAGE_PRUNER_CONFIG,
};
use aptos_crypto::hash::SPARSE_MERKLE_PLACEHOLDER_HASH;
use aptos_db::AptosDB;
use aptos_forge::{LocalSwarm, NodeExt, Swarm};
use aptos_genesis::builder::InitGenesisConfigFn;
use aptos_logger::info;
use aptos_rest_client::Client as RestClient;
use aptos_sdk::{transaction_builder::TransactionFactory, types::LocalAccount};
use aptos_state_sync_driver::{
    metadata_storage::{MetadataStorageInterface, PersistentMetadataStorage},
    snapshot_kind::SnapshotKind,
};
use aptos_storage_interface::{DbReader, StateKind};
use aptos_types::on_chain_config::{FeatureFlag, Features};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

/// The server-side state chunk limit, kept small so the hot snapshot needs
/// several chunks.
const SERVER_MAX_STATE_CHUNK_SIZE: u64 = 30;

fn persist_hot_state(config: &mut NodeConfig) {
    config.storage.hot_state_config.delete_on_restart = false;
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

/// A fullnode fast syncs to a non-genesis snapshot and downloads the hot state
/// snapshot directly, rather than replaying from genesis.
///
/// Catching up alone cannot prove this: a node that replayed everything would
/// also catch up. So the test pins the outcome from three sides -- the applied
/// hot chunk metrics, the restored database itself, and continued sync through a
/// later checkpoint (which verifies the recomputed hot roots against the ledger,
/// using the restored snapshot as its base).
#[tokio::test]
async fn test_hot_state_fullnode_fast_sync() {
    // Single validator serving small state chunks, so the hot snapshot needs
    // several of them.
    let mut swarm = SwarmBuilder::new_local(1)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            persist_hot_state(config);
            config.state_sync.storage_service.max_state_chunk_size = SERVER_MAX_STATE_CHUNK_SIZE;
        }))
        .with_init_genesis_config(Arc::new(|genesis_config| {
            // Let explicit reconfigurations, not the clock, decide the target
            genesis_config.epoch_duration_secs = 10_000;
            let mut features = Features::default();
            features.enable(FeatureFlag::HOTNESS_IN_EPILOGUE);
            features.enable(FeatureFlag::TRANSACTION_INFO_V1);
            features.enable(FeatureFlag::HOT_STATE_ROOT_IN_TXN_INFO);
            genesis_config.initial_features_override = Some(features);
        }))
        .build()
        .await;

    // Attach a fast-syncing VFN. Consensus observer stays off so the intended
    // state-sync path is the one exercised.
    let mut vfn_config = NodeConfig::get_default_vfn_config();
    persist_hot_state(&mut vfn_config);
    vfn_config.state_sync.state_sync_driver.bootstrapping_mode =
        BootstrappingMode::DownloadLatestStates;
    vfn_config
        .state_sync
        .state_sync_driver
        .continuous_syncing_mode = ContinuousSyncingMode::ApplyTransactionOutputs;
    vfn_config.consensus_observer.observer_enabled = false;
    vfn_config.consensus_observer.publisher_enabled = false;
    let vfn_peer_id = state_sync_utils::create_fullnode(vfn_config, &mut swarm).await;

    let validator_client = first_validator_client(&swarm);
    let transaction_factory = swarm.chain_info().transaction_factory();
    let (mut account_0, mut account_1) = create_test_accounts(&mut swarm).await;

    // Wipe the VFN (including its state-sync progress) so it must fast sync
    info!("Wiping fullnode storage ahead of the fast sync.");
    state_sync_utils::stop_fullnode_and_delete_storage(&mut swarm, vfn_peer_id, true).await;

    // Build hot state while the VFN is down, then force an epoch change so a
    // non-genesis checkpoint with a nonempty committed hot root exists.
    info!("Generating load for the hot snapshot.");
    generate_load(
        &validator_client,
        &transaction_factory,
        &mut account_0,
        &account_1,
        30,
    )
    .await;
    aptos_forge::reconfig(
        &validator_client,
        &transaction_factory,
        swarm.chain_info().root_account,
    )
    .await;
    let target_epoch_version = wait_for_epoch_ending_version(&validator_client).await;
    assert!(
        target_epoch_version > 0,
        "The fast sync target must not be genesis!"
    );

    // Fast sync the VFN in one uninterrupted run
    info!("Restarting the fullnode to fast sync.");
    swarm
        .fullnode_mut(vfn_peer_id)
        .unwrap()
        .restart()
        .await
        .unwrap();
    wait_for_all_nodes(&mut swarm).await;

    // While the node is still up, verify it bootstrapped from a snapshot and
    // applied more than one hot chunk along the way.
    let (hot_chunks_applied, last_hot_index, hot_kv_pruner_min_readable) = {
        let fullnode = swarm.fullnode(vfn_peer_id).unwrap();
        let inspection_client = fullnode.inspection_client();

        // `/forge_metrics` reports a histogram's sample count under the base
        // metric name (without the usual `_count` suffix).
        let hot_chunks_applied = inspection_client
            .get_node_metric_i64(
                "aptos_state_sync_storage_synchronizer_chunk_sizes{label=synced_hot_states}",
            )
            .await
            .unwrap()
            .expect("No hot state chunks were applied!");
        let last_hot_index = inspection_client
            .get_node_metric_i64("aptos_state_sync_version{type=synced_hot_states}")
            .await
            .unwrap()
            .expect("No hot state index was recorded!");
        let hot_kv_pruner_min_readable = inspection_client
            .get_node_metric_i64(
                "aptos_pruner_versions{pruner_name=hot_state_kv_pruner,tag=min_readable}",
            )
            .await
            .unwrap()
            .expect("No hot state KV pruner version was recorded!");

        // The oldest ledger version proves the node bootstrapped from a
        // snapshot rather than replaying from genesis.
        let ledger_information = fullnode
            .rest_client()
            .get_ledger_information()
            .await
            .unwrap();
        assert!(
            ledger_information.inner().oldest_ledger_version > 0,
            "The fullnode replayed from genesis instead of restoring a snapshot!"
        );

        (
            hot_chunks_applied,
            last_hot_index,
            hot_kv_pruner_min_readable,
        )
    };
    assert!(
        hot_chunks_applied > 1,
        "Expected more than one hot state chunk to be applied, but only {} were!",
        hot_chunks_applied
    );
    // A zero-based index: a single-item snapshot would legitimately report 0
    assert!(
        last_hot_index > 0,
        "Expected the restored hot snapshot to hold more than one item!"
    );

    // Submit more load and force another checkpoint. With the hot root committed
    // to the ledger, catching up past the restore target means the VFN's
    // recomputed hot roots matched -- i.e. it built on the restored snapshot.
    info!("Generating post-bootstrap load.");
    generate_load(
        &validator_client,
        &transaction_factory,
        &mut account_1,
        &account_0,
        10,
    )
    .await;
    aptos_forge::reconfig(
        &validator_client,
        &transaction_factory,
        swarm.chain_info().root_account,
    )
    .await;
    wait_for_all_nodes(&mut swarm).await;

    let synced_version = swarm
        .fullnode(vfn_peer_id)
        .unwrap()
        .rest_client()
        .get_ledger_information()
        .await
        .unwrap()
        .inner()
        .version;
    assert!(
        synced_version > target_epoch_version,
        "The fullnode did not catch up beyond the restore target! Synced: {}, target: {}",
        synced_version,
        target_epoch_version
    );

    // Finally, inspect the node's own database and driver metadata. This is read
    // only and uses the node's storage config (in particular
    // `delete_on_restart = false`), so it must not erase the restored hot state.
    info!("Verifying the restored hot snapshot on disk.");
    let node_config = swarm.fullnode(vfn_peer_id).unwrap().config().clone();
    swarm.fullnode_mut(vfn_peer_id).unwrap().stop();
    verify_restored_hot_snapshot(&node_config, hot_kv_pruner_min_readable as u64);
}

/// Opens the node's database read-only and checks the restored hot snapshot
/// against the commitments in the target transaction info.
fn verify_restored_hot_snapshot(node_config: &NodeConfig, hot_kv_pruner_min_readable: u64) {
    // The main and hot stages must have completed at the same non-genesis target
    let metadata_storage = PersistentMetadataStorage::new(node_config.storage.dir());
    let target_ledger_info = metadata_storage
        .previous_snapshot_sync_target(SnapshotKind::MainState)
        .unwrap()
        .expect("No main state snapshot sync was recorded!");
    let target_version = target_ledger_info.ledger_info().version();
    assert!(target_version > 0, "The fast sync target was genesis!");
    for kind in [SnapshotKind::MainState, SnapshotKind::HotState] {
        assert_eq!(
            Some(&target_ledger_info),
            metadata_storage
                .previous_snapshot_sync_target(kind)
                .unwrap()
                .as_ref(),
            "The {} stage targeted a different ledger info!",
            kind.get_label()
        );
        assert!(
            metadata_storage
                .is_snapshot_sync_complete(&target_ledger_info, kind)
                .unwrap(),
            "The {} stage did not complete!",
            kind.get_label()
        );
    }
    drop(metadata_storage);

    // Open the node's DB read-only, honoring its hot state config so the
    // restored hot databases are preserved. Pruning must be off: a read-only
    // open rejects configured prune windows.
    let aptos_db = AptosDB::open(
        node_config.storage.get_dir_paths(),
        true, // readonly
        NO_OP_STORAGE_PRUNER_CONFIG,
        node_config.storage.rocksdb_configs,
        node_config.storage.buffered_state_target_items,
        node_config.storage.max_num_nodes_per_lru_cache_shard,
        None,
        node_config.storage.hot_state_config,
    )
    .unwrap();

    // The target transaction info must commit a real (non-placeholder) hot root
    let target_transaction_info = aptos_db
        .get_transaction_info_iterator(target_version, 1)
        .unwrap()
        .next()
        .expect("No transaction info at the target version!")
        .unwrap();
    let committed_hot_root = target_transaction_info
        .hot_state_checkpoint_hash()
        .expect("The fast sync target committed no hot state root!");
    assert_ne!(
        committed_hot_root, *SPARSE_MERKLE_PLACEHOLDER_HASH,
        "The fast sync target committed an empty hot state root!"
    );

    // The restored hot snapshot must be nonempty and hash to that root
    let hot_item_count = aptos_db.get_hot_state_item_count(target_version).unwrap();
    assert!(
        hot_item_count > 0,
        "The restored hot snapshot at the target is empty!"
    );
    let raw_values = aptos_db
        .get_hot_state_value_chunk_iter(target_version, 0, hot_item_count)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let chunk_with_proof = aptos_db
        .get_hot_state_value_chunk_proof(target_version, 0, raw_values)
        .unwrap();
    assert_eq!(
        chunk_with_proof.root_hash, committed_hot_root,
        "The restored hot snapshot does not match the committed hot state root!"
    );

    // The main state root must also match its commitment at the same target
    let main_chunk_with_proof = aptos_db
        .get_state_value_chunk_with_proof(target_version, 0, 1, StateKind::MainState)
        .unwrap();
    assert_eq!(
        main_chunk_with_proof.root_hash,
        target_transaction_info
            .ensure_state_checkpoint_hash()
            .unwrap(),
        "The restored main state snapshot does not match its committed root!"
    );

    // The persisted summary's hot root must match the commitment at its own
    // version (continuous sync has advanced past the restore target by now).
    let persisted_summary = aptos_db.get_persisted_state_summary().unwrap();
    let summary_version = persisted_summary
        .version()
        .expect("The persisted state summary has no version!");
    let summary_transaction_info = aptos_db
        .get_transaction_info_iterator(summary_version, 1)
        .unwrap()
        .next()
        .expect("No transaction info at the summary version!")
        .unwrap();
    assert_eq!(
        Some(persisted_summary.hot_root_hash().unwrap()),
        summary_transaction_info.hot_state_checkpoint_hash(),
        "The persisted hot state root does not match its commitment at version {}!",
        summary_version
    );

    // With pruning disabled, the hot KV pruner was seeded to the restore target
    assert_eq!(
        hot_kv_pruner_min_readable, target_version,
        "The hot state KV pruner was not seeded to the restore target!"
    );

    // Release the DB handles before the swarm tears the node down
    drop(aptos_db);
}

/// Polls until the validator reports an epoch ending version, returning it.
async fn wait_for_epoch_ending_version(validator_client: &RestClient) -> u64 {
    let deadline = Instant::now() + Duration::from_secs(MAX_CATCH_UP_WAIT_SECS);
    loop {
        let ledger_information = validator_client.get_ledger_information().await.unwrap();
        let version = ledger_information.inner().version;
        if version > 0 && ledger_information.inner().epoch > 1 {
            return version;
        }
        assert!(
            Instant::now() < deadline,
            "The validator did not produce an epoch change in time!"
        );
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
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
