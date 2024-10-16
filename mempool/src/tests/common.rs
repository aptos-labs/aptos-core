// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    core_mempool::{CoreMempool, TimelineState},
    network::{BroadcastPeerPriority, MempoolSyncMsg},
};
use anyhow::{format_err, Result};
use aptos_compression::client::CompressionClient;
use aptos_config::config::{NodeConfig, MAX_APPLICATION_MESSAGE_SIZE};
use aptos_consensus_types::common::{TransactionInProgress, TransactionSummary};
use aptos_crypto::{ed25519::Ed25519PrivateKey, PrivateKey, Uniform};
use aptos_types::{
    account_address::AccountAddress,
    chain_id::ChainId,
    mempool_status::MempoolStatusCode,
    transaction::{RawTransaction, Script, SignedTransaction, TransactionArgument},
};
use once_cell::sync::Lazy;
use rand::{rngs::StdRng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub(crate) fn setup_mempool() -> (CoreMempool, ConsensusMock) {
    let mut config = NodeConfig::generate_random_config();
    config.mempool.broadcast_buckets = vec![0];
    (CoreMempool::new(&config), ConsensusMock::new())
}

pub(crate) fn setup_mempool_with_broadcast_buckets(
    buckets: Vec<u64>,
) -> (CoreMempool, ConsensusMock) {
    let mut config = NodeConfig::generate_random_config();
    config.mempool.broadcast_buckets = buckets;
    (CoreMempool::new(&config), ConsensusMock::new())
}

static ACCOUNTS: Lazy<Vec<AccountAddress>> = Lazy::new(|| {
    vec![
        AccountAddress::random(),
        AccountAddress::random(),
        AccountAddress::random(),
        AccountAddress::random(),
    ]
});

static SMALL_SCRIPT: Lazy<Script> = Lazy::new(|| Script::new(vec![], vec![], vec![]));

static LARGE_SCRIPT: Lazy<Script> = Lazy::new(|| {
    let mut args = vec![];
    for _ in 0..200 {
        args.push(TransactionArgument::Address(AccountAddress::random()));
    }
    Script::new(vec![], vec![], args)
});

static HUGE_SCRIPT: Lazy<Script> = Lazy::new(|| {
    let mut args = vec![];
    for _ in 0..200_000 {
        args.push(TransactionArgument::Address(AccountAddress::random()));
    }
    Script::new(vec![], vec![], args)
});

#[derive(Clone, Serialize, Deserialize)]
pub struct TestTransaction {
    pub(crate) address: AccountAddress,
    pub(crate) sequence_number: u64,
    pub(crate) gas_price: u64,
    pub(crate) account_seqno: u64,
    pub(crate) script: Option<Script>,
}

impl TestTransaction {
    pub(crate) fn new(address: usize, sequence_number: u64, gas_price: u64) -> Self {
        Self {
            address: TestTransaction::get_address(address),
            sequence_number,
            gas_price,
            account_seqno: 0,
            script: None,
        }
    }

    pub(crate) fn new_with_large_script(
        address: usize,
        sequence_number: u64,
        gas_price: u64,
    ) -> Self {
        Self {
            address: TestTransaction::get_address(address),
            sequence_number,
            gas_price,
            account_seqno: 0,
            script: Some(LARGE_SCRIPT.clone()),
        }
    }

    pub(crate) fn new_with_huge_script(
        address: usize,
        sequence_number: u64,
        gas_price: u64,
    ) -> Self {
        Self {
            address: TestTransaction::get_address(address),
            sequence_number,
            gas_price,
            account_seqno: 0,
            script: Some(HUGE_SCRIPT.clone()),
        }
    }

    pub(crate) fn new_with_address(
        address: AccountAddress,
        sequence_number: u64,
        gas_price: u64,
    ) -> Self {
        Self {
            address,
            sequence_number,
            gas_price,
            account_seqno: 0,
            script: None,
        }
    }

    pub(crate) fn make_signed_transaction_with_expiration_time(
        &self,
        exp_timestamp_secs: u64,
    ) -> SignedTransaction {
        self.make_signed_transaction_impl(100, exp_timestamp_secs)
    }

    pub(crate) fn make_signed_transaction_with_max_gas_amount(
        &self,
        max_gas_amount: u64,
    ) -> SignedTransaction {
        self.make_signed_transaction_impl(max_gas_amount, u64::MAX)
    }

    pub(crate) fn make_signed_transaction(&self) -> SignedTransaction {
        self.make_signed_transaction_impl(100, u64::MAX)
    }

    fn make_signed_transaction_impl(
        &self,
        max_gas_amount: u64,
        exp_timestamp_secs: u64,
    ) -> SignedTransaction {
        let raw_txn = RawTransaction::new_script(
            self.address,
            self.sequence_number,
            self.script.clone().unwrap_or(SMALL_SCRIPT.clone()),
            max_gas_amount,
            self.gas_price,
            exp_timestamp_secs,
            ChainId::test(),
        );
        let mut seed: [u8; 32] = [0u8; 32];
        seed[..4].copy_from_slice(&[1, 2, 3, 4]);
        let mut rng: StdRng = StdRng::from_seed(seed);
        let privkey = Ed25519PrivateKey::generate(&mut rng);
        raw_txn
            .sign(&privkey, privkey.public_key())
            .expect("Failed to sign raw transaction.")
            .into_inner()
    }

    pub(crate) fn get_address(address: usize) -> AccountAddress {
        ACCOUNTS[address]
    }
}

pub(crate) fn add_txns_to_mempool(
    pool: &mut CoreMempool,
    txns: Vec<TestTransaction>,
) -> Vec<SignedTransaction> {
    let mut transactions = vec![];
    for transaction in txns {
        let txn = transaction.make_signed_transaction();
        pool.add_txn(
            txn.clone(),
            txn.gas_unit_price(),
            transaction.account_seqno,
            TimelineState::NotReady,
            false,
            None,
            Some(BroadcastPeerPriority::Primary),
        );
        transactions.push(txn);
    }
    transactions
}

pub(crate) fn txn_bytes_len(transaction: TestTransaction) -> u64 {
    let txn = transaction.make_signed_transaction();
    txn.txn_bytes_len() as u64
}

pub(crate) fn add_txn(
    pool: &mut CoreMempool,
    transaction: TestTransaction,
) -> Result<SignedTransaction> {
    let txn = transaction.make_signed_transaction();
    add_signed_txn(pool, txn.clone())?;
    Ok(txn)
}

pub(crate) fn add_signed_txn(pool: &mut CoreMempool, transaction: SignedTransaction) -> Result<()> {
    match pool
        .add_txn(
            transaction.clone(),
            transaction.gas_unit_price(),
            0,
            TimelineState::NotReady,
            false,
            None,
            Some(BroadcastPeerPriority::Primary),
        )
        .code
    {
        MempoolStatusCode::Accepted => Ok(()),
        _ => Err(format_err!("insertion failure")),
    }
}

pub(crate) fn batch_add_signed_txn(
    pool: &mut CoreMempool,
    transactions: Vec<SignedTransaction>,
) -> Result<()> {
    for txn in transactions.into_iter() {
        add_signed_txn(pool, txn)?
    }
    Ok(())
}

// Helper struct that keeps state between `.get_block` calls. Imitates work of Consensus.
pub struct ConsensusMock(BTreeMap<TransactionSummary, TransactionInProgress>);

impl ConsensusMock {
    pub(crate) fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub(crate) fn get_block(
        &mut self,
        mempool: &mut CoreMempool,
        max_txns: u64,
        max_bytes: u64,
    ) -> Vec<SignedTransaction> {
        let block = mempool.get_batch(max_txns, max_bytes, true, self.0.clone());
        block.iter().for_each(|t| {
            let txn_summary =
                TransactionSummary::new(t.sender(), t.sequence_number(), t.committed_hash());
            let txn_info = TransactionInProgress::new(t.gas_unit_price());
            self.0.insert(txn_summary, txn_info);
        });
        block
    }
}

/// Decompresses and deserializes the raw message bytes into a message struct
pub fn decompress_and_deserialize(message_bytes: &Vec<u8>) -> MempoolSyncMsg {
    bcs::from_bytes(
        &aptos_compression::decompress(
            message_bytes,
            CompressionClient::Mempool,
            MAX_APPLICATION_MESSAGE_SIZE,
        )
        .unwrap(),
    )
    .unwrap()
}
