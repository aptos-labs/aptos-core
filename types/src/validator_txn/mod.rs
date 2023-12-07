// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use crate::dkg::{DKGAggNode, DKGPvssConfig};
use crate::validator_verifier::ValidatorVerifier;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub enum ValidatorTransaction {
    DummyTopic(DummyValidatorTransaction),
    DKGAggregatedTranscript{
        agg_node: DKGAggNode,
        pvss_config: DKGPvssConfig,
        validator_verifier: ValidatorVerifier,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct DummyValidatorTransaction {
    #[serde(with = "serde_bytes")]
    pub payload: Vec<u8>,
}

impl ValidatorTransaction {
    pub fn dummy(payload: Vec<u8>) -> Self {
        Self::DummyTopic(DummyValidatorTransaction { payload })
    }

    pub fn size_in_bytes(&self) -> usize {
        match self {
            ValidatorTransaction::DummyTopic(txn) => txn.payload.len(),
            ValidatorTransaction::DKGAggregatedTranscript{..} => 1, // DKG todo: real
        }
    }
}

pub mod pool;
