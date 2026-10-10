// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Recording and replaying generated blocks.
//!
//! Comparing two VMs requires both to execute the same transactions, but the
//! workload generators draw from `thread_rng`, so two runs of the same command
//! produce different transaction streams. Recording one run and replaying it
//! removes that difference.
//!
//! # The two directories
//!
//! Every benchmark run takes a `source_dir` (`--data-dir`) and a
//! `checkpoint_dir` (`--checkpoint-dir`). The run copies the DB in `source_dir`
//! into `checkpoint_dir` and works on the copy, so `source_dir` is left
//! untouched and can be reused by the next run.
//!
//! Recording and replaying chain those two directories:
//!
//! 1. A recording run reads the warmup DB from `source_dir`, initializes the
//!    workload into `checkpoint_dir`, writes the generated blocks to a file, and
//!    stops. Nothing is executed and no feature flag is overridden, so
//!    `checkpoint_dir` holds exactly the state the blocks were generated
//!    against.
//! 2. Each replay run passes that `checkpoint_dir` as its own `source_dir`. The
//!    workload is already initialized there, so the replay only applies its
//!    feature flag overrides and executes the recorded blocks against its own
//!    fresh copy.
//!
//! Replays are therefore independent of each other: each one starts from the
//! same recorded base and throws its copy away.
//!
//! # What is recorded
//!
//! The user transactions of each block. Block metadata is dropped and re-minted
//! per block at replay: the recorded one carries the epoch and timestamp of the
//! recording, and the replay's own feature flag overrides move both. Anything
//! else in a block is rejected at record time rather than dropped.

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
                 A replay has to start from the DB the recording left behind, \
                 and check this before applying its own feature flag overrides.",
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

/// Strips the block metadata transaction, which a replay mints fresh against
/// its own DB.
///
/// Panics on any other non-user transaction. A generated block holds nothing
/// else today, and a new kind turning up needs a decision on how a replay
/// reproduces it rather than a silent drop.
pub fn user_transactions(block: Vec<Transaction>) -> Vec<SignedTransaction> {
    block
        .into_iter()
        .filter_map(|txn| match txn {
            Transaction::UserTransaction(txn) => Some(txn),
            Transaction::BlockMetadata(_) | Transaction::BlockMetadataExt(_) => None,
            other @ (Transaction::GenesisTransaction(_)
            | Transaction::StateCheckpoint(_)
            | Transaction::ValidatorTransaction(_)
            | Transaction::BlockEpilogue(_)) => {
                panic!("cannot record a {} transaction", other.type_name())
            },
        })
        .collect()
}
