// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    smoke_test_environment::SwarmBuilder,
    utils::{MAX_CONNECTIVITY_WAIT_SECS, MAX_HEALTHY_WAIT_SECS},
};
use velor_config::config::{NodeConfig, OverrideNodeConfig};
use velor_forge::{NodeExt, Swarm};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

/// Bring up a swarm normally, then run get_bin, and bring up a VFN.
/// Previously get_bin triggered a rebuild of velor-node, which caused issues that were only seen
/// during parallel execution of tests.
/// This test should make regressions obvious.
#[tokio::test]
async fn test_velor_node_after_get_bin() {
    let mut swarm = SwarmBuilder::new_local(1)
        .with_velor()
        .with_init_config(Arc::new(|_, conf, _| {
            conf.api.failpoints_enabled = true;
        }))
        .build()
        .await;
    let version = swarm.versions().max().unwrap();
    let validator_peer_ids = swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>();

    // Before #5308 this re-compiled velor-node and caused a panic on the vfn.
    let _velor_cli = crate::workspace_builder::get_bin("velor");

    let validator = validator_peer_ids[0];
    let _vfn = swarm
        .add_validator_fullnode(
            &version,
            OverrideNodeConfig::new_with_default_base(NodeConfig::get_default_vfn_config()),
            validator,
        )
        .unwrap();

    for fullnode in swarm.full_nodes() {
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
