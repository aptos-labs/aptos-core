// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::proof_of_store::ProofOfStore;
use aptos_types::{account_address::AccountAddress, transaction::SignedTransaction};
use serde::{Deserialize, Serialize};
use std::fmt;

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

/// The payload in block.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum Payload {
    DirectMempool(Vec<SignedTransaction>),
    InQuorumStore(Vec<ProofOfStore>),
    Empty,
}

impl Payload {
    pub fn new_empty() -> Self {
        Payload::Empty
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Payload::DirectMempool(txns) => txns.is_empty(),
            Payload::InQuorumStore(proofs) => proofs.is_empty(),
            Payload::Empty => true,
        }
    }
}


impl fmt::Display for Payload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Payload::DirectMempool(txns) => {
                write!(f, "InMemory txns: {}", txns.len())
            }
            Payload::InQuorumStore(_poavs) => todo!(),
            Payload::Empty => write!(f, "Empty payload"),
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
                    if let Payload::DirectMempool(txns) = payload {
                        for txn in txns {
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
            Payload::Empty => unreachable!(),
        }
    }
}

impl fmt::Display for PayloadFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PayloadFilter::DirectMempool(excluded_txns) => {
                let mut txns_str = "".to_string();
                for tx in excluded_txns.iter() {
                    txns_str += &format!("{} ", tx);
                }
                write!(f, "{}", txns_str)
            }
            PayloadFilter::InQuorumStore(_poavs) => todo!(),
        }
    }
}
