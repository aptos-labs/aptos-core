// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    smoke_test_environment::new_local_swarm_with_aptos,
    state_sync_utils::create_fullnode,
    utils::{create_test_accounts, execute_transactions, MAX_HEALTHY_WAIT_SECS},
};
use anyhow::bail;
use aptos_cached_packages::aptos_stdlib;
use aptos_config::config::{BootstrappingMode, NodeConfig, OverrideNodeConfig};
use aptos_db_indexer_schemas::{
    metadata::MetadataKey,
    schema::{indexer_metadata::InternalIndexerMetadataSchema, state_keys::StateKeysSchema},
};
use aptos_forge::{NodeExt, Result, Swarm, SwarmExt};
use aptos_indexer_grpc_table_info::internal_indexer_db_service::InternalIndexerDBService;
use aptos_rest_client::Client as RestClient;
use aptos_schemadb::DB;
use aptos_types::{account_address::AccountAddress, state_store::state_key::StateKey};
use std::{
    collections::HashSet,
    sync::Arc,
    time::{Duration, Instant},
};
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

    let fullnode = swarm.full_node(fullnode_peer_id).unwrap();
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

    wait_for_account_balance(&client, account1.address())
        .await
        .unwrap();

    let txn = account1.sign_with_transaction_builder(
        factory.payload(aptos_stdlib::aptos_coin_transfer(account2.address(), 10)),
    );

    client.submit_and_wait(&txn).await.unwrap();
    let balance = client
        .view_apt_account_balance(account2.address())
        .await
        .unwrap()
        .into_inner();

    assert_eq!(balance, 10);
}

async fn wait_for_account_balance(client: &RestClient, address: AccountAddress) -> Result<()> {
    const DEFAULT_WAIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);
    let start = std::time::Instant::now();
    while start.elapsed() < DEFAULT_WAIT_TIMEOUT {
        if client
            .get_account_balance(address, "0x1::aptos_coin::AptosCoin")
            .await
            .unwrap()
            .into_inner()
            > 0
        {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    bail!("wait for account(address={}) timeout", address,)
}

fn enable_internal_indexer(node_config: &mut NodeConfig) {
    node_config.storage.rocksdb_configs.enable_storage_sharding = true;
    node_config.indexer_db_config.enable_event = true;
    node_config.indexer_db_config.enable_transaction = true;
    node_config.indexer_db_config.enable_statekeys = true;
}

#[tokio::test]
async fn test_internal_indexer_with_fast_sync() {
    // Create a swarm with 2 validators
    let mut swarm = new_local_swarm_with_aptos(1).await;

    let validator_peer_id = swarm.validators().next().unwrap().peer_id();
    let validator_client = swarm.validator(validator_peer_id).unwrap().rest_client();
    let (mut account_0, account_1) = create_test_accounts(&mut swarm).await;

    execute_transactions(
        &mut swarm,
        &validator_client,
        &mut account_0,
        &account_1,
        true,
    )
    .await;

    let ledger_info = validator_client.get_ledger_information().await.unwrap();
    println!("ledger_info: {:?}", ledger_info);
    let mut vfn_config = NodeConfig::get_default_vfn_config();
    vfn_config.storage.rocksdb_configs.enable_storage_sharding = true;
    vfn_config.state_sync.state_sync_driver.bootstrapping_mode =
        BootstrappingMode::DownloadLatestStates;
    vfn_config
        .storage
        .storage_pruner_config
        .ledger_pruner_config
        .enable = true;
    vfn_config
        .storage
        .storage_pruner_config
        .ledger_pruner_config
        .prune_window = 100;
    vfn_config
        .storage
        .storage_pruner_config
        .ledger_pruner_config
        .batch_size = 50;
    vfn_config
        .storage
        .storage_pruner_config
        .ledger_pruner_config
        .user_pruning_window_offset = 30;

    enable_internal_indexer(&mut vfn_config);

    let peer_id = create_fullnode(vfn_config.clone(), &mut swarm).await;
    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(60))
        .await
        .unwrap();
    let node = swarm.full_node(peer_id).unwrap();
    let node_config = node.config().to_owned();
    node.stop().await.unwrap();
    check_indexer_db(&node_config);
}

fn check_indexer_db(vfn_config: &NodeConfig) {
    let internal_indexer_db = InternalIndexerDBService::get_indexer_db(vfn_config).unwrap();
    let opt = internal_indexer_db
        .get_restore_version_and_progress()
        .unwrap();
    assert!(opt.is_some());
    let indexer_keys: HashSet<StateKey> = get_indexer_db_content::<StateKeysSchema, StateKey>(
        internal_indexer_db.get_inner_db_clone(),
    );
    let meta_keys = get_indexer_db_content::<InternalIndexerMetadataSchema, MetadataKey>(
        internal_indexer_db.get_inner_db_clone(),
    );
    assert!(meta_keys.contains(&MetadataKey::EventPrunerProgress));
    assert!(meta_keys.contains(&MetadataKey::TransactionPrunerProgress));
    assert!(!indexer_keys.is_empty());
}

fn get_indexer_db_content<T, U>(internal_indexer_db: Arc<DB>) -> HashSet<U>
where
    T: aptos_schemadb::schema::Schema,
    U: aptos_schemadb::schema::KeyCodec<T> + std::cmp::Ord + std::fmt::Debug,
    std::collections::HashSet<U>:
        std::iter::FromIterator<<T as aptos_schemadb::schema::Schema>::Key>,
{
    let mut indexer_db_iter = internal_indexer_db.iter::<T>().unwrap();
    indexer_db_iter.seek_to_first();
    indexer_db_iter.map(|iter| iter.unwrap().0).collect()
}
