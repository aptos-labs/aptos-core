// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::needless_borrow)]
#![allow(clippy::ptr_arg)]
#![allow(clippy::let_and_return)]

//! PVSS scheme-independent testing
use velor_crypto::hash::CryptoHash;
use velor_dkg::{
    constants::{G1_PROJ_NUM_BYTES, G2_PROJ_NUM_BYTES},
    pvss::{
        das,
        das::unweighted_protocol,
        insecure_field, test_utils,
        test_utils::{
            get_threshold_configs_for_benchmarking, get_weighted_configs_for_benchmarking,
            reconstruct_dealt_secret_key_randomly, NoAux,
        },
        traits::{transcript::Transcript, SecretSharingConfig},
        GenericWeighting, ThresholdConfig,
    },
    utils::random::random_scalar,
};
use rand::{rngs::StdRng, thread_rng};
use rand_core::SeedableRng;

#[test]
fn test_pvss_all_unweighted() {
    let mut rng = thread_rng();

    //
    // Unweighted PVSS tests
    //
    let tcs = test_utils::get_threshold_configs_for_testing();
    for tc in tcs {
        println!("\nTesting {tc} PVSS");

        let seed = random_scalar(&mut rng);

        // Das
        pvss_deal_verify_and_reconstruct::<das::Transcript>(&tc, seed.to_bytes_le());

        // Insecure testing-only field-element PVSS
        pvss_deal_verify_and_reconstruct::<insecure_field::Transcript>(&tc, seed.to_bytes_le());
    }
}

#[test]
fn test_pvss_all_weighted() {
    let mut rng = thread_rng();

    //
    // PVSS weighted tests
    //
    let wcs = test_utils::get_weighted_configs_for_testing();

    for wc in wcs {
        println!("\nTesting {wc} PVSS");
        let seed = random_scalar(&mut rng);

        // Generically-weighted Das
        // WARNING: Insecure, due to encrypting different shares with the same randomness, do not use!
        pvss_deal_verify_and_reconstruct::<GenericWeighting<das::Transcript>>(
            &wc,
            seed.to_bytes_le(),
        );

        // Generically-weighted field-element PVSS
        // WARNING: Insecure, reveals the dealt secret and its shares.
        pvss_deal_verify_and_reconstruct::<GenericWeighting<insecure_field::Transcript>>(
            &wc,
            seed.to_bytes_le(),
        );

        // Provably-secure Das PVSS
        pvss_deal_verify_and_reconstruct::<das::WeightedTranscript>(&wc, seed.to_bytes_le());
    }
}

#[test]
fn test_pvss_transcript_size() {
    for sc in get_threshold_configs_for_benchmarking() {
        println!();
        let expected_size = expected_transcript_size::<das::Transcript>(&sc);
        let actual_size = actual_transcript_size::<das::Transcript>(&sc);

        print_transcript_size::<das::Transcript>("Expected", &sc, expected_size);
        print_transcript_size::<das::Transcript>("Actual", &sc, actual_size);
    }

    for wc in get_weighted_configs_for_benchmarking() {
        let actual_size = actual_transcript_size::<das::Transcript>(wc.get_threshold_config());
        print_transcript_size::<das::Transcript>("Actual", wc.get_threshold_config(), actual_size);

        let actual_size = actual_transcript_size::<das::WeightedTranscript>(&wc);
        print_transcript_size::<das::WeightedTranscript>("Actual", &wc, actual_size);
    }
}

fn print_transcript_size<T: Transcript>(size_type: &str, sc: &T::SecretSharingConfig, size: usize) {
    let name = T::scheme_name();
    println!("{size_type:8} transcript size for {sc} {name}: {size} bytes");
}

//
// Helper functions
//

/// Basic viability test for a PVSS transcript (weighted or unweighted):
///  1. Deals a secret, creating a transcript
///  2. Verifies the transcript.
///  3. Ensures the a sufficiently-large random subset of the players can recover the dealt secret
fn pvss_deal_verify_and_reconstruct<T: Transcript + CryptoHash>(
    sc: &T::SecretSharingConfig,
    seed_bytes: [u8; 32],
) {
    // println!();
    // println!("Seed: {}", hex::encode(seed_bytes.as_slice()));
    let mut rng = StdRng::from_seed(seed_bytes);

    let d = test_utils::setup_dealing::<T, StdRng>(sc, &mut rng);

    // Test dealing
    let trx = T::deal(
        &sc,
        &d.pp,
        &d.ssks[0],
        &d.eks,
        &d.s,
        &NoAux,
        &sc.get_player(0),
        &mut rng,
    );
    trx.verify(&sc, &d.pp, &vec![d.spks[0].clone()], &d.eks, &vec![NoAux])
        .expect("PVSS transcript failed verification");

    // Test transcript (de)serialization
    let trx_deserialized = T::try_from(trx.to_bytes().as_slice())
        .expect("serialized transcript should deserialize correctly");

    assert_eq!(trx, trx_deserialized);
    if d.dsk != reconstruct_dealt_secret_key_randomly::<StdRng, T>(sc, &mut rng, &d.dks, trx) {
        panic!("Reconstructed SK did not match");
    }
}

fn actual_transcript_size<T: Transcript>(sc: &T::SecretSharingConfig) -> usize {
    let mut rng = thread_rng();

    let trx = T::generate(&sc, &mut rng);
    let actual_size = trx.to_bytes().len();

    actual_size
}

fn expected_transcript_size<T: Transcript<SecretSharingConfig = ThresholdConfig>>(
    sc: &ThresholdConfig,
) -> usize {
    if T::scheme_name() == unweighted_protocol::DAS_SK_IN_G1 {
        G2_PROJ_NUM_BYTES
            + (sc.get_total_num_players() + 1) * (G2_PROJ_NUM_BYTES + G1_PROJ_NUM_BYTES)
    } else {
        panic!("Did not implement support for '{}' yet", T::scheme_name())
    }
}
