// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    schema::state_value_by_key_hash::StateValueByKeyHashSchema, state_kv_db::StateKvDb, AptosDB,
};
use aptos_config::config::{RocksdbConfig, StorageDirPaths};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_db_indexer::db_ops::open_internal_indexer_db;
use aptos_db_indexer_schemas::schema::{
    event_by_key::EventByKeySchema, event_by_version::EventByVersionSchema,
    ordered_transaction_by_account::OrderedTransactionByAccountSchema, state_keys::StateKeysSchema,
};
use aptos_schemadb::{ReadOptions, DB};
use aptos_storage_interface::{DbReader, Result};
use aptos_types::{
    contract_event::ContractEvent,
    event::EventKey,
    transaction::{Transaction::UserTransaction, TransactionListWithProofV2},
};
use rayon::{
    iter::{IntoParallelIterator, ParallelIterator},
    ThreadPoolBuilder,
};
use std::{cmp, collections::HashSet, path::Path};
const SAMPLE_RATE: usize = 500_000;
use clap::Parser;

#[derive(Parser, Debug)]
pub struct ValidationArgs {
    #[clap(short, long)]
    pub db_root_path: String,

    #[clap(short, long)]
    pub internal_indexer_db_path: String,

    #[clap(short, long)]
    pub target_version: u64,
}
#[derive(clap::Subcommand)]
pub enum Cmd {
    ValidateIndexerDB(ValidationArgs),
}

impl Cmd {
    pub fn run(&self) -> Result<()> {
        match self {
            Cmd::ValidateIndexerDB(args) => validate_db_data(
                Path::new(args.db_root_path.as_str()),
                Path::new(&args.internal_indexer_db_path.as_str()),
                args.target_version,
            ),
        }
    }
}

pub fn validate_db_data(
    db_root_path: &Path,
    internal_indexer_db_path: &Path,
    mut target_ledger_version: u64,
) -> Result<()> {
    let num_threads = 30;
    ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()
        .unwrap();
    let internal_db =
        open_internal_indexer_db(internal_indexer_db_path, &RocksdbConfig::default())?;

    verify_state_kvs(db_root_path, &internal_db, target_ledger_version)?;

    let aptos_db = AptosDB::new_for_test_with_sharding(db_root_path, 1000000);
    let batch_size = 20_000;
    let start_version = aptos_db.get_first_txn_version()?.unwrap();
    target_ledger_version = std::cmp::min(
        aptos_db.get_synced_version()?.unwrap(),
        target_ledger_version,
    );
    assert!(
        start_version < target_ledger_version,
        "{}, {}",
        start_version,
        target_ledger_version
    );
    println!(
        "Validating events and transactions {}, {}",
        start_version, target_ledger_version
    );

    // Calculate ranges and split into chunks
    let ranges: Vec<(u64, u64)> = (start_version..target_ledger_version)
        .step_by(batch_size as usize)
        .map(|start| {
            let end = cmp::min(start + batch_size, target_ledger_version);
            (start, end)
        })
        .collect();

    // Process each chunk in parallel
    ranges.into_par_iter().for_each(|(start, end)| {
        let num_of_txns = end - start;
        println!("Validating transactions from {} to {}", start, end);
        let txns = aptos_db
            .get_transactions(start, num_of_txns, target_ledger_version, true)
            .unwrap();
        verify_batch_txn_events(&txns, &internal_db, start)
            .unwrap_or_else(|_| panic!("{}, {} failed to verify", start, end));
        assert_eq!(
            txns.get_transaction_list_with_proof().transactions.len() as u64,
            num_of_txns
        );
    });

    Ok(())
}

pub fn verify_state_kvs(
    db_root_path: &Path,
    internal_db: &DB,
    target_ledger_version: u64,
) -> Result<()> {
    println!("Validating db statekeys");
    let storage_dir = StorageDirPaths::from_path(db_root_path);
    let state_kv_db = StateKvDb::open_sharded(&storage_dir, RocksdbConfig::default(), false)?;

    //read all statekeys from internal db and store them in mem
    let mut all_internal_keys = HashSet::new();
    let mut iter = internal_db.iter::<StateKeysSchema>()?;
    iter.seek_to_first();
    for (key_ind, state_key_res) in iter.enumerate() {
        let state_key = state_key_res?.0;
        let state_key_hash = state_key.hash();
        all_internal_keys.insert(state_key_hash);
        if key_ind % 10_000_000 == 0 {
            println!("Processed {} keys", key_ind);
        }
    }
    println!(
        "Number of state keys in internal db: {}",
        all_internal_keys.len()
    );
    for shard_id in 0..16 {
        let shard = state_kv_db.db_shard(shard_id);
        println!("Validating state_kv for shard {}", shard_id);
        verify_state_kv(shard, &all_internal_keys, target_ledger_version)?;
    }
    Ok(())
}

pub fn verify_batch_txn_events(
    txns: &TransactionListWithProofV2,
    internal_db: &DB,
    start_version: u64,
) -> Result<()> {
    verify_transactions(txns, internal_db, start_version)?;
    verify_events(txns, internal_db, start_version)
}

fn verify_state_kv(
    shard: &DB,
    all_internal_keys: &HashSet<HashValue>,
    target_ledger_version: u64,
) -> Result<()> {
    let read_opts = ReadOptions::default();
    let mut iter = shard.iter_with_opts::<StateValueByKeyHashSchema>(read_opts)?;
    // print a message every 10k keys
    let mut counter = 0;
    iter.seek_to_first();
    let mut missing_keys = 0;
    for value in iter {
        let (state_key_hash, version) = value?.0;
        if version > target_ledger_version {
            continue;
        }
        // check if the state key hash is present in the internal db
        if !all_internal_keys.contains(&state_key_hash) {
            missing_keys += 1;
            println!(
                "State key hash not found in internal db: {:?}, version: {}",
                state_key_hash, version
            );
        }
        counter += 1;
        if counter as usize % SAMPLE_RATE == 0 {
            println!(
                "Processed {} keys, the current sample is {} at version {}",
                counter, state_key_hash, version
            );
        }
    }
    println!("Number of missing keys: {}", missing_keys);
    Ok(())
}

fn verify_transactions(
    transaction_list: &TransactionListWithProofV2,
    internal_indexer_db: &DB,
    start_version: u64,
) -> Result<()> {
    for (idx, txn) in transaction_list
        .get_transaction_list_with_proof()
        .transactions
        .iter()
        .enumerate()
    {
        match txn {
            UserTransaction(signed_transaction) => {
                let key = (
                    signed_transaction.sender(),
                    signed_transaction.sequence_number(),
                );
                match internal_indexer_db.get::<OrderedTransactionByAccountSchema>(&key)? {
                    Some(version) => {
                        assert_eq!(version, start_version + idx as u64);
                        if idx + start_version as usize % SAMPLE_RATE == 0 {
                            println!("Processed {} at {:?}", idx + start_version as usize, key);
                        }
                    },
                    None => {
                        panic!("Transaction not found in internal indexer db: {:?}", key);
                    },
                }
            },
            _ => continue,
        }
    }
    Ok(())
}

fn verify_event_by_key(
    event_key: &EventKey,
    seq_num: u64,
    internal_indexer_db: &DB,
    expected_idx: usize,
    expected_version: u64,
) -> Result<()> {
    match internal_indexer_db.get::<EventByKeySchema>(&(*event_key, seq_num)) {
        Ok(None) => {
            panic!("Event not found in internal indexer db: {:?}", event_key);
        },
        Err(e) => {
            panic!("Error while fetching event: {:?}", e);
        },
        Ok(Some((version, idx))) => {
            assert!(idx as usize == expected_idx && version == expected_version);
            if version as usize % SAMPLE_RATE == 0 {
                println!(
                    "Processed {} at {:?}, {:?}",
                    version, event_key, expected_idx
                );
            }
        },
    }
    Ok(())
}

fn verify_event_by_version(
    event_key: &EventKey,
    seq_num: u64,
    internal_indexer_db: &DB,
    version: u64,
    expected_idx: usize,
) -> Result<()> {
    match internal_indexer_db.get::<EventByVersionSchema>(&(*event_key, version, seq_num)) {
        Ok(None) => {
            panic!("Event not found in internal indexer db: {:?}", event_key);
        },
        Err(e) => {
            panic!("Error while fetching event: {:?}", e);
        },
        Ok(Some(idx)) => {
            assert!(idx as usize == expected_idx);
        },
    }
    Ok(())
}

fn verify_events(
    transaction_list: &TransactionListWithProofV2,
    internal_indexer_db: &DB,
    start_version: u64,
) -> Result<()> {
    let mut version = start_version;
    match &transaction_list.get_transaction_list_with_proof().events {
        None => {
            return Ok(());
        },
        Some(event_vec) => {
            for events in event_vec {
                for (idx, event) in events.iter().enumerate() {
                    match event {
                        ContractEvent::V1(event) => {
                            let seq_num = event.sequence_number();
                            let event_key = event.key();
                            verify_event_by_version(
                                event_key,
                                seq_num,
                                internal_indexer_db,
                                version,
                                idx,
                            )?;
                            verify_event_by_key(
                                event_key,
                                seq_num,
                                internal_indexer_db,
                                idx,
                                version,
                            )?;
                        },
                        _ => continue,
                    }
                }
                version += 1;
            }
        },
    }
    Ok(())
}
