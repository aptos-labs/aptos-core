// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::schema::{
    db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
    transaction_accumulator::TransactionAccumulatorSchema,
    transaction_accumulator_root_hash::TransactionAccumulatorRootHashSchema,
};
use anyhow::anyhow;
use aptos_accumulator::{HashReader, MerkleAccumulator};
use aptos_crypto::{
    HashValue,
    hash::{CryptoHash, TransactionAccumulatorHasher},
};
use aptos_schemadb::{DB, batch::SchemaBatch};
use aptos_storage_interface::Result;
use aptos_types::{
    proof::{
        AccumulatorConsistencyProof, TransactionAccumulatorProof, TransactionAccumulatorRangeProof,
        definition::LeafCount, position::Position,
    },
    transaction::{TransactionInfo, Version},
};
use std::{borrow::Borrow, path::Path, sync::Arc};

pub(crate) type Accumulator =
    MerkleAccumulator<TransactionAccumulatorDb, TransactionAccumulatorHasher>;

#[derive(Debug)]
pub(crate) struct TransactionAccumulatorDb {
    db: Arc<DB>,
}

impl TransactionAccumulatorDb {
    pub(super) fn new(db: Arc<DB>) -> Self {
        Self { db }
    }

    pub(super) fn create_checkpoint(&self, path: impl AsRef<Path>) -> Result<()> {
        self.db.create_checkpoint(path)
    }

    pub(super) fn write_pruner_progress(&self, version: Version) -> Result<()> {
        self.db.put::<DbMetadataSchema>(
            &DbMetadataKey::TransactionAccumulatorPrunerProgress,
            &DbMetadataValue::Version(version),
        )
    }

    pub(super) fn db(&self) -> &DB {
        &self.db
    }

    pub(crate) fn write_schemas(&self, batch: SchemaBatch) -> Result<()> {
        self.db.write_schemas(batch)
    }
}

impl TransactionAccumulatorDb {
    /// Returns frozen subtree root hashes of the accumulator, from left to right.
    pub fn get_frozen_subtree_hashes(&self, num_transactions: LeafCount) -> Result<Vec<HashValue>> {
        Accumulator::get_frozen_subtree_hashes(self, num_transactions).map_err(Into::into)
    }

    /// Returns proof for transaction at `version` towards root of ledger at `ledger_version`.
    pub fn get_transaction_proof(
        &self,
        version: Version,
        ledger_version: Version,
    ) -> Result<TransactionAccumulatorProof> {
        Accumulator::get_proof(self, ledger_version + 1 /* num_leaves */, version)
            .map_err(Into::into)
    }

    /// Returns proof for `num_txns` consecutive transactions starting from `start_version` towards
    /// root of ledger at `ledger_version`.
    pub fn get_transaction_range_proof(
        &self,
        start_version: Option<Version>,
        num_txns: u64,
        ledger_version: Version,
    ) -> Result<TransactionAccumulatorRangeProof> {
        Accumulator::get_range_proof(
            self,
            ledger_version + 1, /* num_leaves */
            start_version,
            num_txns,
        )
        .map_err(Into::into)
    }

    /// Returns proof that shows the ledger at `ledger_version` is consistent with the ledger at
    /// `client_known_version`.
    pub fn get_consistency_proof(
        &self,
        client_known_version: Option<Version>,
        ledger_version: Version,
    ) -> Result<AccumulatorConsistencyProof> {
        let client_known_num_leaves = client_known_version
            .map(|v| v.saturating_add(1))
            .unwrap_or(0);
        let ledger_num_leaves = ledger_version.saturating_add(1);
        Accumulator::get_consistency_proof(self, ledger_num_leaves, client_known_num_leaves)
            .map_err(Into::into)
    }

    /// Saves a batch of txn_info hashes starting from `first_version` in accumulator.
    pub fn put_transaction_accumulator(
        &self,
        first_version: Version,
        txn_infos: &[impl Borrow<TransactionInfo>],
        transaction_accumulator_batch: &mut SchemaBatch,
    ) -> Result<HashValue> {
        let txn_hashes: Vec<HashValue> = txn_infos.iter().map(|t| t.borrow().hash()).collect();

        let (root_hash, writes) = Accumulator::append(
            self,
            first_version, /* num_existing_leaves */
            &txn_hashes,
        )?;
        writes.iter().try_for_each(|(pos, hash)| {
            transaction_accumulator_batch.put::<TransactionAccumulatorSchema>(pos, hash)
        })?;

        Ok(root_hash)
    }

    /// Returns the root hash at given `version`.
    pub fn get_root_hash(&self, version: Version) -> Result<HashValue> {
        if let Some(hash) = self
            .db
            .get::<TransactionAccumulatorRootHashSchema>(&version)?
        {
            return Ok(hash);
        }
        Accumulator::get_root_hash(self, version + 1).map_err(Into::into)
    }

    /// Deletes the transaction accumulator between a range of version in [begin, end).
    ///
    /// To avoid always pruning a full left subtree, we uses the following algorithm.
    /// For each leaf with an odd leaf index.
    /// 1. From the bottom upwards, find the first ancestor that's a left child of its parent.
    /// (the position of which can be got by popping "1"s from the right of the leaf address).
    /// Note that this node DOES NOT become non-useful.
    /// 2. From the node found from the previous step, delete both its children non-useful, and go
    /// to the right child to repeat the process until we reach a leaf node.
    /// More details are in this issue https://github.com/aptos-labs/aptos-core/issues/1288.
    pub(crate) fn prune(begin: Version, end: Version, db_batch: &mut SchemaBatch) -> Result<()> {
        for version_to_delete in begin..end {
            db_batch.delete::<TransactionAccumulatorRootHashSchema>(&version_to_delete)?;
            // The even version will be pruned in the iteration of version + 1.
            if version_to_delete % 2 == 0 {
                continue;
            }

            let first_ancestor_that_is_a_left_child =
                Self::find_first_ancestor_that_is_a_left_child(version_to_delete);

            // This assertion is true because we skip the leaf nodes with address which is a
            // a multiple of 2.
            assert!(!first_ancestor_that_is_a_left_child.is_leaf());

            let mut current = first_ancestor_that_is_a_left_child;
            while !current.is_leaf() {
                db_batch.delete::<TransactionAccumulatorSchema>(&current.left_child())?;
                db_batch.delete::<TransactionAccumulatorSchema>(&current.right_child())?;
                current = current.right_child();
            }
        }
        Ok(())
    }

    /// Returns the first ancestor that is a child of its parent.
    fn find_first_ancestor_that_is_a_left_child(version: Version) -> Position {
        // We can get the first ancestor's position based on the two observations:
        // - floor(level position of a node / 2) = level position of its parent.
        // - if a node is a left child of its parent, its level position should be a multiple of 2.
        // - level position means the position counted from 0 of a single tree level. For example,
        //                a (level position = 0)
        //         /                                \
        //    b (level position = 0)      c(level position = 1)
        //
        // To find the first ancestor which is a left child of its parent, we can keep diving the
        // version by 2 (to find the ancestor) until we get a number which is a multiple of 2
        // (to make sure the ancestor is a left child of its parent). The number of time we
        // divide the version is the level of the ancestor. The remainder is the level position
        // of the ancestor.
        let first_ancestor_that_is_a_left_child_level = version.trailing_ones();
        let index_in_level = version >> first_ancestor_that_is_a_left_child_level;
        Position::from_level_and_pos(first_ancestor_that_is_a_left_child_level, index_in_level)
    }
}

impl HashReader for TransactionAccumulatorDb {
    fn get(&self, position: Position) -> Result<HashValue, anyhow::Error> {
        self.db
            .get::<TransactionAccumulatorSchema>(&position)?
            .ok_or_else(|| anyhow!("{} does not exist.", position))
    }
}
