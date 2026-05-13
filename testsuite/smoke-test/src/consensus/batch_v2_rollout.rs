// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    consensus::helpers::generate_traffic_and_assert_committed, smoke_test_environment::SwarmBuilder,
};
use aptos_forge::{NodeExt, SwarmExt};
use aptos_logger::info;
use std::time::Duration;

const MAX_WAIT_SECS: u64 = 60;

/// Tests the rollout of batch_v2 and opt_qs_v2_payload TX capabilities.
///
/// All validators start with RX enabled (the default). Then, one by one, each
/// validator is restarted with the TX counterpart turned on. After each restart,
/// traffic is generated to confirm the network continues to make progress with
/// the mixed configuration. The `quorum_store_created_batch_count` metric with
/// `batch_version=v2` is checked to verify that v2 batches are being created
/// by the upgraded validators.
#[tokio::test]
async fn test_batch_v2_tx_rollout() {
    let num_validators = 4;

    let mut swarm = SwarmBuilder::new_local(num_validators)
        .with_aptos()
        .build()
        .await;
    let validator_peer_ids = swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>();

    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_WAIT_SECS))
        .await
        .unwrap();

    // Phase 1: All validators have only RX enabled (default). Verify progress.
    info!("Phase 1: all validators with batch_v2_rx and opt_qs_v2_payload_rx only");
    generate_traffic_and_assert_committed(&mut swarm, &validator_peer_ids, Duration::from_secs(10))
        .await;

    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_WAIT_SECS))
        .await
        .unwrap();

    // Phase 2: Roll out TX one validator at a time.
    for (i, peer_id) in validator_peer_ids.iter().enumerate() {
        info!(
            "Phase 2.{}: enabling batch_v2_tx and opt_qs_v2_payload_tx on validator {}",
            i, i
        );

        let val = swarm.validator_mut(*peer_id).unwrap();
        val.stop();
        val.config_mut().consensus.quorum_store.enable_batch_v2_tx = true;
        val.config_mut()
            .consensus
            .quorum_store
            .enable_opt_qs_v2_payload_tx = true;
        let config_path = val.config_path();
        val.config_mut().save_to_path(&config_path).unwrap();
        val.start().unwrap();

        val.wait_until_healthy(
            std::time::Instant::now()
                .checked_add(Duration::from_secs(MAX_WAIT_SECS))
                .unwrap(),
        )
        .await
        .unwrap();

        swarm
            .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_WAIT_SECS))
            .await
            .unwrap();

        // Send traffic through the restarted validator to ensure it creates batches.
        generate_traffic_and_assert_committed(&mut swarm, &[*peer_id], Duration::from_secs(15))
            .await;

        // Verify the restarted validator is creating v2 batches by checking
        // the quorum_store_created_batch_count metric with batch_version=v2.
        let v2_fields =
            std::collections::HashMap::from([("batch_version".to_string(), "v2".to_string())]);
        let v2_count = swarm
            .validator(*peer_id)
            .unwrap()
            .get_metric_with_fields_i64("quorum_store_created_batch_count", v2_fields)
            .await
            .unwrap();
        info!("Validator {} v2 batch count: {:?}", i, v2_count);
        assert!(
            v2_count.unwrap_or(0) > 0,
            "Validator {} should have created v2 batches after enabling batch_v2_tx",
            i,
        );

        swarm
            .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_WAIT_SECS))
            .await
            .unwrap();
    }
}
