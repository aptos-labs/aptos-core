// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::smoke_test_environment::SwarmBuilder;
use aptos_config::config::NodeConfig;
use forge::{NodeExt, Swarm};
use std::sync::Arc;
use std::time::{Duration, Instant};

const MAX_WAIT_SECS: u64 = 60;

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
            NodeConfig::default_for_validator_full_node(),
            validator,
        )
        .unwrap();

    for fullnode in swarm.full_nodes_mut() {
        fullnode
            .wait_until_healthy(Instant::now() + Duration::from_secs(MAX_WAIT_SECS))
            .await
            .unwrap();
        fullnode
            .wait_for_connectivity(Instant::now() + Duration::from_secs(MAX_WAIT_SECS))
            .await
            .unwrap();
    }
}
