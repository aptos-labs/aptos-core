// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#[cfg(any(test, feature = "fuzzing"))]
use crate::dkg::DKGTranscriptMetadata;
use crate::{dkg::DKGTranscript, jwks};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
#[cfg(any(test, feature = "fuzzing"))]
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub enum ValidatorTransaction {
    DKGResult(DKGTranscript),
    ObservedJWKUpdate(jwks::QuorumCertifiedUpdate),
}

impl ValidatorTransaction {
    #[cfg(any(test, feature = "fuzzing"))]
    pub fn dummy(payload: Vec<u8>) -> Self {
        Self::DKGResult(DKGTranscript {
            metadata: DKGTranscriptMetadata {
                epoch: 999,
                author: AccountAddress::ZERO,
            },
            transcript_bytes: payload,
        })
    }

    pub fn size_in_bytes(&self) -> usize {
        bcs::serialized_size(self).unwrap()
    }

    pub fn topic(&self) -> Topic {
        match self {
            ValidatorTransaction::DKGResult(_) => Topic::DKG,
            ValidatorTransaction::ObservedJWKUpdate(update) => {
                Topic::JWK_CONSENSUS(update.update.issuer.clone())
            },
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            ValidatorTransaction::DKGResult(_) => "validator_transaction__dkg_result",
            ValidatorTransaction::ObservedJWKUpdate(_) => {
                "validator_transaction__observed_jwk_update"
            },
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[allow(non_camel_case_types)]
pub enum Topic {
    DKG,
    JWK_CONSENSUS(jwks::Issuer),
}
