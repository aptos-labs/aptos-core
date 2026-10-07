// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Portable, public evidence for DKG resources at an epoch boundary.
//!
//! The bundle supplies no trust anchor. Its recipient must obtain the epoch-ending
//! ledger info from its own authenticated storage, never from the bundle's donor.

use crate::{
    dkg::{chunky_dkg::ChunkyDKGState, DKGState},
    ledger_info::LedgerInfo,
    on_chain_config::OnChainConfig,
    proof::{SparseMerkleProof, TransactionInfoWithProof},
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::io::Read;

/// Bound local-file input before BCS decoding. No network requests are performed.
pub const MAX_DKG_RECOVERY_BUNDLE_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone, Serialize, Deserialize)]
pub struct DkgResourceProof {
    pub value: Option<StateValue>,
    pub proof: SparseMerkleProof,
}

impl DkgResourceProof {
    fn verify<T: OnChainConfig>(&self, root: aptos_crypto::HashValue) -> Result<()> {
        let key = StateKey::on_chain_config::<T>()?;
        self.proof
            .verify(root, *key.crypto_hash_ref(), self.value.as_ref())
    }
}

/// Versioned wire format containing exactly the two public on-chain DKG resources.
/// There are no validator keys, local shares, or arbitrary resource selectors.
#[derive(Clone, Serialize, Deserialize)]
pub enum DkgRecoveryBundle {
    V1 {
        epoch: u64,
        version: Version,
        transaction: TransactionInfoWithProof,
        randomness: DkgResourceProof,
        chunky: DkgResourceProof,
    },
}

impl DkgRecoveryBundle {
    pub fn from_reader(reader: impl Read) -> Result<Self> {
        let mut bytes = Vec::new();
        reader
            .take(MAX_DKG_RECOVERY_BUNDLE_BYTES + 1)
            .read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() as u64 <= MAX_DKG_RECOVERY_BUNDLE_BYTES,
            "DKG recovery bundle exceeds size limit"
        );
        bcs::from_bytes(&bytes).context("Invalid DKG recovery bundle encoding")
    }

    /// Verify every resource against the exact boundary authenticated locally.
    /// Verifying the transaction hash alone is insufficient: the checkpoint root
    /// must authenticate the complete StateValue (including storage metadata).
    pub fn verify(&self, epoch: u64, local_boundary: &LedgerInfo) -> Result<()> {
        let Self::V1 {
            epoch: bundle_epoch,
            version,
            transaction,
            randomness,
            chunky,
        } = self;
        ensure!(
            epoch.checked_sub(1) == Some(local_boundary.epoch())
                && local_boundary.ends_epoch()
                && local_boundary.next_block_epoch() == epoch,
            "Local ledger info does not begin requested epoch"
        );
        ensure!(
            *bundle_epoch == epoch && *version == local_boundary.version(),
            "DKG recovery bundle is for a different epoch boundary"
        );
        transaction
            .verify(local_boundary, *version)
            .context("Invalid epoch-boundary transaction proof")?;
        let root = transaction
            .transaction_info()
            .state_checkpoint_hash()
            .context("Epoch-boundary transaction has no state checkpoint")?;
        randomness
            .verify::<DKGState>(root)
            .context("Invalid randomness DKG resource proof")?;
        chunky
            .verify::<ChunkyDKGState>(root)
            .context("Invalid Chunky DKG resource proof")?;
        Ok(())
    }

    pub fn verified_value<T: OnChainConfig>(
        &self,
        epoch: u64,
        local_boundary: &LedgerInfo,
    ) -> Result<Option<StateValue>> {
        self.verify(epoch, local_boundary)?;
        let Self::V1 {
            randomness, chunky, ..
        } = self;
        let key = StateKey::on_chain_config::<T>()?;
        if key == StateKey::on_chain_config::<DKGState>()? {
            Ok(randomness.value.clone())
        } else {
            ensure!(
                key == StateKey::on_chain_config::<ChunkyDKGState>()?,
                "Unsupported DKG recovery resource"
            );
            Ok(chunky.value.clone())
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::{
        block_info::BlockInfo,
        epoch_state::EpochState,
        proof::{SparseMerkleInternalNode, SparseMerkleLeafNode, TransactionAccumulatorProof},
        transaction::{ExecutionStatus, TransactionInfo},
        validator_verifier::ValidatorVerifier,
    };
    use aptos_crypto::{
        hash::{CryptoHash, SPARSE_MERKLE_PLACEHOLDER_HASH},
        HashValue,
    };

    fn fixture() -> (DkgRecoveryBundle, LedgerInfo) {
        let value = StateValue::from(bcs::to_bytes(&DKGState::default()).unwrap());
        let key = StateKey::on_chain_config::<DKGState>().unwrap();
        let leaf = SparseMerkleLeafNode::new(*key.crypto_hash_ref(), value.hash());
        let transaction_info =
            TransactionInfo::new_placeholder(0, Some(leaf.hash()), ExecutionStatus::Success);
        let boundary = LedgerInfo::new(
            BlockInfo::new(
                9,
                1,
                HashValue::zero(),
                transaction_info.hash(),
                0,
                0,
                Some(EpochState::new(10, ValidatorVerifier::new(vec![]))),
            ),
            HashValue::zero(),
        );
        let bundle = DkgRecoveryBundle::V1 {
            epoch: 10,
            version: 0,
            transaction: TransactionInfoWithProof::new(
                TransactionAccumulatorProof::new(vec![]),
                transaction_info,
            ),
            randomness: DkgResourceProof {
                value: Some(value),
                proof: SparseMerkleProof::new(Some(leaf), vec![]),
            },
            chunky: DkgResourceProof {
                value: None,
                proof: SparseMerkleProof::new(Some(leaf), vec![]),
            },
        };
        (bundle, boundary)
    }

    #[test]
    fn authenticates_inclusion_and_absence_against_local_boundary() {
        let (bundle, boundary) = fixture();
        let bytes = bcs::to_bytes(&bundle).unwrap();
        let decoded = DkgRecoveryBundle::from_reader(bytes.as_slice()).unwrap();
        assert!(decoded
            .verified_value::<DKGState>(10, &boundary)
            .unwrap()
            .is_some());
        assert!(decoded
            .verified_value::<ChunkyDKGState>(10, &boundary)
            .unwrap()
            .is_none());
    }

    #[test]
    fn authenticates_both_resources_and_rejects_swapping_them() {
        let key1 = *StateKey::on_chain_config::<DKGState>()
            .unwrap()
            .crypto_hash_ref();
        let key2 = *StateKey::on_chain_config::<ChunkyDKGState>()
            .unwrap()
            .crypto_hash_ref();
        let value1 = StateValue::from(bcs::to_bytes(&DKGState::default()).unwrap());
        let value2 = StateValue::from(bcs::to_bytes(&ChunkyDKGState::default()).unwrap());
        let leaf1 = SparseMerkleLeafNode::new(key1, value1.hash());
        let leaf2 = SparseMerkleLeafNode::new(key2, value2.hash());
        let common = key1.common_prefix_bits_len(key2);
        let mut root = if key1.bit(common) {
            SparseMerkleInternalNode::new(leaf2.hash(), leaf1.hash()).hash()
        } else {
            SparseMerkleInternalNode::new(leaf1.hash(), leaf2.hash()).hash()
        };
        for depth in (0..common).rev() {
            root = if key1.bit(depth) {
                SparseMerkleInternalNode::new(*SPARSE_MERKLE_PLACEHOLDER_HASH, root).hash()
            } else {
                SparseMerkleInternalNode::new(root, *SPARSE_MERKLE_PLACEHOLDER_HASH).hash()
            };
        }
        let mut siblings1 = vec![*SPARSE_MERKLE_PLACEHOLDER_HASH; common];
        siblings1.push(leaf2.hash());
        let mut siblings2 = vec![*SPARSE_MERKLE_PLACEHOLDER_HASH; common];
        siblings2.push(leaf1.hash());
        let info = TransactionInfo::new_placeholder(0, Some(root), ExecutionStatus::Success);
        let boundary = LedgerInfo::new(
            BlockInfo::new(
                9,
                1,
                HashValue::zero(),
                info.hash(),
                0,
                0,
                Some(EpochState::new(10, ValidatorVerifier::new(vec![]))),
            ),
            HashValue::zero(),
        );
        let mut bundle = DkgRecoveryBundle::V1 {
            epoch: 10,
            version: 0,
            transaction: TransactionInfoWithProof::new(
                TransactionAccumulatorProof::new(vec![]),
                info,
            ),
            randomness: DkgResourceProof {
                value: Some(value1),
                proof: SparseMerkleProof::new(Some(leaf1), siblings1),
            },
            chunky: DkgResourceProof {
                value: Some(value2),
                proof: SparseMerkleProof::new(Some(leaf2), siblings2),
            },
        };
        bundle.verify(10, &boundary).unwrap();
        let DkgRecoveryBundle::V1 {
            randomness, chunky, ..
        } = &mut bundle;
        std::mem::swap(randomness, chunky);
        assert!(bundle.verify(10, &boundary).is_err());
    }

    #[test]
    fn rejects_tampered_epoch_version_transaction_value_and_proof() {
        for failure in 0..7 {
            let (mut bundle, boundary) = fixture();
            let DkgRecoveryBundle::V1 {
                epoch,
                version,
                transaction,
                randomness,
                chunky,
            } = &mut bundle;
            match failure {
                0 => *epoch += 1,
                1 => *version += 1,
                2 => {
                    transaction.transaction_info =
                        TransactionInfo::new_placeholder(1, None, ExecutionStatus::Success)
                },
                3 => randomness.value = Some(StateValue::from(vec![1])),
                4 => randomness.proof = SparseMerkleProof::new(None, vec![]),
                5 => chunky.value = randomness.value.clone(),
                6 => randomness.value = None,
                _ => unreachable!(),
            }
            assert!(bundle.verify(10, &boundary).is_err());
        }
        let (bundle, _) = fixture();
        let wrong_boundary = LedgerInfo::new(
            BlockInfo::new(
                9,
                1,
                HashValue::zero(),
                HashValue::zero(),
                0,
                0,
                Some(EpochState::new(10, ValidatorVerifier::new(vec![]))),
            ),
            HashValue::zero(),
        );
        assert!(bundle.verify(10, &wrong_boundary).is_err());
    }

    #[test]
    fn rejects_missing_checkpoint_even_with_valid_transaction_proof() {
        let (mut bundle, _) = fixture();
        let info = TransactionInfo::new_placeholder(0, None, ExecutionStatus::Success);
        let boundary = LedgerInfo::new(
            BlockInfo::new(
                9,
                1,
                HashValue::zero(),
                info.hash(),
                0,
                0,
                Some(EpochState::new(10, ValidatorVerifier::new(vec![]))),
            ),
            HashValue::zero(),
        );
        let DkgRecoveryBundle::V1 { transaction, .. } = &mut bundle;
        transaction.transaction_info = info;
        transaction.verify(&boundary, 0).unwrap();
        let error = bundle.verify(10, &boundary).unwrap_err();
        assert!(error.to_string().contains("no state checkpoint"));
    }

    #[test]
    fn cannot_select_non_dkg_resources() {
        let (bundle, boundary) = fixture();
        assert!(bundle
            .verified_value::<crate::chain_id::ChainId>(10, &boundary)
            .is_err());
    }

    #[test]
    fn rejects_untrusted_file_encoding_and_oversize_input() {
        let (bundle, _) = fixture();
        let mut bytes = bcs::to_bytes(&bundle).unwrap();
        bytes.push(0);
        assert!(DkgRecoveryBundle::from_reader(bytes.as_slice()).is_err());
        assert!(DkgRecoveryBundle::from_reader(&[255][..]).is_err());
        assert!(DkgRecoveryBundle::from_reader(
            std::io::repeat(0).take(MAX_DKG_RECOVERY_BUNDLE_BYTES + 1)
        )
        .is_err());
    }
}
