// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Recording and replaying generated blocks.
//!
//! Comparing two VMs requires both to execute the same transactions, but the
//! workload generators draw from `thread_rng`, so two runs of the same command
//! produce different transaction streams. Recording one run and replaying it
//! removes that difference.
//!
//! Only user transactions are recorded. Block metadata carries an epoch and a
//! timestamp that go stale as soon as a replay commits anything, so replay mints
//! a fresh metadata transaction per block from the target DB.

use anyhow::{bail, Result};
use aptos_logger::info;
use aptos_types::transaction::{SignedTransaction, Transaction, Version};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{BufReader, BufWriter, Read},
    path::Path,
};

/// Bumped whenever the layout below changes, so an old file is rejected rather
/// than misparsed.
pub const FORMAT_VERSION: u32 = 1;

/// State the blocks were generated against. A replay must start from a DB in the
/// same state, otherwise sequence numbers and transaction expirations do not
/// line up. Checked in [`RecordedBlocks::check_replayable_at`].
#[derive(Debug, Serialize, Deserialize)]
pub struct RecordingHeader {
    pub format_version: u32,
    pub workload_name: String,
    pub block_size: usize,
    /// DB version at the time of generation.
    pub version: Version,
    /// `CurrentTimeMicroseconds` at the time of generation. Recorded
    /// transactions expire 60 seconds after this.
    pub base_usecs: u64,
    pub epoch: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecordedBlocks {
    pub header: RecordingHeader,
    pub blocks: Vec<Vec<SignedTransaction>>,
}

impl RecordedBlocks {
    pub fn write(&self, path: impl AsRef<Path>) -> Result<()> {
        let file = File::create(path.as_ref())?;
        bcs::serialize_into(&mut BufWriter::new(file), self)?;
        info!(
            "Recorded {} blocks ({} transactions) to {:?}",
            self.blocks.len(),
            self.num_transactions(),
            path.as_ref()
        );
        Ok(())
    }

    pub fn read(path: impl AsRef<Path>) -> Result<Self> {
        let mut bytes = vec![];
        BufReader::new(File::open(path.as_ref())?).read_to_end(&mut bytes)?;
        let recorded: Self = bcs::from_bytes(&bytes)?;
        if recorded.header.format_version != FORMAT_VERSION {
            bail!(
                "recorded blocks at {:?} are format version {}, expected {}",
                path.as_ref(),
                recorded.header.format_version,
                FORMAT_VERSION
            );
        }
        info!(
            "Replaying {} blocks ({} transactions) of {} from {:?}",
            recorded.blocks.len(),
            recorded.num_transactions(),
            recorded.header.workload_name,
            path.as_ref()
        );
        Ok(recorded)
    }

    pub fn num_transactions(&self) -> usize {
        self.blocks.iter().map(Vec::len).sum()
    }

    /// Fails if the DB is not in the state the blocks were generated against. A
    /// mismatch means the replay would run at wrong sequence numbers, or far
    /// enough past the recorded expirations that every transaction is discarded.
    pub fn check_replayable_at(&self, version: Version, base_usecs: u64, epoch: u64) -> Result<()> {
        let h = &self.header;
        if (h.version, h.base_usecs, h.epoch) != (version, base_usecs, epoch) {
            bail!(
                "recorded blocks do not match the DB being replayed against: \
                 recorded (version {}, base_usecs {}, epoch {}), \
                 found (version {}, base_usecs {}, epoch {}). \
                 The replay must start from the same checkpoint the recording did, \
                 and apply the same number of feature flips.",
                h.version,
                h.base_usecs,
                h.epoch,
                version,
                base_usecs,
                epoch
            );
        }
        Ok(())
    }
}

/// Keeps the user transactions of each block, dropping block metadata and any
/// other non-user transaction.
pub fn user_transactions(block: Vec<Transaction>) -> Vec<SignedTransaction> {
    block
        .into_iter()
        .filter_map(|txn| match txn {
            Transaction::UserTransaction(txn) => Some(txn),
            Transaction::GenesisTransaction(_)
            | Transaction::BlockMetadata(_)
            | Transaction::BlockMetadataExt(_)
            | Transaction::StateCheckpoint(_)
            | Transaction::ValidatorTransaction(_)
            | Transaction::BlockEpilogue(_) => None,
        })
        .collect()
}
