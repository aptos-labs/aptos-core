// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::{account_address::AccountAddress, transaction::SignedTransaction};
use serde::{Deserialize, Serialize};
use std::{fmt, fmt::Write};

/// The round of a block is a consensus-internal counter, which starts with 0 and increases
/// monotonically. It is used for the protocol safety and liveness (please see the detailed
/// protocol description).
pub type Round = u64;
/// Author refers to the author's account address
pub type Author = AccountAddress;

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct TransactionSummary {
    pub sender: AccountAddress,
    pub sequence_number: u64,
}

impl fmt::Display for TransactionSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.sender, self.sequence_number,)
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct TransactionBatch {
    pub txns: Vec<SignedTransaction>,
    pub total_bytes: u64,
}

impl TransactionBatch {
    pub fn new_from_txns(txns: Vec<SignedTransaction>) -> Self {
        let total_bytes = txns.iter().map(|t| t.raw_txn_bytes_len() as u64).sum();
        Self { txns, total_bytes }
    }
}

/// The payload in block.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum Payload {
    DirectMempool(TransactionBatch),
    InQuorumStore(Vec<ProofOfStore>),
}

impl Payload {
    pub fn empty() -> Self {
        Payload::DirectMempool(TransactionBatch {
            txns: Vec::new(),
            total_bytes: 0,
        })
    }

    pub fn len(&self) -> usize {
        match self {
            Payload::DirectMempool(batch) => batch.txns.len(),
            Payload::InQuorumStore(_poavs) => todo!(),
        }
    }

    pub fn bytes(&self) -> u64 {
        match self {
            Payload::DirectMempool(batch) => batch.total_bytes,
            Payload::InQuorumStore(_poavs) => todo!(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Payload::DirectMempool(batch) => batch.txns.is_empty(),
            Payload::InQuorumStore(_poavs) => todo!(),
        }
    }
}

// TODO: What I really want is an iterator that isn't necessarily a vector (e.g., read lazily from RocksDB). This doesn't seem like the way.
impl IntoIterator for Payload {
    type Item = SignedTransaction;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Payload::DirectMempool(batch) => batch.txns.into_iter(),
            Payload::InQuorumStore(_poavs) => todo!(),
        }
    }
}

impl fmt::Display for Payload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Payload::DirectMempool(batch) => {
                write!(
                    f,
                    "InMemory txns: {}, bytes: {}",
                    batch.txns.len(),
                    batch.total_bytes
                )
            }
            Payload::InQuorumStore(_poavs) => todo!(),
        }
    }
}

/// The payload to filter.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum PayloadFilter {
    DirectMempool(Vec<TransactionSummary>),
    InQuorumStore(Vec<ProofOfStore>),
}

impl From<&Vec<&Payload>> for PayloadFilter {
    fn from(exclude_payloads: &Vec<&Payload>) -> Self {
        if exclude_payloads.is_empty() {
            return PayloadFilter::DirectMempool(vec![]);
        }
        match exclude_payloads.first().unwrap() {
            Payload::DirectMempool(_) => {
                let mut exclude_txns = vec![];
                for payload in exclude_payloads {
                    if let Payload::DirectMempool(batch) = payload {
                        for txn in &batch.txns {
                            exclude_txns.push(TransactionSummary {
                                sender: txn.sender(),
                                sequence_number: txn.sequence_number(),
                            });
                        }
                    }
                }
                PayloadFilter::DirectMempool(exclude_txns)
            }
            Payload::InQuorumStore(_) => todo!(),
        }
    }
}

impl fmt::Display for PayloadFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PayloadFilter::DirectMempool(excluded_txns) => {
                let mut txns_str = "".to_string();
                for tx in excluded_txns.iter() {
                    write!(txns_str, "{} ", tx)?;
                }
                write!(f, "{}", txns_str)
            }
            PayloadFilter::InQuorumStore(_poavs) => todo!(),
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ProofOfStore {
    // TODO: This is currently just a placeholder for a real ProofOfStore implementation.
    placeholder: u64,
}
