// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    consensus::helpers::generate_traffic_and_assert_committed, smoke_test_environment::SwarmBuilder,
};
use aptos_forge::{wait_for_all_nodes_to_catchup, NodeExt, SwarmExt};
use aptos_rest_client::Client;
use std::{sync::Arc, time::Duration};

const MAX_WAIT_SECS: u64 = 60;

/// This test asserts whether a node that cannot reach either the batch or the block author
/// can fetch batches from the block voters.
#[tokio::test]
async fn test_remote_batch_read_from_block_voters() {
    let mut swarm = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_config(Arc::new(|_, conf, _| {
            conf.api.failpoints_enabled = true;
            conf.consensus.quorum_store.enable_opt_quorum_store = true;
        }))
        .build()
        .await;
    let validator_peer_ids = swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>();

    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_WAIT_SECS))
        .await
        .unwrap();

    let validator_client_0 = swarm
        .validator(validator_peer_ids[0])
        .unwrap()
        .rest_client();
    let validator_client_3 = swarm
        .validator(validator_peer_ids[3])
        .unwrap()
        .rest_client();

    // Make every node fetch the batch
    validator_client_0
        .set_failpoint(
            "consensus::send::broadcast_self_only".to_string(),
            "return(BatchMsg)".to_string(),
        )
        .await
        .unwrap();

    // Send traffic to Node 0 only
    generate_traffic_and_assert_committed(
        &mut swarm,
        &[validator_peer_ids[0]],
        Duration::from_secs(20),
    )
    .await;

    // Fail batch request for Node 3, so it cannot commit
    validator_client_3
        .set_failpoint(
            "consensus::send::request_batch".to_string(),
            "return".to_string(),
        )
        .await
        .unwrap();

    // Send traffic to Node 0 only
    generate_traffic_and_assert_committed(
        &mut swarm,
        &[validator_peer_ids[0]],
        Duration::from_secs(5),
    )
    .await;

    swarm.validator(validator_peer_ids[0]).unwrap().stop();

    // Enable batch request for Node 3, so it can catch up
    validator_client_3
        .set_failpoint(
            "consensus::send::request_batch".to_string(),
            "off".to_string(),
        )
        .await
        .unwrap();

    let honest_peers: Vec<(String, Client)> = swarm
        .validators()
        .skip(1)
        .map(|node| (node.name().to_string(), node.rest_client()))
        .collect();

    wait_for_all_nodes_to_catchup(&honest_peers, Duration::from_secs(MAX_WAIT_SECS))
        .await
        .unwrap();

    swarm
        .validator(validator_peer_ids[0])
        .unwrap()
        .start()
        .unwrap();

    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_WAIT_SECS))
        .await
        .unwrap();
}
