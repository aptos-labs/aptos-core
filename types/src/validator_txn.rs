// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{dkg::DKGTranscript, jwks};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub enum ValidatorTransaction {
    DummyTopic1(DummyValidatorTransaction),
    DKGResult(DKGTranscript),
    DummyTopic2(DummyValidatorTransaction),
    ObservedJWKUpdate(jwks::QuorumCertifiedUpdate),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct DummyValidatorTransaction {
    pub valid: bool,
    #[serde(with = "serde_bytes")]
    pub payload: Vec<u8>,
}

impl ValidatorTransaction {
    #[cfg(any(test, feature = "fuzzing"))]
    pub fn dummy1(payload: Vec<u8>) -> Self {
        Self::DummyTopic1(DummyValidatorTransaction {
            valid: true,
            payload,
        })
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn dummy2(payload: Vec<u8>) -> Self {
        Self::DummyTopic2(DummyValidatorTransaction {
            valid: true,
            payload,
        })
    }

    pub fn size_in_bytes(&self) -> usize {
        bcs::serialized_size(self).unwrap()
    }

    pub fn topic(&self) -> Topic {
        match self {
            ValidatorTransaction::DummyTopic1(_) => Topic::DUMMY1,
            ValidatorTransaction::DKGResult(_) => Topic::DKG,
            ValidatorTransaction::DummyTopic2(_) => Topic::DUMMY2,
            ValidatorTransaction::ObservedJWKUpdate(update) => {
                Topic::JWK_CONSENSUS(update.update.issuer.clone())
            },
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[allow(non_camel_case_types)]
pub enum Topic {
    DKG,
    JWK_CONSENSUS(jwks::Issuer),
    DUMMY1,
    DUMMY2,
}
