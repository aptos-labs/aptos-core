// Copyright © Aptos Foundation

use crate::{
    smoke_test_environment::SwarmBuilder,
    utils::{get_current_consensus_config, get_current_version},
};
use aptos_forge::{NodeExt, SwarmExt};
use aptos_logger::{debug, info};
use aptos_rest_client::Client;
use futures::future::join_all;
use std::{sync::Arc, time::Duration};
use aptos_types::on_chain_config::{FeatureFlag, Features};

/// Chain should not be blocked by failing validator txns.
#[tokio::test]
async fn dummy_validator_txns() {
    let swarm = SwarmBuilder::new_local(4)
        .with_init_config(Arc::new(|_, config, _| {
            config.api.failpoints_enabled = true;
        }))
        .with_init_genesis_config(Arc::new(move |conf| {
            // start with vtxn disabled.
            conf.consensus_config.disable_validator_txns();

            // start with dkg enabled.
            let mut features = Features::default();
            features.enable(FeatureFlag::RECONFIGURE_WITH_DKG);
            conf.initial_features_override = Some(features);
        }))
        .with_aptos()
        .build()
        .await;

    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(60))
        .await
        .expect("Epoch 2 taking too long to come!");

    let validator_clients: Vec<Client> =
        swarm.validators().map(|node| node.rest_client()).collect();

    let consensus_config = get_current_consensus_config(&validator_clients[0]).await;
    debug!("consensus_config={:?}", consensus_config);

    info!("Update all validators to start proposing a ValidatorTransaction::Dummy1 in their proposals.");
    let tasks = validator_clients
        .iter()
        .map(|client| {
            client.set_failpoint(
                "mixed_payload_client::extra_test_only_vtxns".to_string(),
                "return".to_string(),
            )
        })
        .collect::<Vec<_>>();
    let aptos_results = join_all(tasks).await;
    println!("aptos_results={:?}", aptos_results);

    let version_milestone_0 = get_current_version(&validator_clients[0]).await;
    let version_milestone_1 = version_milestone_0 + 10;
    info!("Current version: {}, the chain should tolerate potentially invalid vtxns and survive until version {}.", version_milestone_0, version_milestone_1);
    swarm
        .wait_for_all_nodes_to_catchup_to_version(version_milestone_1, Duration::from_secs(60))
        .await
        .expect("milestone 1 taking too long");
}
