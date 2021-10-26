// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use anyhow::ensure;
use diem_types::{
    protocol_spec::DpnProto,
    transaction::{Transaction, TransactionInfo, Version},
};
use diem_vm::VMExecutor;
use executor_types::TransactionReplayer;

use crate::Executor;

impl<V: VMExecutor> TransactionReplayer for Executor<DpnProto, V> {
    fn replay_chunk(
        &self,
        mut first_version: Version,
        mut txns: Vec<Transaction>,
        mut txn_infos: Vec<TransactionInfo>,
    ) -> anyhow::Result<()> {
        let read_lock = self.cache.read();
        ensure!(
            first_version == read_lock.synced_trees().txn_accumulator().num_leaves(),
            "Version not expected. Expected: {}, got: {}",
            read_lock.synced_trees().txn_accumulator().num_leaves(),
            first_version,
        );
        drop(read_lock);
        while !txns.is_empty() {
            let num_txns = txns.len();

            let (output, txns_to_commit, _, txns_to_retry, txn_infos_to_retry) =
                self.replay_transactions_impl(first_version, txns, None, txn_infos)?;
            assert!(txns_to_retry.len() < num_txns);

            self.db
                .writer
                .save_transactions(&txns_to_commit, first_version, None)?;

            self.cache
                .write()
                .update_synced_trees(output.executed_trees().clone());

            txns = txns_to_retry;
            txn_infos = txn_infos_to_retry;
            first_version += txns_to_commit.len() as u64;
        }
        Ok(())
    }

    fn expecting_version(&self) -> Version {
        self.cache
            .read()
            .synced_trees()
            .version()
            .map_or(0, |v| v.checked_add(1).expect("Integer overflow occurred"))
    }
}
