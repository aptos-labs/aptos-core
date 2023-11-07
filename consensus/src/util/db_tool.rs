// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    consensusdb::ConsensusDB,
    quorum_store::{
        quorum_store_db::{QuorumStoreDB, QuorumStoreStorage},
        types::PersistedValue,
    },
};
use anyhow::{bail, Result};
use aptos_consensus_types::{block::Block, common::Payload};
use aptos_crypto::HashValue;
use aptos_types::transaction::{SignedTransaction, Transaction};
use clap::Parser;
use std::{collections::HashMap, path::PathBuf};

#[derive(Parser)]
#[clap(about = "Dump txns from consensus db.")]
pub struct Command {
    #[clap(long, value_parser)]
    pub db_dir: PathBuf,

    // If None, will dump all blocks.
    #[clap(long)]
    pub block_id: Option<HashValue>,
}

impl Command {
    pub async fn run(self) -> Result<()> {
        let txns = self.dump_pending_txns()?;
        println!("{txns:?}");

        Ok(())
    }

    pub fn dump_pending_txns(&self) -> Result<Vec<Transaction>> {
        let quorum_store_db = QuorumStoreDB::new(self.db_dir.clone());
        let all_batches = quorum_store_db.get_all_batches().unwrap();

        let consensus_db = ConsensusDB::new(self.db_dir.clone());
        let (_, _, blocks, _) = consensus_db.get_data()?;

        let mut txns = Vec::new();
        for block in blocks {
            let id = block.id();
            if self.block_id.is_none() || id == self.block_id.unwrap() {
                txns.extend(
                    extract_txns_from_block(&block, &all_batches)?
                        .into_iter()
                        .cloned()
                        .map(Transaction::UserTransaction),
                );
            }
        }

        Ok(txns)
    }
}

pub fn extract_txns_from_block<'a>(
    block: &'a Block,
    all_batches: &'a HashMap<HashValue, PersistedValue>,
) -> anyhow::Result<Vec<&'a SignedTransaction>> {
    match block.payload().as_ref() {
        Some(payload) => {
            let mut block_txns = Vec::new();
            match payload {
                Payload::DirectMempool(_) => {
                    bail!("DirectMempool is not supported.");
                },
                Payload::InQuorumStore(proof_with_data) => {
                    for proof in &proof_with_data.proofs {
                        let digest = proof.digest();
                        if let Some(batch) = all_batches.get(digest) {
                            if let Some(txns) = batch.payload() {
                                block_txns.extend(txns);
                            } else {
                                bail!("Payload is not found for batch ({digest}).");
                            }
                        } else {
                            bail!("Batch ({digest}) is not found.");
                        }
                    }
                },
            }
            Ok(block_txns)
        },
        None => Ok(vec![]),
    }
}
