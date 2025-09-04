// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use anyhow::{ensure, Result};
use velor_crypto::HashValue;
use velor_drop_helper::DropHelper;
use velor_types::{
    proof::accumulator::InMemoryTransactionAccumulator,
    transaction::{TransactionInfo, Version},
};
use derive_more::Deref;
use itertools::zip_eq;
use std::{clone::Clone, sync::Arc};

#[derive(Clone, Debug, Default, Deref)]
pub struct LedgerUpdateOutput {
    #[deref]
    inner: Arc<DropHelper<Inner>>,
}

impl LedgerUpdateOutput {
    pub fn new(
        transaction_infos: Vec<TransactionInfo>,
        transaction_info_hashes: Vec<HashValue>,
        transaction_accumulator: Arc<InMemoryTransactionAccumulator>,
        parent_accumulator: Arc<InMemoryTransactionAccumulator>,
    ) -> Self {
        Self::new_impl(Inner {
            transaction_infos,
            transaction_info_hashes,
            transaction_accumulator,
            parent_accumulator,
        })
    }

    pub fn new_empty(transaction_accumulator: Arc<InMemoryTransactionAccumulator>) -> Self {
        Self::new_impl(Inner::new_empty(transaction_accumulator))
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn new_dummy() -> Self {
        Self::new_empty(Arc::new(InMemoryTransactionAccumulator::new_empty()))
    }

    pub fn new_dummy_with_root_hash(root_hash: HashValue) -> Self {
        let transaction_accumulator = Arc::new(
            InMemoryTransactionAccumulator::new_empty_with_root_hash(root_hash),
        );
        Self::new_impl(Inner {
            parent_accumulator: transaction_accumulator.clone(),
            transaction_accumulator,
            ..Default::default()
        })
    }

    pub fn reconfig_suffix(&self) -> Self {
        Self::new_impl(Inner::new_empty(self.transaction_accumulator.clone()))
    }

    fn new_impl(inner: Inner) -> Self {
        Self {
            inner: Arc::new(DropHelper::new(inner)),
        }
    }
}

#[derive(Default, Debug)]
pub struct Inner {
    pub transaction_infos: Vec<TransactionInfo>,
    pub transaction_info_hashes: Vec<HashValue>,
    pub transaction_accumulator: Arc<InMemoryTransactionAccumulator>,
    pub parent_accumulator: Arc<InMemoryTransactionAccumulator>,
}

impl Inner {
    pub fn new_empty(transaction_accumulator: Arc<InMemoryTransactionAccumulator>) -> Self {
        Self {
            parent_accumulator: transaction_accumulator.clone(),
            transaction_accumulator,
            ..Default::default()
        }
    }

    pub fn txn_accumulator(&self) -> &Arc<InMemoryTransactionAccumulator> {
        &self.transaction_accumulator
    }

    pub fn ensure_transaction_infos_match(
        &self,
        transaction_infos: &[TransactionInfo],
    ) -> Result<()> {
        ensure!(
            self.transaction_infos.len() == transaction_infos.len(),
            "Lengths don't match. {} vs {}",
            self.transaction_infos.len(),
            transaction_infos.len(),
        );

        let mut version = self.first_version();
        for (txn_info, expected_txn_info) in
            zip_eq(self.transaction_infos.iter(), transaction_infos.iter())
        {
            ensure!(
                txn_info == expected_txn_info,
                "Transaction infos don't match. version:{version}, txn_info:{txn_info}, expected_txn_info:{expected_txn_info}",
            );
            version += 1;
        }
        Ok(())
    }

    pub fn first_version(&self) -> Version {
        self.parent_accumulator.num_leaves
    }
}
