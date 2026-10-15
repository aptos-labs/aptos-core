// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::{ensure, Context, Result};
use aptos_logger::info;
use aptos_storage_interface::DbReader;
use aptos_types::{
    dkg::recovery::DkgRecoveryBundle, on_chain_config::OnChainConfig,
    state_store::state_key::StateKey,
};
use std::{fs::File, path::Path};

/// Restore a DKG resource overwritten by a session for a future epoch. Only
/// consensus's key configuration uses this snapshot; the DKG managers must
/// continue to see the latest state so they can finish the pending transition.
/// Uses state-KV history at the epoch boundary, or an explicitly configured
/// public proof bundle if that read fails. The bundle is verified against the
/// authenticated local epoch-ending ledger info. No historical state is written
/// into the DB. On error, the original state remains unchanged.
pub(super) fn recover_dkg_state<T: OnChainConfig + Default>(
    epoch: u64,
    state: &mut Result<T>,
    completed_dealer_epoch: impl Fn(&T) -> Option<u64>,
    db: &dyn DbReader,
    recovery_bundle_path: Option<&Path>,
) -> Result<()> {
    let Some(dealer_epoch) = state.as_ref().ok().and_then(&completed_dealer_epoch) else {
        return Ok(());
    };
    if dealer_epoch < epoch {
        // Preserve missing/stale sessions, including those left by a forced
        // epoch transition. The existing epoch checks decide whether to use them.
        return Ok(());
    }

    let previous_epoch = epoch.checked_sub(1).context("Cannot recover epoch zero")?;
    let proof = db
        .get_epoch_ending_ledger_infos(previous_epoch, epoch)
        .context("Failed to read the ledger info that began the current epoch")?;
    ensure!(
        proof.ledger_info_with_sigs.len() == 1,
        "Expected one epoch-ending ledger info for epoch {previous_epoch}"
    );
    let ledger_info = proof.ledger_info_with_sigs[0].ledger_info();
    ensure!(
        ledger_info.epoch() == previous_epoch
            && ledger_info.ends_epoch()
            && ledger_info.next_block_epoch() == epoch,
        "Ledger info does not begin epoch {epoch}"
    );

    let version = ledger_info.version();
    let value = db
        .get_state_value_by_version(&StateKey::on_chain_config::<T>()?, version)
        .or_else(|original_error| {
            let Some(path) = recovery_bundle_path else {
                return Err(anyhow::Error::from(original_error));
            };
            ensure!(
                std::fs::metadata(path)?.is_file(),
                "DKG recovery bundle must be a regular file"
            );
            let file = File::open(path).context("Cannot open configured DKG recovery bundle")?;
            ensure!(
                file.metadata()?.is_file(),
                "DKG recovery bundle must be a regular file"
            );
            let bundle = DkgRecoveryBundle::from_reader(file)?;
            bundle.verified_value::<T>(epoch, ledger_info)
        })
        .with_context(|| {
            format!(
                "Failed to read {} at epoch-start version {version}",
                T::CONFIG_ID
            )
        })?;
    // A resource that did not exist at the epoch boundary has no usable
    // session, just like the default DKG state at genesis.
    let recovered = value
        .map(|value| T::deserialize_into_config(value.bytes()))
        .transpose()?
        .unwrap_or_default();
    ensure!(
        completed_dealer_epoch(&recovered).is_none_or(|dealer| dealer < epoch),
        "Epoch-start {} still contains a session for a future epoch",
        T::CONFIG_ID
    );

    info!(
        epoch = epoch,
        version = version,
        config = %T::CONFIG_ID,
        "Recovered current-epoch DKG state from epoch boundary"
    );
    *state = Ok(recovered);
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use aptos_crypto::{hash::CryptoHash, HashValue};
    use aptos_storage_interface::{errors::AptosDbError, Result as StorageResult};
    use aptos_temppath::TempPath;
    use aptos_types::{
        aggregate_signature::AggregateSignature,
        block_info::BlockInfo,
        dkg::{
            chunky_dkg::{ChunkyDKGSessionMetadata, ChunkyDKGSessionState, ChunkyDKGState},
            recovery::DkgResourceProof,
            DKGSessionMetadata, DKGSessionState, DKGState,
        },
        epoch_change::EpochChangeProof,
        epoch_state::EpochState,
        ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
        on_chain_config::{OnChainChunkyDKGConfig, OnChainRandomnessConfig},
        proof::{
            SparseMerkleLeafNode, SparseMerkleProof, TransactionAccumulatorProof,
            TransactionInfoWithProof,
        },
        state_store::state_value::StateValue,
        transaction::{ExecutionStatus, TransactionInfo, Version},
        validator_verifier::ValidatorVerifier,
    };
    use std::collections::HashMap;

    const EPOCH: u64 = 10;
    const EPOCH_START_VERSION: Version = 42;

    struct EpochStartDb {
        proof: EpochChangeProof,
        values: HashMap<StateKey, StateValue>,
        fail_reads: bool,
    }

    impl EpochStartDb {
        fn new() -> Self {
            let ledger_info = LedgerInfo::new(
                BlockInfo::new(
                    EPOCH - 1,
                    1,
                    HashValue::zero(),
                    HashValue::zero(),
                    EPOCH_START_VERSION,
                    0,
                    Some(EpochState::new(EPOCH, ValidatorVerifier::new(vec![]))),
                ),
                HashValue::zero(),
            );
            Self {
                proof: EpochChangeProof::new(
                    vec![LedgerInfoWithSignatures::new(
                        ledger_info,
                        AggregateSignature::empty(),
                    )],
                    false,
                ),
                values: HashMap::new(),
                fail_reads: false,
            }
        }

        fn insert<T: OnChainConfig + serde::Serialize>(&mut self, state: &T) {
            self.values.insert(
                StateKey::on_chain_config::<T>().unwrap(),
                StateValue::from(bcs::to_bytes(state).unwrap()),
            );
        }
    }

    impl DbReader for EpochStartDb {
        fn get_epoch_ending_ledger_infos(
            &self,
            start: u64,
            end: u64,
        ) -> StorageResult<EpochChangeProof> {
            assert_eq!((start, end), (EPOCH - 1, EPOCH));
            Ok(self.proof.clone())
        }

        fn get_state_value_by_version(
            &self,
            key: &StateKey,
            version: Version,
        ) -> StorageResult<Option<StateValue>> {
            assert_eq!(
                version,
                self.proof.ledger_info_with_sigs[0].ledger_info().version()
            );
            if self.fail_reads {
                return Err(AptosDbError::Other("epoch-start state pruned".into()));
            }
            Ok(self.values.get(key).cloned())
        }
    }

    // A donor supplies one included public DKG resource plus a verified absence
    // for the other. The recipient retains its authenticated boundary ledger info.
    fn recovery_bundle<T: OnChainConfig>(db: &mut EpochStartDb) -> (TempPath, DkgRecoveryBundle) {
        let key = StateKey::on_chain_config::<T>().unwrap();
        let value = db.values.get(&key).unwrap().clone();
        let leaf = SparseMerkleLeafNode::new(*key.crypto_hash_ref(), value.hash());
        let info = TransactionInfo::new_placeholder(0, Some(leaf.hash()), ExecutionStatus::Success);
        let boundary = LedgerInfo::new(
            BlockInfo::new(
                EPOCH - 1,
                1,
                HashValue::zero(),
                info.hash(),
                0,
                0,
                Some(EpochState::new(EPOCH, ValidatorVerifier::new(vec![]))),
            ),
            HashValue::zero(),
        );
        db.proof = EpochChangeProof::new(
            vec![LedgerInfoWithSignatures::new(
                boundary,
                AggregateSignature::empty(),
            )],
            false,
        );
        let bundle = DkgRecoveryBundle::V1 {
            epoch: EPOCH,
            version: 0,
            transaction: TransactionInfoWithProof::new(
                TransactionAccumulatorProof::new(vec![]),
                info,
            ),
            randomness: DkgResourceProof {
                value: (key == StateKey::on_chain_config::<DKGState>().unwrap())
                    .then(|| value.clone()),
                proof: SparseMerkleProof::new(Some(leaf), vec![]),
            },
            chunky: DkgResourceProof {
                value: (key == StateKey::on_chain_config::<ChunkyDKGState>().unwrap())
                    .then_some(value),
                proof: SparseMerkleProof::new(Some(leaf), vec![]),
            },
        };
        let path = TempPath::new();
        std::fs::write(path.path(), bcs::to_bytes(&bundle).unwrap()).unwrap();
        (path, bundle)
    }

    #[test]
    fn pruned_history_uses_only_valid_bundle_and_preserves_latest_on_error() {
        let current = randomness_state(EPOCH - 1);
        let latest = randomness_state(EPOCH);
        let mut db = EpochStartDb::new();
        db.insert(&current);
        let (path, mut bundle) = recovery_bundle::<DKGState>(&mut db);
        db.fail_reads = true;
        let mut state = Ok(latest.clone());
        recover_dkg_state(EPOCH, &mut state, randomness_dealer, &db, Some(path.path())).unwrap();
        assert_eq!(state.unwrap(), current);
        // Latest-state DKG managers still observe the original DB state. The
        // fallback only replaces this local consensus key-derivation resource.
        assert_eq!(db.values.len(), 1);

        let DkgRecoveryBundle::V1 { randomness, .. } = &mut bundle;
        randomness.value = Some(StateValue::from(bcs::to_bytes(&latest).unwrap()));
        std::fs::write(path.path(), bcs::to_bytes(&bundle).unwrap()).unwrap();
        let mut state = Ok(latest.clone());
        assert!(
            recover_dkg_state(EPOCH, &mut state, randomness_dealer, &db, Some(path.path()))
                .is_err()
        );
        assert_eq!(state.unwrap(), latest);
    }

    #[test]
    fn chunky_dkg_recovers_from_pruned_history_with_verified_bundle() {
        let current = chunky_state(EPOCH - 1);
        let mut db = EpochStartDb::new();
        db.insert(&current);
        let (path, _) = recovery_bundle::<ChunkyDKGState>(&mut db);
        db.fail_reads = true;
        let mut state = Ok(chunky_state(EPOCH));
        recover_dkg_state(EPOCH, &mut state, chunky_dealer, &db, Some(path.path())).unwrap();
        assert_eq!(state.unwrap(), current);
    }

    #[test]
    fn proof_does_not_override_existing_recovered_epoch_checks() {
        for dealer in [EPOCH - 2, EPOCH] {
            let mut db = EpochStartDb::new();
            db.insert(&randomness_state(dealer));
            let (path, _) = recovery_bundle::<DKGState>(&mut db);
            db.fail_reads = true;
            let original = randomness_state(EPOCH);
            let mut state = Ok(original.clone());
            let result =
                recover_dkg_state(EPOCH, &mut state, randomness_dealer, &db, Some(path.path()));
            if dealer == EPOCH {
                assert!(result.is_err());
                assert_eq!(state.unwrap(), original);
            } else {
                result.unwrap();
                let recovered = state.unwrap();
                assert_eq!(recovered, randomness_state(dealer));
                assert!(recovered.maybe_last_complete(EPOCH).is_none());
            }
        }
    }

    #[test]
    fn successful_history_read_does_not_open_configured_bundle() {
        let mut db = EpochStartDb::new();
        // An authenticated absence must NOT be replaced by donor data.
        let mut state = Ok(randomness_state(EPOCH));
        let nonexistent = TempPath::new();
        recover_dkg_state(
            EPOCH,
            &mut state,
            randomness_dealer,
            &db,
            Some(nonexistent.path()),
        )
        .unwrap();
        assert_eq!(state.unwrap(), DKGState::default());
        db.insert(&randomness_state(EPOCH - 1));
        let mut state = Ok(randomness_state(EPOCH));
        recover_dkg_state(
            EPOCH,
            &mut state,
            randomness_dealer,
            &db,
            Some(nonexistent.path()),
        )
        .unwrap();
        assert_eq!(state.unwrap(), randomness_state(EPOCH - 1));
    }

    fn randomness_state(dealer_epoch: u64) -> DKGState {
        DKGState {
            last_completed: Some(DKGSessionState {
                metadata: DKGSessionMetadata {
                    dealer_epoch,
                    randomness_config: OnChainRandomnessConfig::default_enabled().into(),
                    dealer_validator_set: vec![],
                    target_validator_set: vec![],
                },
                start_time_us: dealer_epoch,
                transcript: dealer_epoch.to_le_bytes().to_vec(),
            }),
            in_progress: None,
        }
    }

    fn chunky_state(dealer_epoch: u64) -> ChunkyDKGState {
        ChunkyDKGState {
            last_completed: Some(ChunkyDKGSessionState {
                metadata: ChunkyDKGSessionMetadata {
                    dealer_epoch,
                    chunky_dkg_config: OnChainChunkyDKGConfig::default_enabled().into(),
                    dealer_validator_set: vec![],
                    target_validator_set: vec![],
                },
                start_time_us: dealer_epoch,
                transcript: dealer_epoch.to_le_bytes().to_vec(),
            }),
            in_progress: None,
        }
    }

    fn randomness_dealer(state: &DKGState) -> Option<u64> {
        state
            .last_completed
            .as_ref()
            .map(|s| s.metadata.dealer_epoch)
    }

    fn chunky_dealer(state: &ChunkyDKGState) -> Option<u64> {
        state
            .last_completed
            .as_ref()
            .map(|s| s.metadata.dealer_epoch)
    }

    #[test]
    fn restart_after_either_dkg_finishes_recovers_the_current_epoch() {
        let current_randomness = randomness_state(EPOCH - 1);
        let current_chunky = chunky_state(EPOCH - 1);
        let mut db = EpochStartDb::new();
        db.insert(&current_randomness);
        db.insert(&current_chunky);

        for randomness_finished_first in [true, false] {
            let mut randomness = Ok(current_randomness.clone());
            let mut chunky = Ok(current_chunky.clone());
            if randomness_finished_first {
                randomness = Ok(randomness_state(EPOCH));
                chunky.as_mut().unwrap().in_progress = chunky_state(EPOCH).last_completed;
                assert!(randomness
                    .as_ref()
                    .unwrap()
                    .maybe_last_complete(EPOCH)
                    .is_none());
            } else {
                chunky = Ok(chunky_state(EPOCH));
                randomness.as_mut().unwrap().in_progress = randomness_state(EPOCH).last_completed;
                assert!(chunky
                    .as_ref()
                    .unwrap()
                    .maybe_last_complete(EPOCH)
                    .is_none());
            }

            recover_dkg_state(EPOCH, &mut randomness, randomness_dealer, &db, None).unwrap();
            recover_dkg_state(EPOCH, &mut chunky, chunky_dealer, &db, None).unwrap();
            assert_eq!(
                randomness.unwrap().maybe_last_complete(EPOCH),
                current_randomness.last_completed.as_ref()
            );
            assert_eq!(
                chunky.unwrap().maybe_last_complete(EPOCH),
                current_chunky.last_completed.as_ref()
            );
        }
    }

    #[test]
    fn recovered_transcript_restores_the_same_randomness_key_share() {
        use aptos_types::{
            dkg::{real_dkg::maybe_dk_from_bls_sk, DKGTrait, DefaultDKG},
            validator_signer::ValidatorSigner,
            validator_verifier::ValidatorConsensusInfo,
        };

        let signers = (0..3)
            .map(|i| ValidatorSigner::random([i; 32]))
            .collect::<Vec<_>>();
        let validators = signers
            .iter()
            .map(|s| ValidatorConsensusInfo::new(s.author(), s.public_key(), 1_000_000).into())
            .collect::<Vec<_>>();
        let mut current = randomness_state(EPOCH - 1);
        let session = current.last_completed.as_mut().unwrap();
        session.metadata.dealer_validator_set = validators.clone();
        session.metadata.target_validator_set = validators;
        let params = DefaultDKG::new_public_params(&session.metadata);
        let transcript = DefaultDKG::sample_secret_and_generate_transcript(
            &mut rand::thread_rng(),
            &params,
            0,
            signers[0].private_key(),
            &signers[0].public_key(),
        );
        session.transcript = bcs::to_bytes(&transcript).unwrap();
        let decrypt_key = maybe_dk_from_bls_sk(signers[2].private_key()).unwrap();
        let before =
            DefaultDKG::decrypt_secret_share_from_transcript(&params, &transcript, 2, &decrypt_key)
                .unwrap();
        let mut db = EpochStartDb::new();
        db.insert(&current);

        // The exiting validator still needs its current-epoch share, even though
        // it is not a recipient in the now-completed next-epoch session.
        let mut next = current.clone();
        let next_session = next.last_completed.as_mut().unwrap();
        next_session.metadata.dealer_epoch = EPOCH;
        next_session.metadata.target_validator_set.pop();
        let next_params = DefaultDKG::new_public_params(&next_session.metadata);
        next_session.transcript =
            bcs::to_bytes(&DefaultDKG::sample_secret_and_generate_transcript(
                &mut rand::thread_rng(),
                &next_params,
                0,
                signers[0].private_key(),
                &signers[0].public_key(),
            ))
            .unwrap();
        assert!(next.maybe_last_complete(EPOCH).is_none());
        for pruned in [false, true] {
            let path = pruned.then(|| recovery_bundle::<DKGState>(&mut db).0);
            db.fail_reads = pruned;
            let mut recovered = Ok(next.clone());
            recover_dkg_state(
                EPOCH,
                &mut recovered,
                randomness_dealer,
                &db,
                path.as_ref().map(TempPath::path),
            )
            .unwrap();
            let recovered = recovered.unwrap();
            let session = recovered.maybe_last_complete(EPOCH).unwrap();
            let params = DefaultDKG::new_public_params(&session.metadata);
            let transcript = bcs::from_bytes(&session.transcript).unwrap();
            DefaultDKG::verify_transcript(&params, &transcript).unwrap();
            let after = DefaultDKG::decrypt_secret_share_from_transcript(
                &params,
                &transcript,
                2,
                &decrypt_key,
            )
            .unwrap();
            assert_eq!(
                bcs::to_bytes(&before).unwrap(),
                bcs::to_bytes(&after).unwrap()
            );
        }
    }

    #[test]
    fn current_stale_and_missing_sessions_do_not_read_history() {
        // Any attempted historical read fails the test through DbReader's defaults.
        struct NoReads;
        impl DbReader for NoReads {}
        for original in [
            randomness_state(EPOCH - 1),
            randomness_state(EPOCH - 2),
            DKGState::default(),
        ] {
            let mut state = Ok(original.clone());
            recover_dkg_state(EPOCH, &mut state, randomness_dealer, &NoReads, None).unwrap();
            assert_eq!(state.unwrap(), original);
        }
        let mut missing = Err(anyhow::anyhow!("resource missing"));
        recover_dkg_state(EPOCH, &mut missing, randomness_dealer, &NoReads, None).unwrap();
        assert_eq!(missing.unwrap_err().to_string(), "resource missing");
    }

    #[test]
    fn recovery_preserves_epochs_without_a_usable_session() {
        for previous in [
            None,
            Some(DKGState::default()),
            Some(randomness_state(EPOCH - 2)),
        ] {
            let mut db = EpochStartDb::new();
            if let Some(state) = &previous {
                db.insert(state);
            }
            let mut state = Ok(randomness_state(EPOCH));
            recover_dkg_state(EPOCH, &mut state, randomness_dealer, &db, None).unwrap();
            let recovered = state.unwrap();
            assert_eq!(recovered, previous.unwrap_or_default());
            assert!(recovered.maybe_last_complete(EPOCH).is_none());
        }
    }

    #[test]
    fn recovery_errors_preserve_the_latest_state() {
        for failure in 0..4 {
            let mut db = EpochStartDb::new();
            match failure {
                0 => db.proof.ledger_info_with_sigs.clear(),
                1 => db.fail_reads = true,
                2 => {
                    db.values.insert(
                        StateKey::on_chain_config::<DKGState>().unwrap(),
                        StateValue::from(vec![255]),
                    );
                },
                3 => db.insert(&randomness_state(EPOCH)),
                _ => unreachable!(),
            }
            let original = randomness_state(EPOCH);
            let mut state = Ok(original.clone());
            assert!(recover_dkg_state(EPOCH, &mut state, randomness_dealer, &db, None).is_err());
            assert_eq!(state.unwrap(), original);
        }
    }
}
