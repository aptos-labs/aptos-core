// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    db_debugger::ShardingConfig,
    schema::{
        db_metadata::{DbMetadataKey, DbMetadataSchema},
        event_accumulator::EventAccumulatorSchema,
        ledger_info::LedgerInfoSchema,
        transaction::TransactionSchema,
        transaction_accumulator::TransactionAccumulatorSchema,
        transaction_info::TransactionInfoSchema,
        version_data::VersionDataSchema,
        write_set::WriteSetSchema,
    },
    utils::truncation_helper::{
        get_current_version_in_state_merkle_db, get_ledger_commit_progress,
        get_overall_commit_progress, get_state_kv_commit_progress,
        get_state_merkle_commit_progress,
    },
    AptosDB,
};
use anyhow::Result;
use aptos_config::config::{RocksdbConfigs, StorageDirPaths};
use aptos_consensus::{
    consensusdb::ConsensusDB,
    quorum_store::quorum_store_db::{QuorumStoreDB, QuorumStoreStorage},
};
use aptos_consensus_types::common::{Author, Payload, Round};
use aptos_schemadb::{schema::Schema, ReadOptions, DB};
use aptos_types::transaction::{Transaction, Version};
use clap::Parser;
use std::{collections::HashSet, path::PathBuf};

#[derive(Parser)]
#[clap(about = "Print the version of each types of data.")]
pub struct Cmd {
    #[clap(long, value_parser)]
    pub db_dir: PathBuf,
}

impl Cmd {
    pub fn run(self) -> Vec<Transaction> {
        let consensus_db = ConsensusDB::new(self.db_dir.clone());
        let blocks = consensus_db.get_data().unwrap().2;
        let mut h = HashSet::new();
        for block in blocks {
            if block.id().to_hex()
                == "30b53c828fe7ca40adbaefb65b6b853befb31d74049c9ec89337e674c084f895"
            {
                let p = block.payload().unwrap();
                let proofs = match p {
                    Payload::InQuorumStore(proof_with_status) => proof_with_status.proofs.clone(),
                    _ => unreachable!(),
                };
                for proof in proofs {
                    h.insert(proof.info().clone().digest().clone());
                }
            }
        }
        let mut v = vec![];
        let quorum_store_db = QuorumStoreDB::new(self.db_dir.clone());
        let mut all_batches = quorum_store_db.get_all_batches().unwrap();
        for (id, mut val) in all_batches {
            if h.contains(&id) {
                let t = val.take_payload().unwrap();
                for x in t {
                    v.push(Transaction::UserTransaction(x));
                }
            }
        }
        v
    }
}
