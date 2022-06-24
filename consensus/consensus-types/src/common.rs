// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::proof_of_store::ProofOfStore;
use aptos_crypto::HashValue;
use aptos_types::{account_address::AccountAddress, transaction::SignedTransaction};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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

    pub fn is_direct(&self) -> bool {
        match self {
            Payload::DirectMempool(_) => true,
            Payload::InQuorumStore(_) => false,
            Payload::Empty => false,
        }
    }
}

impl fmt::Display for Payload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Payload::DirectMempool(txns) => {
                write!(f, "InMemory txns: {}", txns.len())
            }
            Payload::InQuorumStore(poavs) => {
                write!(f, "InMemory poavs: {}", poavs.len())
            }
            Payload::Empty => write!(f, "Empty payload"),
        }
    }
}

/// The payload to filter.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum PayloadFilter {
    DirectMempool(Vec<TransactionSummary>),
    InQuorumStore(HashSet<HashValue>),
    //
    Empty,
}

impl From<&Vec<&Payload>> for PayloadFilter {
    fn from(exclude_payloads: &Vec<&Payload>) -> Self {
        if exclude_payloads.is_empty() {
            return PayloadFilter::Empty;
        }
        let direct_mode = exclude_payloads.iter().any(|payload| payload.is_direct());

        if direct_mode {
            let mut exclude_txns = Vec::new();
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
        } else {
            let mut exclude_proofs = HashSet::new();
            for payload in exclude_payloads {
                if let Payload::InQuorumStore(proofs) = payload {
                    for proof in proofs {
                        exclude_proofs.insert(proof.digest().clone());
                    }
                }
            }
            PayloadFilter::InQuorumStore(exclude_proofs)
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
            PayloadFilter::InQuorumStore(exclided_proofs) => {
                let mut txns_str = "".to_string();
                for proof in exclided_proofs.iter() {
                    txns_str += &format!("{} ", proof);
                }
                write!(f, "{}", txns_str)
            }
            PayloadFilter::Empty => {
                write!(f, "Empty filter")
            }
        }
    }
}
