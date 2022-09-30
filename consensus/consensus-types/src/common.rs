// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::HashValue;
use aptos_types::{account_address::AccountAddress, transaction::SignedTransaction};
use rayon::prelude::*;
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

#[derive(Clone)]
pub struct RejectedTransactionSummary {
    pub sender: AccountAddress,
    pub sequence_number: u64,
    pub hash: HashValue,
}

/// The payload in block.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum Payload {
    DirectMempool(Vec<SignedTransaction>),
}

impl Payload {
    pub fn empty() -> Self {
        Payload::DirectMempool(Vec::new())
    }

    pub fn len(&self) -> usize {
        match self {
            Payload::DirectMempool(txns) => txns.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Payload::DirectMempool(txns) => txns.is_empty(),
        }
    }

    /// This is computationally expensive on the first call
    pub fn size(&self) -> usize {
        match self {
            Payload::DirectMempool(txns) => txns
                .par_iter()
                .with_min_len(100)
                .map(|txn| txn.raw_txn_bytes_len())
                .sum(),
        }
    }
}

// TODO: What I really want is an iterator that isn't necessarily a vector (e.g., read lazily from RocksDB). This doesn't seem like the way.
impl IntoIterator for Payload {
    type Item = SignedTransaction;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Payload::DirectMempool(txns) => txns.into_iter(),
        }
    }
}

impl fmt::Display for Payload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Payload::DirectMempool(txns) => {
                write!(f, "InMemory txns: {}", txns.len())
            }
        }
    }
}

/// The payload to filter.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum PayloadFilter {
    DirectMempool(Vec<TransactionSummary>),
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
                    let Payload::DirectMempool(txns) = payload;
                    for txn in txns {
                        exclude_txns.push(TransactionSummary {
                            sender: txn.sender(),
                            sequence_number: txn.sequence_number(),
                        });
                    }
                }
                PayloadFilter::DirectMempool(exclude_txns)
            }
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
        }
    }
}
