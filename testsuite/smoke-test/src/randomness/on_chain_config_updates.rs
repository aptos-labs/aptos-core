// Copyright © Aptos Foundation

use crate::{
    randomness::{decrypt_key_map, get_on_chain_resource, verify_dkg_transcript},
    smoke_test_environment::SwarmBuilder,
    utils::get_current_consensus_config,
};
use aptos_forge::{Node, NodeExt, Swarm, SwarmExt};
use aptos_logger::{debug, info};
use aptos_types::{dkg::DKGState, on_chain_config::FeatureFlag};
use aptos_vm_genesis::default_features_resource_for_genesis;
use std::{sync::Arc, time::Duration};
use futures::future::join_all;
use tokio::time::sleep;
use aptos_rest_client::Client;
use aptos_types::on_chain_config::ConfigurationResource;
use crate::randomness::update_all_on_chain_configs;

/// Enable on-chain randomness in the following steps.
/// - Enable feature `RECONFIGURE_WITH_DKG` in epoch `e`.
/// - Enable validator transactions in consensus config in epoch `e + 1`.
#[tokio::test]
async fn on_chain_config_update_0() {
    let epoch_duration_secs = 20;
    let estimated_dkg_latency_secs = 20;

    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_num_fullnodes(1)
        .with_aptos()
        .with_init_config(Arc::new(|_, conf, _| {
            conf.api.failpoints_enabled = true;
        }))
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;
            conf.allow_new_validators = true;
        }))
        .build_with_cli(0)
        .await;

    let root_addr = swarm.chain_info().root_account().address();
    let _root_idx = cli.add_account_with_address_to_cli(swarm.root_key(), root_addr);

    let decrypt_key_map = decrypt_key_map(&swarm);

    let client_endpoint = swarm.validators().nth(1).unwrap().rest_api_endpoint();
    let client = aptos_rest_client::Client::new(client_endpoint.clone());

    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(3, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Waited too long for epoch 3.");

    let validator_clients: Vec<Client> =
        swarm.validators().map(|node| node.rest_client()).collect();

    info!("Inject fault so next DKG will be stuck.");
    let tasks = validator_clients
        .iter()
        .take(num_validators_to_restart)
        .map(|client| {
            client.set_failpoint(
                "dkg::process_peer_rpc_msg".to_string(),
                "return".to_string(),
            )
        })
        .collect::<Vec<_>>();
    let aptos_results = join_all(tasks).await;
    debug!("aptos_results={:?}", aptos_results);


    info!("DKG for epoch 4 should still be stuck after 40 seconds.");
    tokio::time::sleep(Duration::from_secs(40)).await;
    let epoch = get_on_chain_resource::<ConfigurationResource>(&client).await.epoch();
    assert_eq!(3, epoch);
    let in_progress_dkg_session = get_on_chain_resource::<DKGState>(&client).await.in_progress;
    assert_eq(3, in_progress_dkg_session.unwrap().metadata.dealer_epoch);

    info!("Disable faults.");
    let tasks = validator_clients
        .iter()
        .take(num_validators_to_restart)
        .map(|client| {
            client.set_failpoint(
                "dkg::process_peer_rpc_msg".to_string(),
                "off".to_string(),
            )
        })
        .collect::<Vec<_>>();
    let aptos_results = join_all(tasks).await;
    debug!("aptos_results={:?}", aptos_results);

    info!("Now that DKG is unblocked, epoch 4 should come soon.");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(4, Duration::from_secs(estimated_dkg_latency_secs))
        .await
        .expect("Waited too long for epoch 4.");
    let dkg_session = get_on_chain_resource::<DKGState>(&client)
        .await
        .last_completed
        .expect("dkg result for epoch 6 should be present");
    assert_eq!(4, dkg_session.target_epoch());
    assert!(verify_dkg_transcript(&dkg_session, &decrypt_key_map).is_ok());
}
