// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod consensusdb_test;
mod schema;

use crate::error::DbError;
use anyhow::Result;
use velor_consensus_types::{block::Block, quorum_cert::QuorumCert};
use velor_crypto::HashValue;
use velor_logger::prelude::*;
use velor_schemadb::{batch::SchemaBatch, schema::Schema, Options, DB, DEFAULT_COLUMN_FAMILY_NAME};
use velor_storage_interface::VelorDbError;
pub use schema::{
    block::BlockSchema,
    dag::{CertifiedNodeSchema, DagVoteSchema, NodeSchema},
    quorum_certificate::QCSchema,
};
use schema::{
    single_entry::{SingleEntryKey, SingleEntrySchema},
    BLOCK_CF_NAME, CERTIFIED_NODE_CF_NAME, DAG_VOTE_CF_NAME, NODE_CF_NAME, QC_CF_NAME,
    SINGLE_ENTRY_CF_NAME,
};
use std::{iter::Iterator, path::Path, time::Instant};

/// The name of the consensus db file
pub const CONSENSUS_DB_NAME: &str = "consensus_db";

/// Creates new physical DB checkpoint in directory specified by `checkpoint_path`.
pub fn create_checkpoint<P: AsRef<Path> + Clone>(db_path: P, checkpoint_path: P) -> Result<()> {
    let start = Instant::now();
    let consensus_db_checkpoint_path = checkpoint_path.as_ref().join(CONSENSUS_DB_NAME);
    std::fs::remove_dir_all(&consensus_db_checkpoint_path).unwrap_or(());
    ConsensusDB::new(db_path)
        .db
        .create_checkpoint(&consensus_db_checkpoint_path)?;
    info!(
        path = consensus_db_checkpoint_path,
        time_ms = %start.elapsed().as_millis(),
        "Made ConsensusDB checkpoint."
    );
    Ok(())
}

pub struct ConsensusDB {
    db: DB,
}

impl ConsensusDB {
    pub fn new<P: AsRef<Path> + Clone>(db_root_path: P) -> Self {
        let column_families = vec![
            /* UNUSED CF = */ DEFAULT_COLUMN_FAMILY_NAME,
            BLOCK_CF_NAME,
            QC_CF_NAME,
            SINGLE_ENTRY_CF_NAME,
            NODE_CF_NAME,
            CERTIFIED_NODE_CF_NAME,
            DAG_VOTE_CF_NAME,
            "ordered_anchor_id", // deprecated CF
        ];

        let path = db_root_path.as_ref().join(CONSENSUS_DB_NAME);
        let instant = Instant::now();
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        let db = DB::open(path.clone(), "consensus", column_families, &opts)
            .expect("ConsensusDB open failed; unable to continue");

        info!(
            "Opened ConsensusDB at {:?} in {} ms",
            path,
            instant.elapsed().as_millis()
        );

        Self { db }
    }

    pub fn get_data(
        &self,
    ) -> Result<(
        Option<Vec<u8>>,
        Option<Vec<u8>>,
        Vec<Block>,
        Vec<QuorumCert>,
    )> {
        let last_vote = self.get_last_vote()?;
        let highest_2chain_timeout_certificate = self.get_highest_2chain_timeout_certificate()?;
        let consensus_blocks = self
            .get_all::<BlockSchema>()?
            .into_iter()
            .map(|(_, block)| block)
            .collect();
        let consensus_qcs = self
            .get_all::<QCSchema>()?
            .into_iter()
            .map(|(_, qc)| qc)
            .collect();
        Ok((
            last_vote,
            highest_2chain_timeout_certificate,
            consensus_blocks,
            consensus_qcs,
        ))
    }

    pub fn save_highest_2chain_timeout_certificate(&self, tc: Vec<u8>) -> Result<(), DbError> {
        let mut batch = SchemaBatch::new();
        batch.put::<SingleEntrySchema>(&SingleEntryKey::Highest2ChainTimeoutCert, &tc)?;
        self.commit(batch)?;
        Ok(())
    }

    pub fn save_vote(&self, last_vote: Vec<u8>) -> Result<(), DbError> {
        let mut batch = SchemaBatch::new();
        batch.put::<SingleEntrySchema>(&SingleEntryKey::LastVote, &last_vote)?;
        self.commit(batch)
    }

    pub fn save_blocks_and_quorum_certificates(
        &self,
        block_data: Vec<Block>,
        qc_data: Vec<QuorumCert>,
    ) -> Result<(), DbError> {
        if block_data.is_empty() && qc_data.is_empty() {
            return Err(anyhow::anyhow!("Consensus block and qc data is empty!").into());
        }
        let mut batch = SchemaBatch::new();
        block_data
            .iter()
            .try_for_each(|block| batch.put::<BlockSchema>(&block.id(), block))?;
        qc_data
            .iter()
            .try_for_each(|qc| batch.put::<QCSchema>(&qc.certified_block().id(), qc))?;
        self.commit(batch)
    }

    pub fn delete_blocks_and_quorum_certificates(
        &self,
        block_ids: Vec<HashValue>,
    ) -> Result<(), DbError> {
        if block_ids.is_empty() {
            return Err(anyhow::anyhow!("Consensus block ids is empty!").into());
        }
        let mut batch = SchemaBatch::new();
        block_ids.iter().try_for_each(|hash| {
            batch.delete::<BlockSchema>(hash)?;
            batch.delete::<QCSchema>(hash)
        })?;
        self.commit(batch)
    }

    /// Write the whole schema batch including all data necessary to mutate the ledger
    /// state of some transaction by leveraging rocksdb atomicity support.
    fn commit(&self, batch: SchemaBatch) -> Result<(), DbError> {
        self.db.write_schemas_relaxed(batch)?;
        Ok(())
    }

    /// Get latest timeout certificates (we only store the latest highest timeout certificates).
    fn get_highest_2chain_timeout_certificate(&self) -> Result<Option<Vec<u8>>, DbError> {
        Ok(self
            .db
            .get::<SingleEntrySchema>(&SingleEntryKey::Highest2ChainTimeoutCert)?)
    }

    pub fn delete_highest_2chain_timeout_certificate(&self) -> Result<(), DbError> {
        let mut batch = SchemaBatch::new();
        batch.delete::<SingleEntrySchema>(&SingleEntryKey::Highest2ChainTimeoutCert)?;
        self.commit(batch)
    }

    /// Get serialized latest vote (if available)
    fn get_last_vote(&self) -> Result<Option<Vec<u8>>, DbError> {
        Ok(self
            .db
            .get::<SingleEntrySchema>(&SingleEntryKey::LastVote)?)
    }

    pub fn delete_last_vote_msg(&self) -> Result<(), DbError> {
        let mut batch = SchemaBatch::new();
        batch.delete::<SingleEntrySchema>(&SingleEntryKey::LastVote)?;
        self.commit(batch)?;
        Ok(())
    }

    pub fn put<S: Schema>(&self, key: &S::Key, value: &S::Value) -> Result<(), DbError> {
        let mut batch = SchemaBatch::new();
        batch.put::<S>(key, value)?;
        self.commit(batch)?;
        Ok(())
    }

    pub fn delete<S: Schema>(&self, keys: Vec<S::Key>) -> Result<(), DbError> {
        let mut batch = SchemaBatch::new();
        keys.iter().try_for_each(|key| batch.delete::<S>(key))?;
        self.commit(batch)
    }

    pub fn get_all<S: Schema>(&self) -> Result<Vec<(S::Key, S::Value)>, DbError> {
        let mut iter = self.db.iter::<S>()?;
        iter.seek_to_first();
        Ok(iter.collect::<Result<Vec<(S::Key, S::Value)>, VelorDbError>>()?)
    }

    pub fn get<S: Schema>(&self, key: &S::Key) -> Result<Option<S::Value>, DbError> {
        Ok(self.db.get::<S>(key)?)
    }
}
