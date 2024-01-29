// Copyright © Aptos Foundation

use crate::smoke_test_environment::SwarmBuilder;
use aptos_forge::{NodeExt, SwarmExt};
use aptos_logger::info;
use std::{sync::Arc, time::Duration};
use aptos_types::on_chain_config::FeatureFlag;
use aptos_vm_genesis::default_features_resource_for_genesis;

#[tokio::test]
async fn foo_bar() {
    let epoch_duration_secs = 30;
    let estimated_dkg_latency_secs = 30;
    let time_limit_secs = epoch_duration_secs + estimated_dkg_latency_secs;
    let num_validators = 4;
    let mut swarm = SwarmBuilder::new_local(num_validators)
        .with_num_fullnodes(1)
        .with_aptos()
        .with_init_genesis_config(Arc::new(|conf| {
            let mut features = default_features_resource_for_genesis();
            features.disable(FeatureFlag::RECONFIGURE_WITH_DKG);
            conf.initial_features_override = Some(features);
        }))
        .build()
        .await;

    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(3, Duration::from_secs(epoch_duration_secs * 10))
        .await
        .unwrap();

    for node in swarm.validators_mut() {
        node.restart().await.unwrap();
    }

    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(5, Duration::from_secs(time_limit_secs))
        .await
        .unwrap();
}
