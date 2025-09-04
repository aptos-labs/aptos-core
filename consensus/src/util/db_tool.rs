// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::unwrap_used)]

use crate::{
    consensusdb::ConsensusDB,
    quorum_store::{
        quorum_store_db::{QuorumStoreDB, QuorumStoreStorage},
        types::PersistedValue,
    },
};
use anyhow::{bail, Result};
use velor_consensus_types::{block::Block, common::Payload};
use velor_crypto::HashValue;
use velor_types::transaction::{SignedTransaction, Transaction};
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
            #[allow(clippy::unwrap_in_result)]
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

fn extract_txns_from_quorum_store(
    digests: impl Iterator<Item = HashValue>,
    all_batches: &HashMap<HashValue, PersistedValue>,
) -> anyhow::Result<Vec<&SignedTransaction>> {
    let mut block_txns = Vec::new();
    for digest in digests {
        if let Some(batch) = all_batches.get(&digest) {
            if let Some(txns) = batch.payload() {
                block_txns.extend(txns);
            } else {
                bail!("Payload is not found for batch ({digest}).");
            }
        } else {
            bail!("Batch ({digest}) is not found.");
        }
    }
    Ok(block_txns)
}

pub fn extract_txns_from_block<'a>(
    block: &'a Block,
    all_batches: &'a HashMap<HashValue, PersistedValue>,
) -> anyhow::Result<Vec<&'a SignedTransaction>> {
    match block.payload().as_ref() {
        Some(payload) => match payload {
            Payload::DirectMempool(_) => {
                bail!("DirectMempool is not supported.");
            },
            Payload::InQuorumStore(proof_with_data) => extract_txns_from_quorum_store(
                proof_with_data.proofs.iter().map(|proof| *proof.digest()),
                all_batches,
            ),
            Payload::InQuorumStoreWithLimit(proof_with_data) => extract_txns_from_quorum_store(
                proof_with_data
                    .proof_with_data
                    .proofs
                    .iter()
                    .map(|proof| *proof.digest()),
                all_batches,
            ),
            Payload::QuorumStoreInlineHybrid(inline_batches, proof_with_data, _)
            | Payload::QuorumStoreInlineHybridV2(inline_batches, proof_with_data, _) => {
                let mut all_txns = extract_txns_from_quorum_store(
                    proof_with_data.proofs.iter().map(|proof| *proof.digest()),
                    all_batches,
                )
                .unwrap();
                for (_, txns) in inline_batches {
                    all_txns.extend(txns);
                }
                Ok(all_txns)
            },
            Payload::OptQuorumStore(opt_qs_payload) => {
                let mut all_txns = extract_txns_from_quorum_store(
                    opt_qs_payload
                        .proof_with_data()
                        .iter()
                        .map(|proof| *proof.digest()),
                    all_batches,
                )
                .unwrap();
                all_txns.extend(
                    extract_txns_from_quorum_store(
                        opt_qs_payload
                            .opt_batches()
                            .iter()
                            .map(|info| *info.digest()),
                        all_batches,
                    )
                    .unwrap(),
                );
                Ok(all_txns)
            },
        },
        None => Ok(vec![]),
    }
}
