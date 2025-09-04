// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    smoke_test_environment::SwarmBuilder,
    utils::{
        add_node_to_seeds, assert_balance, create_and_fund_account, transfer_coins,
        MAX_CATCH_UP_WAIT_SECS, MAX_CONNECTIVITY_WAIT_SECS, MAX_HEALTHY_WAIT_SECS,
    },
};
use velor_config::{
    config::{DiscoveryMethod, NodeConfig, OverrideNodeConfig, PeerRole},
    network_id::NetworkId,
};
use velor_forge::{LocalSwarm, NodeExt, Swarm, SwarmExt};
use std::time::{Duration, Instant};

#[tokio::test]
async fn test_full_node_basic_flow() {
    let mut swarm = local_swarm_with_fullnodes(1, 1).await;
    let validator_peer_id = swarm.validators().next().unwrap().peer_id();
    let vfn_peer_id = swarm.full_nodes().next().unwrap().peer_id();
    let version = swarm.versions().max().unwrap();
    let pfn_peer_id = swarm
        .add_full_node(
            &version,
            OverrideNodeConfig::new_with_default_base(NodeConfig::get_default_pfn_config()),
        )
        .await
        .unwrap();
    for fullnode in swarm.full_nodes() {
        fullnode
            .wait_until_healthy(Instant::now() + Duration::from_secs(MAX_HEALTHY_WAIT_SECS))
            .await
            .unwrap();
    }
    let transaction_factory = swarm.chain_info().transaction_factory();

    // create clients for all nodes
    let validator_client = swarm.validator(validator_peer_id).unwrap().rest_client();
    let vfn_client = swarm.full_node(vfn_peer_id).unwrap().rest_client();
    let pfn_client = swarm.full_node(pfn_peer_id).unwrap().rest_client();

    let mut account_0 = create_and_fund_account(&mut swarm, 10).await;
    let account_1 = create_and_fund_account(&mut swarm, 10).await;

    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_CATCH_UP_WAIT_SECS))
        .await
        .unwrap();

    // Send txn to PFN
    let _txn = transfer_coins(
        &pfn_client,
        &transaction_factory,
        &mut account_0,
        &account_1,
        1,
    )
    .await;

    assert_balance(&validator_client, &account_0, 9).await;
    assert_balance(&validator_client, &account_1, 11).await;
    assert_balance(&vfn_client, &account_0, 9).await;
    assert_balance(&vfn_client, &account_1, 11).await;
    assert_balance(&pfn_client, &account_0, 9).await;
    assert_balance(&pfn_client, &account_1, 11).await;

    // Send txn to VFN
    let txn = transfer_coins(
        &vfn_client,
        &transaction_factory,
        &mut account_0,
        &account_1,
        1,
    )
    .await;

    assert_balance(&validator_client, &account_0, 8).await;
    assert_balance(&validator_client, &account_1, 12).await;
    assert_balance(&vfn_client, &account_0, 8).await;
    assert_balance(&vfn_client, &account_1, 12).await;

    pfn_client.wait_for_signed_transaction(&txn).await.unwrap();
    assert_balance(&pfn_client, &account_0, 8).await;
    assert_balance(&pfn_client, &account_1, 12).await;

    // Send txn to Validator
    let txn = transfer_coins(
        &vfn_client,
        &transaction_factory,
        &mut account_0,
        &account_1,
        1,
    )
    .await;

    assert_balance(&validator_client, &account_0, 7).await;
    assert_balance(&validator_client, &account_1, 13).await;

    vfn_client.wait_for_signed_transaction(&txn).await.unwrap();
    assert_balance(&vfn_client, &account_0, 7).await;
    assert_balance(&vfn_client, &account_1, 13).await;

    pfn_client.wait_for_signed_transaction(&txn).await.unwrap();
    assert_balance(&pfn_client, &account_0, 7).await;
    assert_balance(&pfn_client, &account_1, 13).await;
}

#[tokio::test]
async fn test_vfn_failover() {
    // VFN failover happens when validator is down even for default_failovers = 0
    let mut vfn_config = NodeConfig::get_default_vfn_config();
    vfn_config.mempool.default_failovers = 0;
    let mut swarm = SwarmBuilder::new_local(4)
        .with_num_fullnodes(4)
        .with_velor()
        .with_vfn_config(vfn_config)
        .build()
        .await;
    let transaction_factory = swarm.chain_info().transaction_factory();

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

    // Setup accounts
    let mut account_0 = create_and_fund_account(&mut swarm, 10).await;
    let account_1 = create_and_fund_account(&mut swarm, 10).await;

    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_CATCH_UP_WAIT_SECS))
        .await
        .unwrap();

    // set up client
    let validator_peer_ids = swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>();
    let vfn_peer_ids = swarm.full_nodes().map(|v| v.peer_id()).collect::<Vec<_>>();
    let validator = validator_peer_ids[1];
    let vfn_client = swarm.full_node(vfn_peer_ids[1]).unwrap().rest_client();

    // submit client requests directly to VFN of dead V
    swarm.validator_mut(validator).unwrap().stop();

    transfer_coins(
        &vfn_client,
        &transaction_factory,
        &mut account_0,
        &account_1,
        1,
    )
    .await;

    assert_balance(&vfn_client, &account_0, 9).await;
    assert_balance(&vfn_client, &account_1, 11).await;

    transfer_coins(
        &vfn_client,
        &transaction_factory,
        &mut account_0,
        &account_1,
        1,
    )
    .await;

    assert_balance(&vfn_client, &account_0, 8).await;
    assert_balance(&vfn_client, &account_1, 12).await;
}

#[tokio::test]
async fn test_private_full_node() {
    let mut swarm = local_swarm_with_fullnodes(4, 1).await;
    let vfn_peer_id = swarm.full_nodes().next().unwrap().peer_id();

    let transaction_factory = swarm.chain_info().transaction_factory();

    // Here we want to add two swarms, a private full node, followed by a user full node connected to it
    let mut private_config = NodeConfig::get_default_pfn_config();
    let private_network = private_config.full_node_networks.first_mut().unwrap();
    // Disallow public connections
    private_network.max_inbound_connections = 0;
    // Also, we only want it to purposely connect to 1 VFN
    private_network.max_outbound_connections = 1;

    let mut user_config = NodeConfig::get_default_pfn_config();
    let user_network = user_config.full_node_networks.first_mut().unwrap();
    // Disallow fallbacks to VFNs
    user_network.max_outbound_connections = 1;
    user_network.discovery_method = DiscoveryMethod::None;

    // The secret sauce, add the user as a downstream to the seeds
    add_node_to_seeds(
        &mut private_config,
        &user_config,
        NetworkId::Public,
        PeerRole::Downstream,
    );

    // Now we need to connect the VFNs to the private swarm
    let version = swarm.versions().max().unwrap();
    add_node_to_seeds(
        &mut private_config,
        swarm.fullnode(vfn_peer_id).unwrap().config(),
        NetworkId::Public,
        PeerRole::PreferredUpstream,
    );
    let private = swarm
        .add_full_node(
            &version,
            OverrideNodeConfig::new_with_default_base(private_config),
        )
        .await
        .unwrap();

    // And connect the user to the private swarm
    add_node_to_seeds(
        &mut user_config,
        swarm.full_node(private).unwrap().config(),
        NetworkId::Public,
        PeerRole::PreferredUpstream,
    );
    let user = swarm
        .add_full_node(
            &version,
            OverrideNodeConfig::new_with_default_base(user_config),
        )
        .await
        .unwrap();

    swarm
        .wait_for_connectivity(Instant::now() + Duration::from_secs(MAX_CONNECTIVITY_WAIT_SECS))
        .await
        .unwrap();

    // Ensure that User node is connected to private node and only the private node
    {
        let user_node = swarm.full_node(user).unwrap();
        assert_eq!(
            1,
            user_node
                .get_connected_peers(NetworkId::Public, None)
                .await
                .unwrap()
                .unwrap_or(0),
            "User node is connected to more than one peer"
        );
    }

    // read state from full node client
    let validator_client = swarm.validators().next().unwrap().rest_client();
    let user_client = swarm.full_node(user).unwrap().rest_client();

    let mut account_0 = create_and_fund_account(&mut swarm, 100).await;
    let account_1 = create_and_fund_account(&mut swarm, 10).await;

    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_CATCH_UP_WAIT_SECS))
        .await
        .unwrap();

    // send txn from user node and check both validator and user node have correct balance
    transfer_coins(
        &user_client,
        &transaction_factory,
        &mut account_0,
        &account_1,
        10,
    )
    .await;
    assert_balance(&user_client, &account_0, 90).await;
    assert_balance(&user_client, &account_1, 20).await;
    assert_balance(&validator_client, &account_0, 90).await;
    assert_balance(&validator_client, &account_1, 20).await;
}

async fn local_swarm_with_fullnodes(num_validators: usize, num_fullnodes: usize) -> LocalSwarm {
    SwarmBuilder::new_local(num_validators)
        .with_num_fullnodes(num_fullnodes)
        .with_velor()
        .build()
        .await
}
