// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    smoke_test_environment::SwarmBuilder,
    utils::{get_current_consensus_config, get_current_version},
};
use velor_forge::{NodeExt, SwarmExt};
use velor_logger::{debug, info};
use velor_rest_client::Client;
use velor_types::on_chain_config::OnChainRandomnessConfig;
use futures::future::join_all;
use std::{sync::Arc, time::Duration};

/// Chain should not be blocked by failing validator txns.
/// TODO: reimplement dummy vtxn and reenable this.
#[ignore]
#[tokio::test]
async fn dummy_validator_txns() {
    let swarm = SwarmBuilder::new_local(4)
        .with_init_config(Arc::new(|_, config, _| {
            config.api.failpoints_enabled = true;
        }))
        .with_init_genesis_config(Arc::new(move |conf| {
            // start with randomness enabled.
            conf.consensus_config.enable_validator_txns();
            conf.randomness_config_override = Some(OnChainRandomnessConfig::default_enabled());
        }))
        .with_velor()
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
    let velor_results = join_all(tasks).await;
    println!("velor_results={:?}", velor_results);

    let version_milestone_0 = get_current_version(&validator_clients[0]).await;
    let version_milestone_1 = version_milestone_0 + 10;
    info!("Current version: {}, the chain should tolerate potentially invalid vtxns and survive until version {}.", version_milestone_0, version_milestone_1);
    swarm
        .wait_for_all_nodes_to_catchup_to_version(version_milestone_1, Duration::from_secs(60))
        .await
        .expect("milestone 1 taking too long");
}
