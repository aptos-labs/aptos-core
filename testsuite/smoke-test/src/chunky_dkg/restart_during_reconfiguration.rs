// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{smoke_test_environment::SwarmBuilder, utils::get_on_chain_resource};
use aptos_config::config::{OverrideNodeConfig, PersistableConfig};
use aptos_forge::{NodeExt, SwarmExt};
use aptos_rest_client::Client;
use aptos_types::{
    dkg::{chunky_dkg::ChunkyDKGState, DKGState},
    on_chain_config::{
        FeatureFlag, Features, OnChainChunkyDKGConfig, OnChainConfig, OnChainRandomnessConfig,
    },
    randomness::PerBlockRandomness,
};
use move_core_types::language_storage::CORE_CODE_ADDRESS;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

async fn get_on_chain_resource_at_version<T: OnChainConfig>(client: &Client, version: u64) -> T {
    client
        .get_account_resource_at_version_bcs::<T>(
            CORE_CODE_ADDRESS,
            &T::struct_tag().to_canonical_string(),
            version,
        )
        .await
        .unwrap()
        .into_inner()
}

/// Ordinary DKG can overwrite its last completed session while Chunky DKG
/// still blocks the epoch transition. Restarted validators must keep producing
/// the current epoch's randomness, including when a quorum restarts.
#[tokio::test]
async fn validator_restart_after_randomness_dkg_completion() {
    let mut swarm = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            config.api.failpoints_enabled = true;
            config.consensus.quorum_store.enable_batch_v2_tx = true;
            config.consensus.quorum_store.enable_batch_v2_rx = true;
            config.consensus.quorum_store.enable_opt_qs_v2_payload_tx = true;
            config.consensus.quorum_store.enable_opt_qs_v2_payload_rx = true;
            config
                .state_sync
                .state_sync_driver
                .enable_auto_bootstrapping = true;
            config
                .state_sync
                .state_sync_driver
                .max_connection_deadline_secs = 3;
        }))
        .with_init_genesis_config(Arc::new(|config| {
            config.epoch_duration_secs = 30;
            config.consensus_config.enable_validator_txns();
            // Require randomness in every block so the regression cannot be
            // hidden by a workload with no randomness-using transactions.
            config.consensus_config.disable_rand_check();
            config.randomness_config_override = Some(OnChainRandomnessConfig::default_enabled());
            config.chunky_dkg_config_override = Some(OnChainChunkyDKGConfig::default_enabled());
            let mut features = Features::default();
            features.enable(FeatureFlag::ENCRYPTED_TRANSACTIONS);
            config.initial_features_override = Some(features);
        }))
        .build()
        .await;

    swarm
        // The first Chunky DKG includes expensive key generation on debug builds.
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(600))
        .await
        .unwrap();
    let clients = swarm
        .validators()
        .map(|node| node.rest_client())
        .collect::<Vec<_>>();
    let failpoint = "chunky_dkg::process_dkg_start_event";
    for client in &clients {
        client
            .set_failpoint(failpoint.into(), "return".into())
            .await
            .unwrap();
    }

    let deadline = Instant::now() + Duration::from_secs(120);
    let (stalled_epoch, version_before_restart) = loop {
        assert!(
            Instant::now() < deadline,
            "Timed out waiting for ordinary DKG to finish while Chunky DKG is pending"
        );
        let ledger = clients[0]
            .get_ledger_information()
            .await
            .unwrap()
            .into_inner();
        let ordinary =
            get_on_chain_resource_at_version::<DKGState>(&clients[0], ledger.version).await;
        let chunky =
            get_on_chain_resource_at_version::<ChunkyDKGState>(&clients[0], ledger.version).await;
        if ordinary.in_progress.is_none()
            && ordinary
                .last_completed
                .as_ref()
                .is_some_and(|s| s.metadata.dealer_epoch == ledger.epoch)
            && chunky
                .in_progress
                .as_ref()
                .is_some_and(|s| s.metadata.dealer_epoch == ledger.epoch)
        {
            break (ledger.epoch, ledger.version);
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    };

    // Persist the failpoint across restart so recovery cannot hide the bug by
    // completing Chunky DKG and advancing to the next epoch.
    for node in swarm.validators_mut().take(3) {
        node.stop();
        let path = node.config_path();
        let mut config = OverrideNodeConfig::load_config(path.clone()).unwrap();
        config
            .override_config_mut()
            .failpoints
            .get_or_insert_with(Default::default)
            .insert(failpoint.into(), "return".into());
        config.save_config(path).unwrap();
        node.start().unwrap();
    }

    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(180))
        .await
        .unwrap();
    swarm
        .liveness_check(Instant::now() + Duration::from_secs(180))
        .await
        .unwrap();
    let ledger = clients[0]
        .get_ledger_information()
        .await
        .unwrap()
        .into_inner();
    assert_eq!(ledger.epoch, stalled_epoch);
    assert!(ledger.version > version_before_restart);
    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(60))
        .await
        .unwrap();
    for client in &clients {
        let randomness =
            get_on_chain_resource_at_version::<PerBlockRandomness>(client, ledger.version).await;
        assert_eq!(randomness.epoch, stalled_epoch);
        assert!(
            randomness.seed.is_some(),
            "Restarted validators lost current-epoch randomness"
        );
        let state = get_on_chain_resource::<DKGState>(client).await;
        assert_eq!(
            state.last_completed.unwrap().metadata.dealer_epoch,
            stalled_epoch
        );
    }
}
