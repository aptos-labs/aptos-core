// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{smoke_test_environment::SwarmBuilder, utils::get_current_version};
use aptos_forge::{NodeExt, SwarmExt};
use std::{sync::Arc, time::Duration};

#[tokio::test]
async fn fallback_test() {
    let swarm = SwarmBuilder::new_local(1)
        .with_init_config(Arc::new(|_, config, _| {
            config.api.failpoints_enabled = true;
            config.execution.discard_failed_blocks = true;
        }))
        .with_aptos()
        .build()
        .await;

    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(60))
        .await
        .expect("Epoch 2 taking too long to come!");

    let client = swarm.validators().next().unwrap().rest_client();

    client
        .set_failpoint(
            "aptos_vm::vm_wrapper::execute_transaction".to_string(),
            "100%return".to_string(),
        )
        .await
        .unwrap();

    for _i in 0..1 {
        let version_milestone_0 = get_current_version(&client).await;
        let version_milestone_1 = version_milestone_0 + 5;
        println!("Current version: {}, the chain should tolerate discarding failed blocks, waiting for {}.", version_milestone_0, version_milestone_1);
        swarm
            .wait_for_all_nodes_to_catchup_to_version(version_milestone_1, Duration::from_secs(30))
            .await
            .expect("milestone 1 taking too long");
    }
}
