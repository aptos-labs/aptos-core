// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_cached_packages::aptos_stdlib;
use aptos_config::{
    config::{NodeConfig, Peer, PeerRole, HANDSHAKE_VERSION},
    network_id::NetworkId,
};
use aptos_forge::{reconfig, LocalSwarm, NodeExt, Swarm, SwarmExt};
use aptos_rest_client::{Client as RestClient, Client};
use aptos_sdk::{
    transaction_builder::TransactionFactory,
    types::{transaction::SignedTransaction, LocalAccount},
};
use aptos_types::{
    network_address::{NetworkAddress, Protocol},
    on_chain_config::{OnChainConfig, OnChainConsensusConfig, OnChainExecutionConfig},
};
use move_core_types::language_storage::CORE_CODE_ADDRESS;
use rand::random;
use std::{collections::HashSet, net::Ipv4Addr, sync::Arc, time::Duration};

pub const MAX_CATCH_UP_WAIT_SECS: u64 = 180; // The max time we'll wait for nodes to catch up
pub const MAX_CONNECTIVITY_WAIT_SECS: u64 = 180; // The max time we'll wait for nodes to gain connectivity
pub const MAX_HEALTHY_WAIT_SECS: u64 = 120; // The max time we'll wait for nodes to become healthy

pub fn add_node_to_seeds(
    dest_config: &mut NodeConfig,
    seed_config: &NodeConfig,
    network_id: NetworkId,
    peer_role: PeerRole,
) {
    let dest_network_config = dest_config
        .full_node_networks
        .iter_mut()
        .find(|network| network.network_id == network_id)
        .unwrap();
    let seed_network_config = seed_config
        .full_node_networks
        .iter()
        .find(|network| network.network_id == network_id)
        .unwrap();

    let seed_peer_id = seed_network_config.peer_id();
    let seed_key = seed_network_config.identity_key().public_key();

    let seed_peer = if peer_role != PeerRole::Downstream {
        // For upstreams, we know the address, but so don't duplicate the keys in the config (lazy way)
        // TODO: This is ridiculous, we need a better way to manipulate these `NetworkAddress`s
        let address = seed_network_config.listen_address.clone();
        let port_protocol = address
            .as_slice()
            .iter()
            .find(|protocol| matches!(protocol, Protocol::Tcp(_)))
            .unwrap();
        let address = NetworkAddress::from_protocols(vec![
            Protocol::Ip4(Ipv4Addr::new(127, 0, 0, 1)),
            port_protocol.clone(),
            Protocol::NoiseIK(seed_key),
            Protocol::Handshake(HANDSHAKE_VERSION),
        ])
        .unwrap();

        Peer::new(vec![address], HashSet::new(), peer_role)
    } else {
        // For downstreams, we don't know the address, but we know the keys
        let mut seed_keys = HashSet::new();
        seed_keys.insert(seed_key);
        Peer::new(vec![], seed_keys, peer_role)
    };

    dest_network_config.seeds.insert(seed_peer_id, seed_peer);
}

pub async fn create_and_fund_account(swarm: &'_ mut dyn Swarm, amount: u64) -> LocalAccount {
    let mut info = swarm.aptos_public_info();
    info.create_and_fund_user_account(amount).await.unwrap()
}

/// Creates and funds two test accounts
pub async fn create_test_accounts(swarm: &mut LocalSwarm) -> (LocalAccount, LocalAccount) {
    let token_amount = 1000;
    let account_0 = create_and_fund_account(swarm, token_amount).await;
    let account_1 = create_and_fund_account(swarm, token_amount).await;
    (account_0, account_1)
}

/// Executes transactions using the given transaction factory, client and
/// accounts. If `execute_epoch_changes` is true, also execute transactions to
/// force reconfigurations.
pub async fn execute_transactions(
    swarm: &mut LocalSwarm,
    client: &RestClient,
    sender: &mut LocalAccount,
    receiver: &LocalAccount,
    execute_epoch_changes: bool,
) {
    // Execute several transactions
    let num_transfers = 10;
    let transaction_factory = swarm.chain_info().transaction_factory();
    if execute_epoch_changes {
        transfer_and_maybe_reconfig(
            client,
            &transaction_factory,
            swarm.chain_info().root_account,
            sender,
            receiver,
            num_transfers,
        )
        .await;
    } else {
        for _ in 0..num_transfers {
            // Execute simple transfer transactions
            transfer_coins(client, &transaction_factory, sender, receiver, 1).await;
        }
    }

    // Always ensure that at least one reconfiguration transaction is executed
    if !execute_epoch_changes {
        aptos_forge::reconfig(
            client,
            &transaction_factory,
            swarm.chain_info().root_account,
        )
        .await;
    }
}

/// Executes transactions and waits for all nodes to catch up
pub async fn execute_transactions_and_wait(
    swarm: &mut LocalSwarm,
    client: &RestClient,
    sender: &mut LocalAccount,
    receiver: &LocalAccount,
    epoch_changes: bool,
) {
    execute_transactions(swarm, client, sender, receiver, epoch_changes).await;
    wait_for_all_nodes(swarm).await;
}

pub async fn transfer_coins_non_blocking(
    client: &RestClient,
    transaction_factory: &TransactionFactory,
    sender: &mut LocalAccount,
    receiver: &LocalAccount,
    amount: u64,
) -> SignedTransaction {
    let txn = sender.sign_with_transaction_builder(transaction_factory.payload(
        aptos_stdlib::aptos_coin_transfer(receiver.address(), amount),
    ));

    client.submit(&txn).await.unwrap();
    txn
}

pub async fn transfer_coins(
    client: &RestClient,
    transaction_factory: &TransactionFactory,
    sender: &mut LocalAccount,
    receiver: &LocalAccount,
    amount: u64,
) -> SignedTransaction {
    let txn =
        transfer_coins_non_blocking(client, transaction_factory, sender, receiver, amount).await;

    client.wait_for_signed_transaction(&txn).await.unwrap();

    txn
}

pub async fn transfer_and_maybe_reconfig(
    client: &RestClient,
    transaction_factory: &TransactionFactory,
    root_account: Arc<LocalAccount>,
    sender: &mut LocalAccount,
    receiver: &LocalAccount,
    num_transfers: usize,
) {
    for _ in 0..num_transfers {
        // Reconfigurations have a 20% chance of being executed
        if random::<u16>() % 5 == 0 {
            reconfig(client, transaction_factory, root_account.clone()).await;
        }

        transfer_coins(client, transaction_factory, sender, receiver, 1).await;
    }
}

pub async fn assert_balance(client: &RestClient, account: &LocalAccount, balance: u64) {
    let on_chain_balance = client
        .get_account_balance(account.address())
        .await
        .unwrap()
        .into_inner();

    assert_eq!(on_chain_balance.get(), balance);
}

/// This helper function creates 3 new accounts, mints funds, transfers funds
/// between the accounts and verifies that these operations succeed.
pub async fn check_create_mint_transfer(swarm: &mut LocalSwarm) {
    check_create_mint_transfer_node(swarm, 0).await;
}

/// This helper function creates 3 new accounts, mints funds, transfers funds
/// between the accounts and verifies that these operations succeed on one specific validator.
pub async fn check_create_mint_transfer_node(swarm: &mut LocalSwarm, idx: usize) {
    let client = swarm.validators().nth(idx).unwrap().rest_client();

    // Create account 0, mint 10 coins and check balance
    let transaction_factory = TransactionFactory::new(swarm.chain_id());
    let mut info = swarm.aptos_public_info_for_node(idx);
    let mut account_0 = info.create_and_fund_user_account(10).await.unwrap();
    assert_balance(&client, &account_0, 10).await;

    // Create account 1, mint 1 coin, transfer 3 coins from account 0 to 1, check balances
    let account_1 = info.create_and_fund_user_account(1).await.unwrap();
    transfer_coins(&client, &transaction_factory, &mut account_0, &account_1, 3).await;

    assert_balance(&client, &account_0, 7).await;
    assert_balance(&client, &account_1, 4).await;

    // Create account 2, mint 15 coins and check balance
    let account_2 = info.create_and_fund_user_account(15).await.unwrap();
    assert_balance(&client, &account_2, 15).await;
}

/// Waits for all nodes to catch up
pub async fn wait_for_all_nodes(swarm: &mut LocalSwarm) {
    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_CATCH_UP_WAIT_SECS))
        .await
        .unwrap();
}

/// Returns the current consensus config
pub async fn get_current_consensus_config(rest_client: &RestClient) -> OnChainConsensusConfig {
    bcs::from_bytes(
        &rest_client
            .get_account_resource_bcs::<Vec<u8>>(
                CORE_CODE_ADDRESS,
                "0x1::consensus_config::ConsensusConfig",
            )
            .await
            .unwrap()
            .into_inner(),
    )
    .unwrap()
}

pub(crate) async fn get_current_execution_config(
    rest_client: &RestClient,
) -> OnChainExecutionConfig {
    bcs::from_bytes(
        &rest_client
            .get_account_resource_bcs::<Vec<u8>>(
                CORE_CODE_ADDRESS,
                "0x1::execution_config::ExecutionConfig",
            )
            .await
            .unwrap()
            .into_inner(),
    )
    .unwrap()
}

/// Returns the current ledger info version
pub async fn get_current_version(rest_client: &RestClient) -> u64 {
    rest_client
        .get_ledger_information()
        .await
        .unwrap()
        .inner()
        .version
}

pub async fn get_on_chain_resource<T: OnChainConfig>(rest_client: &Client) -> T {
    let maybe_response = rest_client
        .get_account_resource_bcs::<T>(CORE_CODE_ADDRESS, T::struct_tag().to_string().as_str())
        .await;
    let response = maybe_response.unwrap();
    response.into_inner()
}

#[cfg(test)]
pub mod swarm_utils {
    use aptos_config::config::{NodeConfig, SecureBackend, WaypointConfig};
    use aptos_secure_storage::{KVStorage, Storage};
    use aptos_types::waypoint::Waypoint;

    pub fn insert_waypoint(node_config: &mut NodeConfig, waypoint: Waypoint) {
        node_config.base.waypoint = WaypointConfig::FromConfig(waypoint);

        let f = |backend: &SecureBackend| {
            let mut storage: Storage = backend.into();
            storage
                .set(aptos_global_constants::WAYPOINT, waypoint)
                .expect("Unable to write waypoint");
            storage
                .set(aptos_global_constants::GENESIS_WAYPOINT, waypoint)
                .expect("Unable to write waypoint");
        };
        let backend = &node_config.consensus.safety_rules.backend;
        f(backend);
    }
}
