// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#[cfg(any(test, feature = "fuzzing"))]
use crate::dkg::DKGTranscriptMetadata;
use crate::{
    dkg::{chunky_dkg::CertifiedChunkyDKGOutput, DKGTranscript},
    jwks, trust_observation,
    validator_verifier::ValidatorVerifier,
};
use anyhow::Context;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
#[cfg(any(test, feature = "fuzzing"))]
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub enum ValidatorTransaction {
    DKGResult(DKGTranscript),
    ObservedJWKUpdate(jwks::QuorumCertifiedUpdate),
    ChunkyDKGResult(CertifiedChunkyDKGOutput),
    ObservedTrustUpdate(trust_observation::QuorumCertifiedUpdate),
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

    pub fn type_name(&self) -> &'static str {
        match self {
            ValidatorTransaction::DKGResult(_) => "validator_transaction__dkg_result",
            ValidatorTransaction::ObservedJWKUpdate(_) => {
                "validator_transaction__observed_jwk_update"
            },
            ValidatorTransaction::ChunkyDKGResult(_) => "validator_transaction__chunky_dkg_result",
            ValidatorTransaction::ObservedTrustUpdate(_) => {
                "validator_transaction__observed_trust_update"
            },
        }
    }

    pub fn verify(&self, verifier: &ValidatorVerifier) -> anyhow::Result<()> {
        match self {
            ValidatorTransaction::DKGResult(dkg_result) => dkg_result
                .verify(verifier)
                .context("DKGResult verification failed"),
            ValidatorTransaction::ObservedJWKUpdate(_) => Ok(()),
            ValidatorTransaction::ChunkyDKGResult(_) => {
                // TODO: Implement verification for ChunkyDKGResult
                Ok(())
            },
            ValidatorTransaction::ObservedTrustUpdate(update) => {
                let authors = update
                    .multi_sig
                    .get_signers_addresses(&verifier.get_ordered_account_addresses());
                verifier
                    .check_voting_power(authors.iter(), true)
                    .context("ObservedTrustUpdate lacks quorum voting power")?;
                verifier
                    .verify_multi_signatures(&update.update, &update.multi_sig)
                    .context("ObservedTrustUpdate quorum signature verification failed")
            },
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[allow(non_camel_case_types)]
pub enum Topic {
    DKG,
    JWK_CONSENSUS(jwks::Issuer),
    JWK_CONSENSUS_PER_KEY_MODE {
        issuer: jwks::Issuer,
        kid: jwks::KID,
    },
    ChunkyDKG,
    TRUST_OBSERVATION(trust_observation::TargetId),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aggregate_signature::AggregateSignature;

    #[test]
    fn observed_trust_update_is_appended_to_validator_transaction() {
        let txn =
            ValidatorTransaction::ObservedTrustUpdate(trust_observation::QuorumCertifiedUpdate {
                update: trust_observation::ObservedValue {
                    target_id: b"intel-root".to_vec(),
                    config_version: 1,
                    epoch: 7,
                    version: 1,
                    value: b"certificate".to_vec(),
                },
                multi_sig: AggregateSignature::empty(),
            });

        let bytes = bcs::to_bytes(&txn).unwrap();
        assert_eq!(bytes[0], 3); // Existing validator transaction BCS tags stay unchanged.
        assert_eq!(
            bcs::from_bytes::<ValidatorTransaction>(&bytes).unwrap(),
            txn
        );
        assert_eq!(
            txn.type_name(),
            "validator_transaction__observed_trust_update"
        );
    }
}
