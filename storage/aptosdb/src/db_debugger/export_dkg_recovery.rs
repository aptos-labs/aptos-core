// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::AptosDB;
use anyhow::{ensure, Context};
use aptos_config::config::{StorageConfig, StorageDirPaths, NO_OP_STORAGE_PRUNER_CONFIG};
use aptos_storage_interface::{DbReader, Result};
use aptos_types::{
    dkg::{
        chunky_dkg::ChunkyDKGState,
        recovery::{DkgRecoveryBundle, DkgResourceProof, MAX_DKG_RECOVERY_BUNDLE_BYTES},
        DKGState,
    },
    on_chain_config::OnChainConfig,
    state_store::state_key::StateKey,
};
use clap::Parser;
use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Parser)]
#[clap(about = "Export authenticated public DKG resources from a read-only donor DB checkpoint")]
pub struct Cmd {
    /// A consistent DB checkpoint retaining state-KV, state-Merkle and ledger
    /// proof history at the epoch boundary. Do not point at a running node's DB.
    #[clap(long, value_parser)]
    db_dir: PathBuf,
    /// The epoch to recover (not its preceding dealer epoch).
    #[clap(long)]
    epoch: u64,
    /// A new output file. Existing files are never overwritten.
    #[clap(long, value_parser)]
    output: PathBuf,
}

impl Cmd {
    pub fn run(self) -> Result<()> {
        Ok(self.run_inner()?)
    }

    fn run_inner(self) -> anyhow::Result<()> {
        let mut config = StorageConfig::default();
        config.hot_state_config.delete_on_restart = false;
        let db = AptosDB::open(
            StorageDirPaths::from_path(&self.db_dir),
            true, // readonly: never prune, restore, or write into the donor DB
            NO_OP_STORAGE_PRUNER_CONFIG,
            config.rocksdb_configs,
            config.buffered_state_target_items,
            config.max_num_nodes_per_lru_cache_shard,
            None,
            config.hot_state_config,
        )?;
        let bundle = export_bundle(&db, self.epoch)?;
        write_bundle(&bundle, &self.output)?;
        println!(
            "Exported verified public DKG recovery bundle for epoch {}",
            self.epoch
        );
        Ok(())
    }
}

fn write_bundle(bundle: &DkgRecoveryBundle, output: &Path) -> anyhow::Result<()> {
    let bytes = bcs::to_bytes(bundle)?;
    ensure!(
        bytes.len() as u64 <= MAX_DKG_RECOVERY_BUNDLE_BYTES,
        "DKG recovery bundle exceeds size limit"
    );
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    Ok(())
}

fn export_bundle(db: &dyn DbReader, epoch: u64) -> anyhow::Result<DkgRecoveryBundle> {
    let previous_epoch = epoch.checked_sub(1).context("Cannot recover epoch zero")?;
    let proof = db.get_epoch_ending_ledger_infos(previous_epoch, epoch)?;
    ensure!(
        proof.ledger_info_with_sigs.len() == 1,
        "Expected one epoch-ending ledger info"
    );
    let boundary = proof.ledger_info_with_sigs[0].ledger_info();
    let version = boundary.version();
    let resource = |key| -> anyhow::Result<_> {
        let (value, proof) = db
            .get_state_value_with_proof_by_version(&key, version)
            .context(
                "Donor must retain state-KV AND state-Merkle proof history at the epoch boundary",
            )?;
        Ok(DkgResourceProof { value, proof })
    };
    let bundle = DkgRecoveryBundle::V1 {
        epoch,
        version,
        transaction: db
            .get_transaction_by_version(version, version, false)?
            .proof,
        randomness: resource(StateKey::on_chain_config::<DKGState>()?)?,
        chunky: resource(StateKey::on_chain_config::<ChunkyDKGState>()?)?,
    };
    bundle.verify(epoch, boundary)?;
    // Detect unusable/corrupt Move encodings on the donor too. This is not a
    // replacement for the recipient's existing epoch/transcript/key checks.
    if let Some(value) = bundle.verified_value::<DKGState>(epoch, boundary)? {
        DKGState::deserialize_into_config(value.bytes())?;
    }
    if let Some(value) = bundle.verified_value::<ChunkyDKGState>(epoch, boundary)? {
        ChunkyDKGState::deserialize_into_config(value.bytes())?;
    }
    Ok(bundle)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use aptos_crypto::{hash::CryptoHash, HashValue};
    use aptos_storage_interface::errors::AptosDbError;
    use aptos_types::{
        aggregate_signature::AggregateSignature,
        block_info::BlockInfo,
        epoch_change::EpochChangeProof,
        epoch_state::EpochState,
        ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
        proof::{
            SparseMerkleLeafNode, SparseMerkleProof, TransactionAccumulatorProof,
            TransactionInfoWithProof,
        },
        state_store::state_value::StateValue,
        transaction::{
            ExecutionStatus, Transaction, TransactionInfo, TransactionWithProof, Version,
        },
        validator_verifier::ValidatorVerifier,
    };

    struct Donor {
        boundary: LedgerInfo,
        info: TransactionInfo,
        value: StateValue,
        leaf: SparseMerkleLeafNode,
        fail_history: bool,
        corrupt_value: bool,
    }

    impl Donor {
        fn new() -> Self {
            let value = StateValue::from(bcs::to_bytes(&DKGState::default()).unwrap());
            let leaf = SparseMerkleLeafNode::new(
                *StateKey::on_chain_config::<DKGState>()
                    .unwrap()
                    .crypto_hash_ref(),
                value.hash(),
            );
            let info =
                TransactionInfo::new_placeholder(0, Some(leaf.hash()), ExecutionStatus::Success);
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
            Self {
                boundary,
                info,
                value,
                leaf,
                fail_history: false,
                corrupt_value: false,
            }
        }
    }

    impl DbReader for Donor {
        fn get_epoch_ending_ledger_infos(&self, start: u64, end: u64) -> Result<EpochChangeProof> {
            assert_eq!((start, end), (9, 10));
            Ok(EpochChangeProof::new(
                vec![LedgerInfoWithSignatures::new(
                    self.boundary.clone(),
                    AggregateSignature::empty(),
                )],
                false,
            ))
        }

        fn get_transaction_by_version(
            &self,
            version: Version,
            ledger_version: Version,
            fetch_events: bool,
        ) -> Result<TransactionWithProof> {
            assert_eq!((version, ledger_version, fetch_events), (0, 0, false));
            Ok(TransactionWithProof::new(
                0,
                Transaction::StateCheckpoint(HashValue::zero()),
                None,
                TransactionInfoWithProof::new(
                    TransactionAccumulatorProof::new(vec![]),
                    self.info.clone(),
                ),
            ))
        }

        fn get_state_value_with_proof_by_version(
            &self,
            key: &StateKey,
            version: Version,
        ) -> Result<(Option<StateValue>, SparseMerkleProof)> {
            assert_eq!(version, 0);
            if self.fail_history {
                return Err(AptosDbError::Other("history pruned".into()));
            }
            let value = if *key == StateKey::on_chain_config::<DKGState>().unwrap() {
                Some(
                    if self.corrupt_value {
                        StateValue::from(vec![0])
                    } else {
                        self.value.clone()
                    },
                )
            } else {
                assert_eq!(*key, StateKey::on_chain_config::<ChunkyDKGState>().unwrap());
                None
            };
            Ok((value, SparseMerkleProof::new(Some(self.leaf), vec![])))
        }
    }

    #[test]
    fn exported_bundle_roundtrips_and_authenticates_on_recipient() {
        let donor = Donor::new();
        let exported = export_bundle(&donor, 10).unwrap();
        let bytes = bcs::to_bytes(&exported).unwrap();
        let received = DkgRecoveryBundle::from_reader(bytes.as_slice()).unwrap();
        // In production this ledger info comes from the recipient's own DB.
        let value = received
            .verified_value::<DKGState>(10, &donor.boundary)
            .unwrap()
            .unwrap();
        assert_eq!(value, donor.value);
        assert!(received
            .verified_value::<ChunkyDKGState>(10, &donor.boundary)
            .unwrap()
            .is_none());
    }

    #[test]
    fn output_is_create_new_and_can_be_imported() {
        let donor = Donor::new();
        let bundle = export_bundle(&donor, 10).unwrap();
        let output = aptos_temppath::TempPath::new();
        write_bundle(&bundle, output.path()).unwrap();
        let bytes = std::fs::read(output.path()).unwrap();
        DkgRecoveryBundle::from_reader(bytes.as_slice())
            .unwrap()
            .verify(10, &donor.boundary)
            .unwrap();
        assert!(write_bundle(&bundle, output.path()).is_err());
        assert_eq!(std::fs::read(output.path()).unwrap(), bytes);
    }

    #[test]
    fn exporter_rejects_unavailable_history_and_invalid_proofs() {
        let mut donor = Donor::new();
        donor.fail_history = true;
        assert!(export_bundle(&donor, 10).is_err());
        donor.fail_history = false;
        donor.corrupt_value = true;
        assert!(export_bundle(&donor, 10).is_err());
        assert!(export_bundle(&donor, 0).is_err());
    }
}
