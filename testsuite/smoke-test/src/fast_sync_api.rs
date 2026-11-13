// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::smoke_test_environment::SwarmBuilder;
use aptos_config::config::{BootstrappingMode, NodeConfig, OverrideNodeConfig};
use aptos_forge::Swarm;
use std::time::Duration;

/// Verifies what a node still fast syncing serves from its API.
///
/// A node in this state is alive but has no ledger data, and the two are
/// reported differently on purpose: k8s uses `/-/healthy` as a liveness probe
/// (see terraform/helm/fullnode), so failing it while the node catches up would
/// restart the pod and make a long fast sync impossible to finish.
#[tokio::test]
async fn test_fast_syncing_node_api() {
    // Create a swarm with a single validator
    let mut swarm = SwarmBuilder::new_local(1).with_aptos().build().await;
    let validator_peer_id = swarm.validators().next().unwrap().peer_id();

    // Add a fast syncing fullnode, then stop the validator it would sync from
    // so that it stays unbootstrapped for the duration of the test
    let mut vfn_config = NodeConfig::get_default_vfn_config();
    vfn_config.state_sync.state_sync_driver.bootstrapping_mode =
        BootstrappingMode::DownloadLatestStates;
    let vfn_peer_id = swarm
        .add_validator_fullnode(
            &swarm.versions().max().unwrap(),
            OverrideNodeConfig::new_with_default_base(vfn_config),
            validator_peer_id,
        )
        .unwrap();
    swarm.validator_mut(validator_peer_id).unwrap().stop();

    // Wait for the fullnode's API to come up. It binds the port before it has
    // anything to serve, which is what the k8s startup probe (a TCP check)
    // relies on.
    let vfn = swarm.full_node(vfn_peer_id).unwrap();
    let api_url = vfn.rest_api_endpoint();
    let client = reqwest::Client::new();
    let healthy_url = api_url.join("/v1/-/healthy").unwrap();

    let mut liveness_status = None;
    for _ in 0..60 {
        if let Ok(response) = client.get(healthy_url.clone()).send().await {
            liveness_status = Some(response.status());
            break;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    // Liveness passes: the node is running, it just has nothing yet
    assert_eq!(
        liveness_status,
        Some(reqwest::StatusCode::OK),
        "A fast syncing node must pass its liveness check!"
    );

    // Readiness fails: the node has not caught up
    let readiness = client
        .get(api_url.join("/v1/-/healthy?duration_secs=30").unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(
        readiness.status(),
        reqwest::StatusCode::SERVICE_UNAVAILABLE,
        "A fast syncing node must fail its readiness check!"
    );

    // And the rest of the API says why, rather than reporting an internal error
    let index = client
        .get(api_url.join("/v1/").unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(index.status(), reqwest::StatusCode::SERVICE_UNAVAILABLE);
    let body = index.text().await.unwrap();
    assert!(
        body.contains("node_not_bootstrapped"),
        "Expected a node_not_bootstrapped error code, got: {}",
        body
    );
}
