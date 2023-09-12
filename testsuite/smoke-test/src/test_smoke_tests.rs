// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    smoke_test_environment::SwarmBuilder,
    test_utils::{MAX_CONNECTIVITY_WAIT_SECS, MAX_HEALTHY_WAIT_SECS},
};
use aptos_config::config::{NodeConfig, OverrideNodeConfig};
use aptos_forge::{NodeExt, Swarm};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

/// Bring up a swarm normally, then run get_bin, and bring up a VFN.
/// Previously get_bin triggered a rebuild of aptos-node, which caused issues that were only seen
/// during parallel execution of tests.
/// This test should make regressions obvious.
#[tokio::test]
async fn test_aptos_node_after_get_bin() {
    let mut swarm = SwarmBuilder::new_local(1)
        .with_aptos()
        .with_init_config(Arc::new(|_, conf, _| {
            conf.api.failpoints_enabled = true;
        }))
        .build()
        .await;
    let version = swarm.versions().max().unwrap();
    let validator_peer_ids = swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>();

    // Before #5308 this re-compiled aptos-node and caused a panic on the vfn.
    let _aptos_cli = crate::workspace_builder::get_bin("aptos");

    let validator = validator_peer_ids[0];
    let _vfn = swarm
        .add_validator_fullnode(
            &version,
            OverrideNodeConfig::new_with_default_base(NodeConfig::get_default_vfn_config()),
            validator,
        )
        .unwrap();

    for fullnode in swarm.full_nodes_mut() {
        fullnode
            .wait_until_healthy(Instant::now() + Duration::from_secs(MAX_HEALTHY_WAIT_SECS))
            .await
            .unwrap();
        fullnode
            .wait_for_connectivity(Instant::now() + Duration::from_secs(MAX_CONNECTIVITY_WAIT_SECS))
            .await
            .unwrap();
    }
}
