// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    randomness::{decrypt_key_map, verify_dkg_transcript, wait_for_dkg_finish},
    smoke_test_environment::SwarmBuilder,
    utils::get_on_chain_resource,
};
use velor_forge::{NodeExt, SwarmExt};
use velor_logger::{debug, info};
use velor_rest_client::Client;
use velor_types::{dkg::DKGState, on_chain_config::OnChainRandomnessConfig};
use futures::future::join_all;
use std::{sync::Arc, time::Duration};

#[tokio::test]
async fn validator_restart_during_dkg() {
    let epoch_duration_secs = 30;
    let estimated_dkg_latency_secs = 30;
    let time_limit_secs = epoch_duration_secs + estimated_dkg_latency_secs;
    let num_validators = 4;
    let num_validators_to_restart = 3;
    let mut swarm = SwarmBuilder::new_local(num_validators)
        .with_num_fullnodes(1)
        .with_velor()
        .with_init_config(Arc::new(|_, conf, _| {
            conf.api.failpoints_enabled = true;
        }))
        .with_init_genesis_config(Arc::new(|conf| {
            conf.epoch_duration_secs = 30;

            // Ensure randomness is enabled.
            conf.consensus_config.enable_validator_txns();
            conf.randomness_config_override = Some(OnChainRandomnessConfig::default_enabled());
        }))
        .build()
        .await;

    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(epoch_duration_secs * 10))
        .await
        .unwrap();

    let decrypt_key_map = decrypt_key_map(&swarm);

    info!("Wait for an epoch start.");
    let validator_clients: Vec<Client> =
        swarm.validators().map(|node| node.rest_client()).collect();
    let dkg_session_1 = wait_for_dkg_finish(&validator_clients[3], None, time_limit_secs).await;

    info!(
        "Current epoch is {}.",
        dkg_session_1.metadata.dealer_epoch + 1
    );

    info!("Inject fault to all validators so they get stuck upon the first DKG message received.");
    let tasks = validator_clients
        .iter()
        .take(num_validators_to_restart)
        .map(|client| {
            client.set_failpoint(
                "dkg::process_dkg_start_event".to_string(),
                "panic".to_string(),
            )
        })
        .collect::<Vec<_>>();
    let velor_results = join_all(tasks).await;
    debug!("velor_results={:?}", velor_results);

    info!("Restart nodes after they panic.");
    for (node_idx, node) in swarm
        .validators_mut()
        .enumerate()
        .take(num_validators_to_restart)
    {
        while node.health_check().await.is_ok() {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        info!("node {} panicked", node_idx);
        node.restart().await.unwrap();
        info!("node {} restarted", node_idx);
    }

    info!(
        "DKG should be able to continue. Wait until we fully entered epoch {}.",
        dkg_session_1.target_epoch() + 1
    );

    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(
            dkg_session_1.target_epoch() + 1,
            Duration::from_secs(time_limit_secs),
        )
        .await
        .unwrap();
    let dkg_session_2 = get_on_chain_resource::<DKGState>(&validator_clients[3])
        .await
        .last_completed
        .clone()
        .unwrap();
    assert!(verify_dkg_transcript(&dkg_session_2, &decrypt_key_map).is_ok());
}
