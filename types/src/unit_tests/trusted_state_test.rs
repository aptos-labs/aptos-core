// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aggregate_signature::{AggregateSignature, PartialSignatures},
    block_info::BlockInfo,
    epoch_change::EpochChangeProof,
    epoch_state::EpochState,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    proof::accumulator::mock::MockTransactionAccumulator,
    transaction::Version,
    trusted_state::{TrustedState, TrustedStateChange, TrustedStateHasher},
    validator_signer::ValidatorSigner,
    validator_verifier::{random_validator_verifier, ValidatorConsensusInfo, ValidatorVerifier},
    waypoint::Waypoint,
};
use velor_crypto::hash::{CryptoHash, CryptoHasher, HashValue};
use bcs::test_helpers::assert_canonical_encode_decode;
use proptest::{
    collection::{size_range, vec, SizeRange},
    prelude::*,
    sample::Index,
};
use std::sync::Arc;

// hack strategy to generate a length from `impl Into<SizeRange>`
fn arb_length(size_range: impl Into<SizeRange>) -> impl Strategy<Value = usize> {
    vec(Just(()), size_range).prop_map(|vec| vec.len())
}

/// For `n` epoch changes, we sample `n+1` validator sets of variable size
/// `validators_per_epoch`. The `+1` is for the initial validator set in the first
/// epoch.
fn arb_validator_sets(
    epoch_changes: impl Into<SizeRange>,
    validators_per_epoch: impl Into<SizeRange>,
) -> impl Strategy<Value = Vec<(Vec<ValidatorSigner>, ValidatorVerifier)>> {
    vec(arb_length(validators_per_epoch), epoch_changes.into() + 1).prop_map(
        |validators_per_epoch_vec| {
            validators_per_epoch_vec
                .into_iter()
                .map(|num_validators| {
                    // all uniform voting power
                    let voting_power = None;
                    // human readable incrementing account addresses
                    let int_account_addrs = true;
                    random_validator_verifier(num_validators, voting_power, int_account_addrs)
                })
                .collect::<Vec<_>>()
        },
    )
}

/// Convert a slice of `ValidatorSigner` (includes the private signing key) into
/// the public-facing `EpochState` type (just the public key).
fn into_epoch_state(epoch: u64, signers: &[ValidatorSigner]) -> EpochState {
    EpochState {
        epoch,
        verifier: Arc::new(ValidatorVerifier::new(
            signers
                .iter()
                .map(|signer| {
                    ValidatorConsensusInfo::new(
                        signer.author(),
                        signer.public_key(),
                        1, /* voting power */
                    )
                })
                .collect(),
        )),
    }
}

/// Create all signatures for a `LedgerInfoWithSignatures` given a set of signers
/// and a `LedgerInfo`.
fn sign_ledger_info(
    signers: &[ValidatorSigner],
    verifier: &ValidatorVerifier,
    ledger_info: &LedgerInfo,
) -> AggregateSignature {
    let partial_sig = PartialSignatures::new(
        signers
            .iter()
            .map(|s| (s.author(), s.sign(ledger_info).unwrap()))
            .collect(),
    );
    verifier
        .aggregate_signatures(partial_sig.signatures_iter())
        .unwrap()
}

fn mock_ledger_info(
    epoch: u64,
    version: Version,
    root_hash: HashValue,
    next_epoch_state: Option<EpochState>,
) -> LedgerInfo {
    LedgerInfo::new(
        BlockInfo::new(
            epoch,
            0,                 /* round */
            HashValue::zero(), /* id */
            root_hash,         /* executed_state_id */
            version,
            0, /* timestamp_usecs */
            next_epoch_state,
        ),
        HashValue::zero(),
    )
}

// A strategy for generating components of an UpdateToLatestLedgerResponse with
// a correct EpochChangeProof.
fn arb_update_proof(
    // the epoch of the first LedgerInfoWithSignatures
    start_epoch: u64,
    // the version of the first LedgerInfoWithSignatures
    start_version: Version,
    // the distribution of versions changes between LedgerInfoWithSignatures
    version_delta: impl Into<SizeRange>,
    // the distribution for the number of epoch changes to generate
    epoch_changes: impl Into<SizeRange>,
    // the distribution for the number of validators in each epoch
    validators_per_epoch: impl Into<SizeRange>,
) -> impl Strategy<
    Value = (
        // The validator sets for each epoch
        Vec<Vec<ValidatorSigner>>,
        // The epoch change ledger infos
        Vec<LedgerInfoWithSignatures>,
        // The latest ledger info inside the last epoch
        LedgerInfoWithSignatures,
        // A mock accumulator consistent with the generated ledger infos
        MockTransactionAccumulator,
    ),
> {
    // helpful diagram:
    //
    // input:
    //   num epoch changes
    //
    // output:
    //   vsets: [S_1 .. S_n+1],
    //   epoch changes: [L_1, .., L_n],
    //   latest ledger_info: L_n+1
    //
    // let S_i = ith set of validators
    // let L_i = ith ledger info
    // S_i -> L_i => ith validators sign ith ledger info
    // L_i -> S_i+1 => ith ledger info contains i+1'th validators for epoch change
    // L_n+1 = a ledger info inside the nth epoch (contains S = None)
    //
    // base case: n = 0 => no epoch changes
    //
    // [ S_1 ] (None)
    //     \   __^
    //      v /
    //    [ L_1 ]
    //
    // otherwise, for n > 0:
    //
    // [ S_1, S_2, ..., S_n+1 ] (None)
    //    \    ^ \       ^ \   __^
    //     v  /   v     /   v /
    //    [ L_1, L_2, ..., L_n+1 ]
    //

    let version_delta = size_range(version_delta);
    let epoch_changes = size_range(epoch_changes);
    let validators_per_epoch = size_range(validators_per_epoch);

    // sample n, the number of epoch changes
    arb_length(epoch_changes).prop_flat_map(move |epoch_changes| {
        (
            // sample the validator sets, including the signers for the first epoch
            arb_validator_sets(epoch_changes, validators_per_epoch.clone()),
            // generate n version deltas
            vec(arb_length(version_delta.clone()), epoch_changes),
        )
            .prop_map(move |(mut signers_and_verifier, version_deltas)| {
                // if generating from genesis, then there is no validator set to
                // sign the genesis block.
                if start_epoch == 0 {
                    // this will always succeed, since
                    // n >= 0, |vsets| = n + 1 ==> |vsets| >= 1
                    let (signers, verifier) = signers_and_verifier.first_mut().unwrap();
                    *signers = vec![];
                    *verifier = random_validator_verifier(0, None, true).1;
                }

                // build a mock accumulator with fake txn hashes up to the last
                // version. we'll ensure that all ledger infos have appropriate
                // root hash values for their version.
                let end_version = start_version + version_deltas.iter().sum::<usize>() as u64;
                let accumulator = MockTransactionAccumulator::with_version(end_version);

                let mut epoch = start_epoch;
                let mut version = start_version;
                let num_epoch_changes = signers_and_verifier.len() - 1;

                let signers = signers_and_verifier.iter().take(num_epoch_changes);
                let next_sets = signers_and_verifier.iter().skip(1);

                let ledger_infos_with_sigs = signers
                    .zip(next_sets)
                    .zip(version_deltas)
                    .map(|((curr_vset, next_vset), version_delta)| {
                        let next_vset = into_epoch_state(epoch + 1, &next_vset.0);
                        let root_hash = accumulator.get_root_hash(version);
                        let ledger_info =
                            mock_ledger_info(epoch, version, root_hash, Some(next_vset));
                        let aggregated_sig =
                            sign_ledger_info(&curr_vset.0, &curr_vset.1, &ledger_info);

                        epoch += 1;
                        version += version_delta as u64;

                        LedgerInfoWithSignatures::new(ledger_info, aggregated_sig)
                    })
                    .collect::<Vec<_>>();

                // this will always succeed, since
                // n >= 0, |vsets| = n + 1 ==> |vsets| >= 1
                let last_vset = signers_and_verifier.last().unwrap();
                let root_hash = accumulator.get_root_hash(version);
                let latest_ledger_info = mock_ledger_info(epoch, version, root_hash, None);
                let aggregated_sig =
                    sign_ledger_info(&last_vset.0, &last_vset.1, &latest_ledger_info);
                let latest_ledger_info_with_sigs =
                    LedgerInfoWithSignatures::new(latest_ledger_info, aggregated_sig);
                (
                    signers_and_verifier.into_iter().map(|x| x.0).collect(),
                    ledger_infos_with_sigs,
                    latest_ledger_info_with_sigs,
                    accumulator,
                )
            })
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn test_trusted_state_roundtrip_canonical_serialization(trusted_state in any::<TrustedState>()) {
        assert_canonical_encode_decode(trusted_state);
    }

    #[test]
    fn test_trusted_state_hasher(trusted_state in any::<TrustedState>()) {
        let bytes = bcs::to_bytes(&trusted_state).unwrap();
        let hash1 = TrustedStateHasher::hash_all(&bytes);
        let hash2 = trusted_state.hash();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_ratchet_from(
        (_vsets, lis_with_sigs, latest_li, _) in arb_update_proof(
            10,   /* start epoch */
            123,  /* start version */
            1..3, /* version delta */
            1..3, /* epoch changes */
            1..5, /* validators per epoch */
        )
    ) {
        let first_epoch_change_li = lis_with_sigs.first().unwrap();
        let waypoint = Waypoint::new_epoch_boundary(first_epoch_change_li.ledger_info())
            .expect("Generating waypoint failed even though we passed an epoch change ledger info");
        let trusted_state = TrustedState::from_epoch_waypoint(waypoint);

        let expected_latest_version = latest_li.ledger_info().version();
        let expected_latest_epoch_change_li = lis_with_sigs.last().cloned();
        let expected_validator_set = expected_latest_epoch_change_li
            .as_ref()
            .and_then(|li_with_sigs| li_with_sigs.ledger_info().next_epoch_state());

        let change_proof = EpochChangeProof::new(lis_with_sigs, false /* more */);
        let trusted_state_change = trusted_state
            .verify_and_ratchet_inner(&latest_li, &change_proof)
            .expect("Should never error or be stale when ratcheting from waypoint with valid proofs");

        match trusted_state_change {
            TrustedStateChange::Epoch {
                new_state,
                latest_epoch_change_li,
            } => {
                assert_eq!(new_state.version(), expected_latest_version);
                assert_eq!(Some(latest_epoch_change_li), expected_latest_epoch_change_li.as_ref());
                assert_eq!(latest_epoch_change_li.ledger_info().next_epoch_state(), expected_validator_set);
            }
            _ => panic!("Ratcheting from a waypoint should always provide the epoch for that waypoint"),
        };
    }

    #[test]
    fn test_ratchet_version_only(
        (_vsets, mut lis_with_sigs, latest_li, accumulator) in arb_update_proof(
            1,    /* start epoch */
            1,    /* start version */
            1..3, /* version delta */
            1,    /* epoch changes */
            1..5, /* validators per epoch */
        )
    ) {
        // Assume we have already ratcheted into this epoch
        let epoch_change_li = lis_with_sigs.remove(0);
        let start_version = epoch_change_li.ledger_info().version();
        let trusted_state = TrustedState::try_from_epoch_change_li(
            epoch_change_li.ledger_info(),
            accumulator.get_accumulator_summary(start_version),
        ).unwrap();

        let expected_latest_version = latest_li.ledger_info().version();

        // Use an empty epoch change proof
        let change_proof = EpochChangeProof::new(vec![], false /* more */);
        let trusted_state_change = trusted_state
            .verify_and_ratchet_inner(&latest_li, &change_proof)
            .expect("Should never error or be stale when ratcheting from waypoint with valid proofs");

        match trusted_state_change {
            TrustedStateChange::Epoch{ .. } => panic!("Empty change proof so we should not change epoch"),
            TrustedStateChange::Version {
                new_state,
            } => {
                assert_eq!(new_state.version(), expected_latest_version);
            }
            TrustedStateChange::NoChange => assert_eq!(trusted_state.version(), expected_latest_version),
        };
    }

    #[test]
    fn test_ratchet_fails_with_gap_in_proof(
        (_vsets, mut lis_with_sigs, latest_li, accumulator) in arb_update_proof(
            1,    /* start epoch */
            1,    /* start version */
            3,    /* version delta */
            3..6, /* epoch changes */
            1..3, /* validators per epoch */
        ),
        li_gap_idx in any::<Index>(),
    ) {
        let initial_li_with_sigs = lis_with_sigs.remove(0);
        let initial_li = initial_li_with_sigs.ledger_info();
        let trusted_state = TrustedState::try_from_epoch_change_li(
            initial_li,
            accumulator.get_accumulator_summary(initial_li.version()),
        ).unwrap();

        // materialize index and remove an epoch change in the proof to add a gap
        let li_gap_idx = li_gap_idx.index(lis_with_sigs.len() - 1);
        lis_with_sigs.remove(li_gap_idx);

        let change_proof = EpochChangeProof::new(lis_with_sigs, false /* more */);
        // should fail since there's a missing epoch change li in the change proof.
        trusted_state
            .verify_and_ratchet_inner(&latest_li, &change_proof)
            .expect_err("Should always return Err with an invalid change proof");
    }

    #[test]
    fn test_ratchet_succeeds_with_more(
        (_vsets, mut lis_with_sigs, latest_li, accumulator) in arb_update_proof(
            1,    /* start epoch */
            1,    /* start version */
            3,    /* version delta */
            3..6, /* epoch changes */
            1..3, /* validators per epoch */
        ),
    ) {
        let initial_li_with_sigs = lis_with_sigs.remove(0);
        let initial_li = initial_li_with_sigs.ledger_info();
        let trusted_state = TrustedState::try_from_epoch_change_li(
            initial_li,
            accumulator.get_accumulator_summary(initial_li.version()),
        ).unwrap();

        // remove the last LI from the proof
        lis_with_sigs.pop();

        let expected_latest_epoch_change_li = lis_with_sigs.last().unwrap().clone();
        let expected_latest_version = expected_latest_epoch_change_li
            .ledger_info()
            .version();

        // ratcheting with more = false should fail, since the state proof claims
        // we're done syncing epoch changes but doesn't get us all the way to the
        // latest ledger info
        let mut change_proof = EpochChangeProof::new(lis_with_sigs, false /* more */);
        trusted_state
            .verify_and_ratchet_inner(&latest_li, &change_proof)
            .expect_err("Should return Err when more is false and there's a gap");

        // ratcheting with more = true is fine
        change_proof.more = true;
        let trusted_state_change = trusted_state
            .verify_and_ratchet_inner(&latest_li, &change_proof)
            .expect("Should succeed with more in EpochChangeProof");

        match trusted_state_change {
            TrustedStateChange::Epoch {
                new_state,
                latest_epoch_change_li,
            } => {
                assert_eq!(new_state.version(), expected_latest_version);
                assert_eq!(latest_epoch_change_li, &expected_latest_epoch_change_li);
            }
            _ => panic!("Unexpected ratchet result"),
        };
    }

    #[test]
    fn test_ratchet_fails_with_invalid_signature(
        (_vsets, mut lis_with_sigs, latest_li, accumulator) in arb_update_proof(
            1,    /* start epoch */
            1,    /* start version */
            1,    /* version delta */
            2..5, /* epoch changes */
            1..5, /* validators per epoch */
        ),
        bad_li_idx in any::<Index>(),
    ) {
        let initial_li_with_sigs = lis_with_sigs.remove(0);
        let initial_li = initial_li_with_sigs.ledger_info();
        let trusted_state = TrustedState::try_from_epoch_change_li(
            initial_li,
            accumulator.get_accumulator_summary(initial_li.version()),
        ).unwrap();

        // Swap in a bad ledger info without signatures
        let li_with_sigs = bad_li_idx.get(&lis_with_sigs);
        let bad_li_with_sigs = LedgerInfoWithSignatures::new(
            li_with_sigs.ledger_info().clone(),
            AggregateSignature::empty(), /* empty signatures */
        );
        *bad_li_idx.get_mut(&mut lis_with_sigs) = bad_li_with_sigs;

        let change_proof = EpochChangeProof::new(lis_with_sigs, false /* more */);
        trusted_state
            .verify_and_ratchet_inner(&latest_li, &change_proof)
            .expect_err("Should always return Err with an invalid change proof");
    }

    #[test]
    fn test_ratchet_fails_with_invalid_latest_li(
        (_vsets, mut lis_with_sigs, latest_li, accumulator) in arb_update_proof(
            1,    /* start epoch */
            1,    /* start version */
            1,    /* version delta */
            1..5, /* epoch changes */
            1..5, /* validators per epoch */
        ),
    ) {
        let initial_li_with_sigs = lis_with_sigs.remove(0);
        let initial_li = initial_li_with_sigs.ledger_info();
        let trusted_state = TrustedState::try_from_epoch_change_li(
            initial_li,
            accumulator.get_accumulator_summary(initial_li.version()),
        ).unwrap();

        let good_li = latest_li.ledger_info();
        let change_proof = EpochChangeProof::new(lis_with_sigs, false /* more */);
        let sigs = latest_li.signatures();

        // Verifying latest ledger infos with mismatched data and signatures should fail
        let bad_li_1 = LedgerInfoWithSignatures::new(
            LedgerInfo::new(
                BlockInfo::new(
                    good_li.epoch(),
                    0,                 /* round */
                    HashValue::zero(), /* id */
                    HashValue::zero(), /* executed_state_id */
                    good_li.version(),
                    42, /* bad timestamp_usecs */
                    None,
                ),
                HashValue::zero(),
            ),
            sigs.clone(),
        );
        let bad_li_2 = LedgerInfoWithSignatures::new(
            mock_ledger_info(good_li.epoch(), good_li.version(), HashValue::zero(), None),
            sigs.clone(),
        );
        let bad_li_3 = LedgerInfoWithSignatures::new(
            mock_ledger_info(999, good_li.version(), good_li.transaction_accumulator_hash(), None),
            sigs.clone(),
        );
        let bad_li_4 = LedgerInfoWithSignatures::new(
            mock_ledger_info(good_li.epoch(), 999, good_li.transaction_accumulator_hash(), None),
            sigs.clone(),
        );
        let bad_li_5 = LedgerInfoWithSignatures::new(good_li.clone(), AggregateSignature::empty());

        trusted_state.verify_and_ratchet_inner(&bad_li_1, &change_proof).unwrap_err();
        trusted_state.verify_and_ratchet_inner(&bad_li_2, &change_proof).unwrap_err();
        trusted_state.verify_and_ratchet_inner(&bad_li_3, &change_proof).unwrap_err();
        trusted_state.verify_and_ratchet_inner(&bad_li_4, &change_proof).unwrap_err();
        trusted_state.verify_and_ratchet_inner(&bad_li_5, &change_proof).unwrap_err();
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1))]

    #[test]
    fn test_stale_ratchet(
        (_vsets, lis_with_sigs, latest_li, _) in arb_update_proof(
            1,    /* start epoch */
            1,    /* start version */
            1..3, /* version delta */
            1,    /* epoch changes */
            1..5, /* validators per epoch */
        ),
    ) {
        // We've ratched beyond the response change proof, so attempting to ratchet
        // that change proof should just return `TrustedStateChange::Stale`.
        let future_version = 456;
        let future_accumulator = MockTransactionAccumulator::with_version(future_version);
        let root_hash = future_accumulator.get_root_hash(future_version);
        let epoch_change_li = mock_ledger_info(123 /* epoch */, future_version, root_hash, Some(EpochState::empty()));
        let trusted_state = TrustedState::try_from_epoch_change_li(
            &epoch_change_li,
            future_accumulator.get_accumulator_summary(future_version),
        ).unwrap();

        let change_proof = EpochChangeProof::new(lis_with_sigs, false /* more */);
        trusted_state
            .verify_and_ratchet_inner(&latest_li, &change_proof)
            .expect_err("Expected stale change, got valid change");
    }
}
