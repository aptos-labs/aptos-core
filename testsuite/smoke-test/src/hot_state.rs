// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Smoke tests for the hot state feature.
//!
//! `HOTNESS_IN_EPILOGUE` (hot state promotion in the block epilogue) is kept on
//! in every test. `TRANSACTION_INFO_V1` commits the hot state root to the ledger
//! accumulator; when it is on, a divergent hot state is rejected during commit
//! and during state sync chunk verification. That is what turns these tests into
//! real assertions rather than smoke -- catching up at all implies matching roots.
//!
//! Fast sync and backup/restore do not support hot state yet, so the syncing
//! tests only exercise apply-outputs and re-execution, never
//! `DownloadLatestStates`. Hot state is always persisted across restarts here
//! (`delete_on_restart = false`), so a restarting node reloads it from disk.

use crate::{
    smoke_test_environment::SwarmBuilder,
    state_sync_utils,
    utils::{
        create_test_accounts, execute_transactions, execute_transactions_and_wait,
        get_on_chain_resource, transfer_coins, wait_for_all_nodes, MAX_CATCH_UP_WAIT_SECS,
    },
};
use aptos_config::config::{BootstrappingMode, ContinuousSyncingMode, NodeConfig};
use aptos_forge::{NodeExt, Swarm};
use aptos_genesis::builder::InitGenesisConfigFn;
use aptos_logger::info;
use aptos_types::on_chain_config::{FeatureFlag, Features};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

/// Keep persisted hot state across restarts, so a restarting node reloads it from
/// disk rather than starting empty.
fn persist_hot_state(config: &mut NodeConfig) {
    config.storage.hot_state_config.delete_on_restart = false;
}

/// Genesis features for the suite: `HOTNESS_IN_EPILOGUE` is always on;
/// `TRANSACTION_INFO_V1` is optionally on from the start.
fn hot_state_genesis(enable_txn_info_v1: bool) -> InitGenesisConfigFn {
    Arc::new(move |genesis_config| {
        let mut features = Features::default();
        features.enable(FeatureFlag::HOTNESS_IN_EPILOGUE);
        if enable_txn_info_v1 {
            features.enable(FeatureFlag::TRANSACTION_INFO_V1);
        } else {
            features.disable(FeatureFlag::TRANSACTION_INFO_V1);
        }
        genesis_config.initial_features_override = Some(features);
    })
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

    let validator_peer_id = swarm.validators().next().unwrap().peer_id();
    let validator_client = swarm.validator(validator_peer_id).unwrap().rest_client();

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

/// A fullnode bootstraps by re-executing every transaction from genesis and must
/// rebuild hot state from the block epilogues. `TRANSACTION_INFO_V1` is on from
/// genesis, so the recomputed hot state root is verified against the ledger.
#[tokio::test]
async fn test_hot_state_fullnode_execution_sync() {
    let mut swarm = SwarmBuilder::new_local(1)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| persist_hot_state(config)))
        .with_init_genesis_config(hot_state_genesis(true))
        .build()
        .await;

    let mut vfn_config = NodeConfig::get_default_vfn_config();
    persist_hot_state(&mut vfn_config);
    vfn_config.state_sync.state_sync_driver.bootstrapping_mode =
        BootstrappingMode::ExecuteTransactionsFromGenesis;
    vfn_config
        .state_sync
        .state_sync_driver
        .continuous_syncing_mode = ContinuousSyncingMode::ExecuteTransactions;

    // Wipe and re-sync the fullnode, re-executing from genesis.
    let vfn_peer_id = state_sync_utils::create_fullnode(vfn_config, &mut swarm).await;
    state_sync_utils::test_fullnode_sync(vfn_peer_id, &mut swarm, true, true).await;
}

/// A fullnode syncs by applying transaction outputs while `TRANSACTION_INFO_V1`
/// is turned on mid-chain via governance. The fullnode is restarted twice: once
/// preserving storage (reload hot state from disk) and once after a wipe
/// (bootstrap from genesis), so it crosses the V0 -> V1 boundary while rebuilding
/// hot state.
#[tokio::test]
async fn test_hot_state_fullnode_output_sync_enable_txn_info_v1() {
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

    // Owned handles, so nodes can be restarted afterwards. Load runs between user
    // accounts to avoid contending on the root sequence number with the
    // governance script below (which submits as root via a separate tracker).
    let validator_peer_id = swarm.validators().next().unwrap().peer_id();
    let validator_client = swarm.validator(validator_peer_id).unwrap().rest_client();
    let transaction_factory = swarm.chain_info().transaction_factory();
    let (mut account_0, mut account_1) = create_test_accounts(&mut swarm).await;

    let features = get_on_chain_resource::<Features>(&validator_client).await;
    assert!(
        !features.is_enabled(FeatureFlag::TRANSACTION_INFO_V1),
        "TRANSACTION_INFO_V1 should start disabled"
    );

    // Phase 1: build hot state while transaction infos are still V0.
    info!("Generating pre-V1 load.");
    for _ in 0..10 {
        transfer_coins(
            &validator_client,
            &transaction_factory,
            &mut account_0,
            &account_1,
            1,
        )
        .await;
    }
    wait_for_all_nodes(&mut swarm).await;

    // Enable TRANSACTION_INFO_V1 via governance; it applies at the next epoch.
    info!("Enabling TRANSACTION_INFO_V1 via governance.");
    let root_addr = swarm.chain_info().root_account().address();
    let root_idx = cli.add_account_with_address_to_cli(swarm.root_key(), root_addr);
    let script = format!(
        r#"
script {{
    use aptos_framework::aptos_governance;
    use aptos_framework::features;

    fun main(core_resources: &signer) {{
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);
        features::change_feature_flags_for_next_epoch(&framework_signer, vector[{}], vector[]);
        aptos_governance::reconfigure(&framework_signer);
    }}
}}
"#,
        FeatureFlag::TRANSACTION_INFO_V1 as u64
    );
    cli.run_script(root_idx, &script)
        .await
        .expect("Failed to enable TRANSACTION_INFO_V1 via governance.");

    // Wait for the feature to take effect on chain (applied at the next epoch).
    let deadline = Instant::now() + Duration::from_secs(MAX_CATCH_UP_WAIT_SECS);
    loop {
        let features = get_on_chain_resource::<Features>(&validator_client).await;
        if features.is_enabled(FeatureFlag::TRANSACTION_INFO_V1) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "TRANSACTION_INFO_V1 was not enabled in time"
        );
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    info!("TRANSACTION_INFO_V1 is now enabled on chain.");

    // Phase 2: more load, now committing the hot state root via TransactionInfoV1.
    for _ in 0..10 {
        transfer_coins(
            &validator_client,
            &transaction_factory,
            &mut account_1,
            &account_0,
            1,
        )
        .await;
    }
    wait_for_all_nodes(&mut swarm).await;

    // Restart preserving storage: the fullnode reloads hot state from disk.
    info!("Restarting fullnode (storage preserved).");
    swarm
        .fullnode_mut(vfn_peer_id)
        .unwrap()
        .restart()
        .await
        .unwrap();
    wait_for_all_nodes(&mut swarm).await;

    // Wipe and re-bootstrap from genesis, crossing the V0 -> V1 boundary.
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
    for _ in 0..5 {
        transfer_coins(
            &validator_client,
            &transaction_factory,
            &mut account_0,
            &account_1,
            1,
        )
        .await;
    }
    wait_for_all_nodes(&mut swarm).await;
}
