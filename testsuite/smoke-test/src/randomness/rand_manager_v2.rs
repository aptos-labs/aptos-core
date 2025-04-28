// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::smoke_test_environment::SwarmBuilder;
use aptos_forge::{LocalSwarm, Node, NodeExt, Swarm, SwarmExt};
use aptos_logger::{debug, info};
use aptos_types::on_chain_config::{FeatureFlag, Features, OnChainRandomnessConfig};
use std::{sync::Arc, time::Duration};

#[tokio::test]
async fn rand_manager_v2() {
    let num_validators = 4;
    let (mut swarm, mut cli, _) = SwarmBuilder::new_local(num_validators)
        .with_num_fullnodes(1)
        .with_aptos()
        .with_init_genesis_config(Arc::new(|conf| {
            conf.epoch_duration_secs = 30;

            // Ensure randomness is enabled.
            conf.consensus_config.enable_validator_txns();
            conf.randomness_config_override = Some(OnChainRandomnessConfig::default_enabled());

            let mut features = Features::default();
            features.enable(FeatureFlag::RAND_MANAGER_V2);
            conf.initial_features_override = Some(features);
        }))
        .build_with_cli(0)
        .await;

    let root_addr = swarm.chain_info().root_account().address();
    let root_idx = cli.add_account_with_address_to_cli(swarm.root_key(), root_addr);

    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(3, Duration::from_secs(40))
        .await
        .unwrap();

    info!("3-out-of-4-nodes migration in epoch 3 should be fine while v2 is enabled.");
    migrate_3_nodes(&mut swarm).await;
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(4, Duration::from_secs(40))
        .await
        .unwrap();

    info!("Disabling v2 in epoch 4.");
    let script = r#"
script {
    use aptos_framework::aptos_governance;
    use std::features;
    fun main(core_resources: &signer) {
        let framework = aptos_governance::get_signer_testnet_only(core_resources, @0x1);
        features::change_feature_flags_for_next_epoch(&framework, vector[], vector[features::get_rand_manager_v2_feature()]);
        aptos_governance::reconfigure(&framework);
    }
}
"#;

    let txn_summary = cli.run_script(root_idx, script).await.unwrap();
    debug!("txn_summary={:?}", txn_summary);

    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(5, Duration::from_secs(40))
        .await
        .unwrap();

    info!("3-out-of-4-nodes migration in epoch 5 should break liveness while v2 is disabled. (Epoch 6 will never come.)");
    migrate_3_nodes(&mut swarm).await;
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(6, Duration::from_secs(60))
        .await
        .err()
        .unwrap();
}

async fn migrate_3_nodes(swarm: &mut LocalSwarm) {
    for (node_idx, node) in swarm.validators_mut().enumerate().take(3) {
        node.stop();
        node.clear_storage().await.unwrap();
        info!("node {} stopped and lost storage", node_idx);
    }

    for (node_idx, node) in swarm.validators_mut().enumerate().take(2) {
        node.restart().await.unwrap();
        info!("node {} restarted", node_idx);
    }
}
