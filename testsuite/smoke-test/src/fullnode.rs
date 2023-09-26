// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    smoke_test_environment::new_local_swarm_with_aptos, test_utils::MAX_HEALTHY_WAIT_SECS,
};
use anyhow::bail;
use aptos_cached_packages::aptos_stdlib;
use aptos_config::config::{NodeConfig, OverrideNodeConfig};
use aptos_forge::{NodeExt, Result, Swarm};
use aptos_rest_client::Client as RestClient;
use aptos_types::account_address::AccountAddress;
use std::time::{Duration, Instant};

#[tokio::test]
async fn test_indexer() {
    let mut swarm = new_local_swarm_with_aptos(1).await;

    let version = swarm.versions().max().unwrap();
    let fullnode_peer_id = swarm
        .add_full_node(
            &version,
            OverrideNodeConfig::new_with_default_base(NodeConfig::get_default_pfn_config()),
        )
        .await
        .unwrap();
    let validator_peer_id = swarm.validators().next().unwrap().peer_id();
    let _vfn_peer_id = swarm
        .add_validator_full_node(
            &version,
            OverrideNodeConfig::new_with_default_base(NodeConfig::get_default_vfn_config()),
            validator_peer_id,
        )
        .unwrap();

    let fullnode = swarm.full_node_mut(fullnode_peer_id).unwrap();
    fullnode
        .wait_until_healthy(Instant::now() + Duration::from_secs(MAX_HEALTHY_WAIT_SECS))
        .await
        .unwrap();

    let client = fullnode.rest_client();

    let account1 = swarm.aptos_public_info().random_account();
    let account2 = swarm.aptos_public_info().random_account();

    let mut chain_info = swarm.chain_info().into_aptos_public_info();
    let factory = chain_info.transaction_factory();
    chain_info
        .create_user_account(account1.public_key())
        .await
        .unwrap();
    // TODO(Gas): double check if this is correct
    chain_info
        .mint(account1.address(), 10_000_000_000)
        .await
        .unwrap();
    chain_info
        .create_user_account(account2.public_key())
        .await
        .unwrap();

    wait_for_account(&client, account1.address()).await.unwrap();

    let txn = account1.sign_with_transaction_builder(
        factory.payload(aptos_stdlib::aptos_coin_transfer(account2.address(), 10)),
    );

    client.submit_and_wait(&txn).await.unwrap();
    let balance = client
        .get_account_balance(account2.address())
        .await
        .unwrap()
        .into_inner();

    assert_eq!(balance.get(), 10);
}

async fn wait_for_account(client: &RestClient, address: AccountAddress) -> Result<()> {
    const DEFAULT_WAIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);
    let start = std::time::Instant::now();
    while start.elapsed() < DEFAULT_WAIT_TIMEOUT {
        if client.get_account(address).await.is_ok() {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    bail!("wait for account(address={}) timeout", address,)
}
