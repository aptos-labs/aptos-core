// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::smoke_test_environment::SwarmBuilder;
use aptos_forge::{Swarm, SwarmExt};
use std::time::Duration;

#[tokio::test]
async fn test_pipeline_drains_suffix_across_epoch_changes() {
    // With seven validators a quorum can advance while two nodes are stuck in
    // shutdown. Check progress on every validator after each epoch boundary.
    let swarm = SwarmBuilder::new_local(7).with_aptos().build().await;
    let info = swarm.chain_info().into_aptos_public_info();

    for _ in 0..3 {
        let state = tokio::time::timeout(Duration::from_secs(90), info.reconfig())
            .await
            .expect("Epoch change must finish draining the old pipeline");
        swarm
            .wait_for_all_nodes_to_catchup_to_epoch(state.epoch, Duration::from_secs(60))
            .await
            .expect("Every validator must enter the new epoch");
        // Merely persisting the epoch-ending state is insufficient: the
        // shutdown deadlock leaves that state visible but prevents new blocks.
        swarm
            .wait_for_all_nodes_to_catchup_to_version(state.version + 10, Duration::from_secs(60))
            .await
            .expect("Every validator must commit blocks after the epoch change");
    }
}
