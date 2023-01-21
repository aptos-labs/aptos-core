// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{smoke_test_environment::SwarmBuilder, txn_emitter::generate_traffic};
use aptos_forge::{NodeExt, SwarmExt, TransactionType};
use std::{sync::Arc, time::Duration};

const MAX_WAIT_SECS: u64 = 60;

/// Checks progress even if half the nodes are not actually writing batches to the DB.
/// Shows that remote reading of batches is working.
/// Note this is more than expected (f) byzantine behavior.
#[tokio::test]
async fn test_remote_batch_reads() {
    let mut swarm = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_config(Arc::new(|_, conf, _| {
            conf.api.failpoints_enabled = true;
        }))
        .build()
        .await;
    let validator_peer_ids = swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>();

    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_WAIT_SECS))
        .await
        .unwrap();

    for peer_id in validator_peer_ids.iter().take(2) {
        let validator_client = swarm.validator(*peer_id).unwrap().rest_client();
        validator_client
            .set_failpoint("quorum_store::save".to_string(), "return".to_string())
            .await
            .unwrap();
    }

    let txn_stat = generate_traffic(
        &mut swarm,
        &[validator_peer_ids[2]],
        Duration::from_secs(10),
        1,
        vec![
            (TransactionType::P2P, 70),
            (TransactionType::AccountGeneration, 20),
        ],
    )
    .await
    .unwrap();
    println!("{:?}", txn_stat.rate(Duration::from_secs(10)));
    // assert some much smaller number than expected, so it doesn't fail under contention
    assert!(txn_stat.submitted > 30);
    assert!(txn_stat.committed > 30);

    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_WAIT_SECS))
        .await
        .unwrap();
}
