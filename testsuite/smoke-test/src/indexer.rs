// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_indexer::{
    database::{new_db_pool, PgDbPool, PgPoolConnection},
    default_processor::DefaultTransactionProcessor,
    indexer::tailer::Tailer,
    models::transactions::TransactionModel,
};
use diesel::connection::Connection;
use forge::{AptosContext, AptosTest, Result, Test};
use std::sync::Arc;

pub struct Indexer;

impl Test for Indexer {
    fn name(&self) -> &'static str {
        "ecosystem::indexer"
    }
}

pub fn wipe_database(conn: &PgPoolConnection) {
    for table in [
        "write_set_changes",
        "events",
        "user_transactions",
        "block_metadata_transactions",
        "transactions",
        "processor_statuses",
        "__diesel_schema_migrations",
    ] {
        conn.execute(&format!("DROP TABLE IF EXISTS {}", table))
            .unwrap();
    }
}

/// By default, skips test unless `INDEXER_DATABASE_URL` is set.
/// In CI, will explode if `INDEXER_DATABASE_URL` is NOT set.
pub fn should_skip() -> bool {
    if std::env::var("CIRCLECI").is_ok() {
        std::env::var("INDEXER_DATABASE_URL").expect("must set 'INDEXER_DATABASE_URL' in CI!");
    }
    if std::env::var("INDEXER_DATABASE_URL").is_ok() {
        false
    } else {
        println!("`INDEXER_DATABASE_URL` is not set: skipping indexer tests");
        true
    }
}

pub fn setup_indexer(ctx: &mut AptosContext) -> anyhow::Result<(PgDbPool, Tailer)> {
    let database_url = std::env::var("INDEXER_DATABASE_URL")
        .expect("must set 'INDEXER_DATABASE_URL' to run tests!");

    let conn_pool = new_db_pool(database_url.as_str())?;
    wipe_database(&conn_pool.get()?);

    let mut tailer = Tailer::new(ctx.url(), conn_pool.clone())?;
    tailer.run_migrations();

    let pg_transaction_processor = DefaultTransactionProcessor::new(conn_pool.clone());
    tailer.add_processor(Arc::new(pg_transaction_processor));
    Ok((conn_pool, tailer))
}

#[async_trait::async_trait]
impl AptosTest for Indexer {
    async fn run<'t>(&self, ctx: &mut AptosContext<'t>) -> Result<()> {
        if aptos_indexer::should_skip_pg_tests() {
            return Ok(());
        }
        let (conn_pool, mut tailer) = setup_indexer(ctx)?;

        let client = ctx.client();
        client.get_ledger_information().await.unwrap();

        // Set up accounts, generate some traffic
        let mut account1 = ctx.create_and_fund_user_account(1000).await.unwrap();
        let account2 = ctx.create_and_fund_user_account(1000).await.unwrap();
        // This transfer should emit events
        let t_tx = ctx.transfer(&mut account1, &account2, 717).await.unwrap();

        // Why do this twice? To ensure the idempotency of the tailer :-)
        let mut version: u64 = 0;
        for _ in 0..2 {
            // Process the next versions
            version = client
                .get_ledger_information()
                .await
                .unwrap()
                .into_inner()
                .version;

            tailer.process_next_batch((version + 1) as u8).await;

            // Get them into the array and sort by type in order to prevent ordering from breaking tests
            let mut transactions = vec![];
            for v in 0..2 {
                transactions.push(TransactionModel::get_by_version(v, &conn_pool.get()?).unwrap());
            }
            transactions.sort_by(|a, b| a.0.type_.partial_cmp(&b.0.type_).unwrap());

            // This is a block metadata transaction
            let (tx1, ut1, bmt1, events1, wsc1) = &transactions[0];
            assert_eq!(tx1.type_, "block_metadata_transaction");
            assert!(ut1.is_none());
            assert!(bmt1.is_some());
            assert!(events1.is_empty());
            assert!(!wsc1.is_empty());

            // This is the genesis transaction
            let (tx0, ut0, bmt0, events0, wsc0) = &transactions[1];
            assert_eq!(tx0.type_, "genesis_transaction");
            assert!(ut0.is_none());
            assert!(bmt0.is_none());
            assert!(!events0.is_empty());
            assert!(wsc0.len() > 10);

            // This is the transfer
            let (tx2, ut2, bmt2, events2, wsc2) =
                TransactionModel::get_by_hash(t_tx.hash.to_string().as_str(), &conn_pool.get()?)
                    .unwrap();

            assert_eq!(tx2.type_, "user_transaction");
            assert_eq!(tx2.hash, t_tx.hash.to_string());

            // This is a user transaction, so the bmt should be None
            assert!(ut2.is_some());
            assert!(bmt2.is_none());
            assert!(wsc2.len() > 1);
            assert_eq!(events2.len(), 2);
            assert_eq!(events2.get(0).unwrap().type_, "0x1::TestCoin::SentEvent");
            assert_eq!(
                events2.get(1).unwrap().type_,
                "0x1::TestCoin::ReceivedEvent"
            );
        }

        let latest_version = tailer.set_fetcher_to_lowest_processor_version().await;
        assert!(latest_version > version);

        Ok(())
    }
}
