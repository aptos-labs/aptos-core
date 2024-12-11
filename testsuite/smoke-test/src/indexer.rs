// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_cached_packages::aptos_stdlib::aptos_token_stdlib;
use aptos_forge::{AptosPublicInfo, Result, Swarm};
use aptos_indexer::{
    database::{new_db_pool, PgDbPool, PgPoolConnection},
    models::transactions::TransactionQuery,
};
use aptos_sdk::types::LocalAccount;
use diesel::RunQueryDsl;
use std::sync::Arc;

pub fn wipe_database(conn: &mut PgPoolConnection) {
    for command in [
        "DROP SCHEMA public CASCADE",
        "CREATE SCHEMA public",
        "GRANT ALL ON SCHEMA public TO postgres",
        "GRANT ALL ON SCHEMA public TO public",
    ] {
        diesel::sql_query(command).execute(conn).unwrap();
    }
}

pub fn get_database_url() -> String {
    std::env::var("INDEXER_DATABASE_URL").expect("must set 'INDEXER_DATABASE_URL' to run tests!")
}

pub fn setup_indexer() -> anyhow::Result<PgDbPool> {
    let conn_pool = new_db_pool(get_database_url().as_str())?;
    wipe_database(&mut conn_pool.get()?);
    Ok(conn_pool)
}

pub async fn execute_nft_txns<'t>(creator: LocalAccount, info: &mut AptosPublicInfo) -> Result<()> {
    let collection_name = "collection name".to_owned().into_bytes();
    let token_name = "token name".to_owned().into_bytes();
    let collection_builder =
        info.transaction_factory()
            .payload(aptos_token_stdlib::token_create_collection_script(
                collection_name.clone(),
                "description".to_owned().into_bytes(),
                "uri".to_owned().into_bytes(),
                20_000_000,
                vec![false, false, false],
            ));

    let collection_txn = creator.sign_with_transaction_builder(collection_builder);
    info.client().submit_and_wait(&collection_txn).await?;

    let token_builder =
        info.transaction_factory()
            .payload(aptos_token_stdlib::token_create_token_script(
                collection_name.clone(),
                token_name.clone(),
                "collection description".to_owned().into_bytes(),
                3,
                4,
                "uri".to_owned().into_bytes(),
                creator.address(),
                1,
                0,
                vec![false, false, false, false, true],
                vec!["age".as_bytes().to_vec()],
                vec!["3".as_bytes().to_vec()],
                vec!["int".as_bytes().to_vec()],
            ));

    let token_txn = creator.sign_with_transaction_builder(token_builder);
    info.client().submit_and_wait(&token_txn).await?;

    let token_mutator =
        info.transaction_factory()
            .payload(aptos_token_stdlib::token_mutate_token_properties(
                creator.address(),
                creator.address(),
                collection_name.clone(),
                token_name.clone(),
                0,
                2,
                vec!["age".as_bytes().to_vec()],
                vec!["2".as_bytes().to_vec()],
                vec!["int".as_bytes().to_vec()],
            ));
    let mutate_txn = creator.sign_with_transaction_builder(token_mutator);
    info.client().submit_and_wait(&mutate_txn).await?;
    Ok(())
}

// TODO(grao): Old indexer is not used anymore, cleanup corresponding code and tests.
#[ignore]
#[tokio::test]
async fn test_old_indexer() {
    if aptos_indexer::should_skip_pg_tests() {
        return;
    }

    let conn_pool = setup_indexer().unwrap();

    let swarm = crate::smoke_test_environment::SwarmBuilder::new_local(1)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            config.storage.enable_indexer = true;

            config.indexer.enabled = true;
            config.indexer.postgres_uri = Some(get_database_url());
            config.indexer.processor =
                Some(aptos_indexer::processors::default_processor::NAME.to_string());
        }))
        .build()
        .await;

    let mut info = swarm.aptos_public_info();

    let ledger = info
        .client()
        .get_ledger_information()
        .await
        .unwrap()
        .into_inner();

    println!("ledger state: {:?}", ledger);

    // Set up accounts, generate some traffic
    // TODO(Gas): double check this
    let mut account1 = info
        .create_and_fund_user_account(50_000_000_000)
        .await
        .unwrap();
    let account2 = info
        .create_and_fund_user_account(50_000_000_000)
        .await
        .unwrap();
    // This transfer should emit events
    let t_tx = info.transfer(&mut account1, &account2, 717).await.unwrap();
    // test NFT creation event indexing
    execute_nft_txns(account1, &mut info).await.unwrap();

    // Let the test complete! Yes, this does suck.
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    // Get them into the array and sort by type in order to prevent ordering from breaking tests
    let mut transactions = vec![];
    for v in 0..2 {
        transactions
            .push(TransactionQuery::get_by_version(v, &mut conn_pool.get().unwrap()).unwrap());
    }
    transactions.sort_by(|a, b| a.0.type_.partial_cmp(&b.0.type_).unwrap());

    // This is a block metadata transaction
    let (tx1, ut1, bmt1, events1, wsc1) = &transactions[0];
    assert_eq!(tx1.type_, "block_metadata_transaction");
    assert!(ut1.is_none());
    assert!(bmt1.is_some());
    assert!(!events1.is_empty());
    assert!(!wsc1.is_empty());

    // This is the genesis transaction
    let (tx0, ut0, bmt0, events0, wsc0) = &transactions[1];
    assert_eq!(tx0.type_, "genesis_transaction");
    assert!(ut0.is_none());
    assert!(bmt0.is_none());
    assert!(!events0.is_empty());
    assert!(wsc0.len() > 10);

    // This is the transfer
    let (tx2, ut2, bmt2, events2, wsc2) = TransactionQuery::get_by_hash(
        t_tx.hash.to_string().as_str(),
        &mut conn_pool.get().unwrap(),
    )
    .unwrap();

    assert_eq!(tx2.type_, "user_transaction");
    assert_eq!(tx2.hash, t_tx.hash.to_string());

    // This is a user transaction, so the bmt should be None
    assert!(ut2.is_some());
    assert!(bmt2.is_none());
    assert!(wsc2.len() > 1);
    assert_eq!(events2.len(), 2);
    assert_eq!(events2.first().unwrap().type_, "0x1::coin::WithdrawEvent");
    assert_eq!(events2.get(1).unwrap().type_, "0x1::coin::DepositEvent");
}
