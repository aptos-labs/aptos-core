// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::db_access::DbAccessUtil;
use anyhow::Result;
use aptos_storage_interface::{
    DbReaderWriter, state_store::state_view::db_state_view::LatestDbStateCheckpointView,
};
use aptos_transaction_generator_lib::{CounterState, ReliableTransactionSubmitter};
use aptos_types::{
    account_address::AccountAddress,
    account_config::AccountResource,
    state_store::MoveResourceExt,
    transaction::{SignedTransaction, Transaction},
};
use async_trait::async_trait;
use std::{
    collections::HashMap,
    sync::{atomic::AtomicUsize, mpsc},
    time::{Duration, Instant},
};

pub struct DbReliableTransactionSubmitter {
    pub db: DbReaderWriter,
    pub block_sender: mpsc::SyncSender<Vec<Transaction>>,
}

#[async_trait]
impl ReliableTransactionSubmitter for DbReliableTransactionSubmitter {
    async fn get_account_balance(&self, account_address: AccountAddress) -> Result<u64> {
        let db_state_view = self.db.reader.latest_state_checkpoint_view().unwrap();
        DbAccessUtil::get_fungible_store(&account_address, &db_state_view)
            .map(|fungible_store| fungible_store.balance())
    }

    async fn query_sequence_number(&self, address: AccountAddress) -> Result<u64> {
        let db_state_view = self.db.reader.latest_state_checkpoint_view().unwrap();
        Ok(
            AccountResource::fetch_move_resource(&db_state_view, &address)
                .unwrap()
                .map(|account| account.sequence_number())
                .unwrap_or(0),
        )
        //.context("account doesn't exist")
    }

    async fn execute_transactions_with_counter(
        &self,
        txns: &[SignedTransaction],
        _state: &CounterState,
    ) -> Result<()> {
        self.block_sender.send(
            txns.iter()
                .map(|t| Transaction::UserTransaction(t.clone()))
                .collect(),
        )?;

        let start = Instant::now();
        for txn in txns {
            loop {
                if let Some(txn_output) = self
                    .db
                    .reader
                    .get_transaction_by_hash(
                        txn.committed_hash(),
                        self.db.reader.get_latest_ledger_info_version().unwrap(),
                        false,
                    )
                    .unwrap()
                {
                    if txn_output.proof.transaction_info().status().is_success() {
                        break;
                    } else {
                        panic!(
                            "Transaction failed: {:?}",
                            txn_output.proof.transaction_info()
                        );
                    }
                }
                if start.elapsed().as_secs() > 30 {
                    panic!("Transaction timed out");
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
        Ok(())
    }

    fn create_counter_state(&self) -> CounterState {
        CounterState {
            submit_failures: vec![AtomicUsize::new(0)],
            wait_failures: vec![AtomicUsize::new(0)],
            successes: AtomicUsize::new(0),
            by_client: HashMap::new(),
        }
    }
}
