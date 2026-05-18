// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![allow(clippy::needless_borrow)]
#![allow(clippy::ptr_arg)]
#![allow(clippy::let_and_return)]

//! PVSS scheme-independent testing
#[cfg(test)]
use aptos_crypto::TSecretSharingConfig;
use aptos_crypto::{
    blstrs::{random_scalar, G1_PROJ_NUM_BYTES, G2_PROJ_NUM_BYTES},
    weighted_config::WeightedConfigArkworks,
};
#[cfg(test)]
use aptos_dkg::pvss::traits::AggregatableTranscript;
use aptos_dkg::pvss::{
    chunky, das,
    das::unweighted_protocol,
    insecure_field, test_utils,
    test_utils::{
        get_threshold_configs_for_benchmarking, get_weighted_configs_for_benchmarking,
        reconstruct_dealt_secret_key_randomly, NoAux,
    },
    traits::transcript::{
        Aggregatable, Aggregated, HasAggregatableSubtranscript, Transcript, TranscriptCore,
        WithMaxNumShares,
    },
    GenericWeighting, ThresholdConfigBlstrs,
};
use ark_bn254::Bn254;
use ark_ec::pairing::Pairing;
use rand::{rngs::StdRng, thread_rng};
use rand_core::{CryptoRng, RngCore, SeedableRng};
use std::time::Instant;

// TODO: Add a test for public parameters serialization roundtrip?

#[test]
fn test_pvss_all_unweighted() {
    let mut rng = thread_rng();

    //
    // Unweighted PVSS tests
    //
    // TODO: might be better to grab (t,n) pairs, then intialise them for each PVSS using `new()` ? and consider renaming that `new_from_threshold()` ?
    let tcs = test_utils::get_threshold_configs_for_testing();
    for tc in tcs {
        println!("\nTesting {tc} PVSS");

        let seed = random_scalar(&mut rng);

        // Das
        pvss_deal_verify_and_reconstruct::<das::Transcript>(&tc, seed.to_bytes_le());

        // Insecure testing-only field-element PVSS.
        // TODO: Remove? Or potentially useful as it might show bugs arising elsewhere?
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
        // TODO: Remove?
        let t = Instant::now();
        pvss_deal_verify_and_reconstruct::<GenericWeighting<das::Transcript>>(
            &wc,
            seed.to_bytes_le(),
        );
        println!("  GenericWeighting<das::Transcript>: {:?}", t.elapsed());

        // Generically-weighted field-element PVSS
        // WARNING: Insecure, reveals the dealt secret and its shares.
        // TODO: Remove?
        let t = Instant::now();
        pvss_deal_verify_and_reconstruct::<GenericWeighting<insecure_field::Transcript>>(
            &wc,
            seed.to_bytes_le(),
        );
        println!(
            "  GenericWeighting<insecure_field::Transcript>: {:?}",
            t.elapsed()
        );

        // Provably-secure Das PVSS
        let t = Instant::now();
        pvss_deal_verify_and_reconstruct::<das::WeightedTranscript>(&wc, seed.to_bytes_le());
        println!("  das::WeightedTranscript: {:?}", t.elapsed());
    }

    // Restarting the loop here because now it'll grab **arkworks** weighted `ThresholdConfig`s over BN254 instead.
    // Public parameters (incl. dlog table) are built once for all configs and shared across the loop.
    let wcs = test_utils::get_weighted_configs_for_testing();
    let n_max = wcs
        .iter()
        .map(|wc| wc.get_total_weight())
        .max()
        .expect("no configs");
    let pp = chunky::PublicParameters::<Bn254>::with_max_num_shares(n_max);
    let mut rng = thread_rng();
    for wc in wcs {
        println!("\nTesting {wc} PVSS");

        let (d_unsigned_v1, d_unsigned_v2, d_signed_v1, d_signed_v2) =
            test_utils::setup_dealing_chunky_all_four_with_pp(&wc, &pp, &mut rng);

        let t = Instant::now();
        run_weighted_pvss_deal_verify_reconstruct::<
            Bn254,
            chunky::UnsignedWeightedTranscript<Bn254>,
            _,
        >(&wc, d_unsigned_v1, &mut rng);
        println!(
            "  chunky::UnsignedWeightedTranscript<Bn254>: {:?}",
            t.elapsed()
        );

        let t = Instant::now();
        run_weighted_pvss_deal_verify_reconstruct::<
            Bn254,
            chunky::UnsignedWeightedTranscriptv2<Bn254>,
            _,
        >(&wc, d_unsigned_v2, &mut rng);
        println!(
            "  chunky::UnsignedWeightedTranscriptv2<Bn254>: {:?}",
            t.elapsed()
        );

        let t = Instant::now();
        run_weighted_pvss_deal_verify_reconstruct::<
            Bn254,
            chunky::SignedWeightedTranscript<Bn254>,
            _,
        >(&wc, d_signed_v1, &mut rng);
        println!(
            "  chunky::SignedWeightedTranscript<Bn254>: {:?}",
            t.elapsed()
        );

        let t = Instant::now();
        run_weighted_pvss_deal_verify_reconstruct::<
            Bn254,
            chunky::SignedWeightedTranscriptv2<Bn254>,
            _,
        >(&wc, d_signed_v2, &mut rng);
        println!(
            "  chunky::SignedWeightedTranscriptv2<Bn254>: {:?}",
            t.elapsed()
        );
    }
}

// Disabling this test for now because it's not testing anything
#[test]
#[ignore] // Run with: cargo test test_pvss_transcript_size -- --ignored --nocapture
fn test_pvss_transcript_size() {
    for sc in get_threshold_configs_for_benchmarking() {
        println!();
        let expected_size = expected_transcript_size::<das::Transcript>(&sc);
        let actual_size = actual_transcript_size::<das::Transcript>(&sc);

        print_transcript_size::<das::Transcript>("Expected", &sc, expected_size);
        print_transcript_size::<das::Transcript>("Actual", &sc, actual_size);
    }

    // Restarting the loop here because now it'll grab **arkworks** `ThresholdConfig`s with BN254
    // uses default chunk sizes, so probably want to modify this at some point to allow a wider range
    // Ideally should iterate over a vec of (t, n), not the actual threshold configs... but won't be a bottleneck
    // TODO: this loop isn't testing anything...
    for sc in get_weighted_configs_for_benchmarking().iter().take(1) {
        // Only trying 1 for now to keep tests fast (also the second one has the same n, which means it would yield the same size...)
        println!();
        let actual_size =
            actual_transcript_size::<chunky::UnsignedWeightedTranscript<ark_bn254::Bn254>>(&sc);
        print_transcript_size::<chunky::UnsignedWeightedTranscript<ark_bn254::Bn254>>(
            "Actual for BN254",
            &sc,
            actual_size,
        ); // TODO: also do signed here? or only do signed?
    }

    // Restarting so it grabs BLS12-381 instead of BN254... TODO: could get rid of this with some work
    // TODO: this loop isn't testing anything...
    for sc in get_weighted_configs_for_benchmarking().iter().take(1) {
        // Only trying 1 for now to keep tests fast (also the second one has the same n, which means it would yield the same size...)

        println!();
        let actual_size = actual_transcript_size::<
            chunky::UnsignedWeightedTranscript<ark_bls12_381::Bls12_381>,
        >(&sc);
        print_transcript_size::<chunky::UnsignedWeightedTranscript<ark_bls12_381::Bls12_381>>(
            "Actual for BLS12_381",
            &sc,
            actual_size,
        );
    }

    for wc in get_weighted_configs_for_benchmarking() {
        // TODO: this loop isn't testing anything...
        let actual_size = actual_transcript_size::<das::Transcript>(wc.get_threshold_config());
        print_transcript_size::<das::Transcript>("Actual", wc.get_threshold_config(), actual_size);

        let actual_size = actual_transcript_size::<das::WeightedTranscript>(&wc);
        print_transcript_size::<das::WeightedTranscript>("Actual", &wc, actual_size);
    }
}

#[cfg(test)]
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
#[cfg(test)]
fn pvss_deal_verify_and_reconstruct<T: AggregatableTranscript>(
    sc: &<T as TranscriptCore>::SecretSharingConfig,
    seed_bytes: [u8; 32],
) {
    // println!();
    // println!("Seed: {}", hex::encode(seed_bytes.as_slice()));
    let mut rng = StdRng::from_seed(seed_bytes);

    let d = test_utils::setup_dealing::<T, StdRng>(sc, None, &mut rng);

    // Test dealing
    let trx = T::deal(
        &sc,
        &d.pp,
        &d.ssks[0],
        &d.spks[0],
        &d.eks,
        &d.s,
        &NoAux,
        &sc.get_player(0),
        &mut rng,
    );
    trx.verify(&sc, &d.pp, &[d.spks[0].clone()], &d.eks, &[NoAux])
        .expect("PVSS transcript failed verification");

    // Test transcript (de)serialization
    let trx_deserialized = T::try_from(trx.to_bytes().as_slice())
        .expect("serialized transcript should deserialize correctly");

    assert_eq!(trx, trx_deserialized);
    if d.dsk != reconstruct_dealt_secret_key_randomly::<StdRng, T>(sc, &mut rng, &d.dks, trx, &d.pp)
    {
        panic!("Reconstructed SK did not match");
    }
}

#[cfg(test)]
#[allow(dead_code)]
fn test_pvss_aggregate_subtranscript_and_decrypt<E: Pairing, T>(
    sc: &WeightedConfigArkworks<E::ScalarField>,
    seed_bytes: [u8; 32],
) where
    T: Transcript<SecretSharingConfig = WeightedConfigArkworks<E::ScalarField>>,
    T: HasAggregatableSubtranscript<
        Subtranscript: Aggregatable<SecretSharingConfig = WeightedConfigArkworks<E::ScalarField>>,
    >,
{
    let mut rng = StdRng::from_seed(seed_bytes); // deterministic rng
                                                 //let mut rng = rand::thread_rng();

    let d = test_utils::setup_dealing_weighted::<E::ScalarField, T, _>(sc, &mut rng);

    let all_trs: Vec<_> = (0..9)
        .map(|i| {
            T::deal(
                &sc,
                &d.pp,
                &d.ssks[i],
                &d.spks[i],
                &d.eks,
                &d.s,
                &NoAux,
                &sc.get_player(i),
                &mut rng,
            )
        })
        .collect();

    // Use the first player's transcript as the accumulator
    let mut agg = all_trs[0].get_subtranscript().to_aggregated();

    // Aggregate all other transcripts into it
    for trs in all_trs.iter().skip(1) {
        agg.aggregate_with(&sc, &trs.get_subtranscript()).unwrap();
    }

    let agg = agg.normalize();

    #[allow(unused_variables)]
    let final_share = agg.decrypt_own_share(sc, &sc.get_player(0), &d.dks[0], &d.pp);

    // TODO: should compare it with sum of shares
}

#[cfg(test)]
#[allow(dead_code)] // We no longer have unweighted nonaggregatable PVSS code
fn nonaggregatable_pvss_deal_verify_and_reconstruct<T: HasAggregatableSubtranscript>(
    sc: &T::SecretSharingConfig,
    seed_bytes: [u8; 32],
) {
    // println!();
    // println!("Seed: {}", hex::encode(seed_bytes.as_slice()));
    let mut rng = StdRng::from_seed(seed_bytes);

    let d = test_utils::setup_dealing::<T, StdRng>(sc, None, &mut rng);

    // Test dealing
    let trx = T::deal(
        &sc,
        &d.pp,
        &d.ssks[0],
        &d.spks[0],
        &d.eks,
        &d.s,
        &NoAux,
        &sc.get_player(0),
        &mut rng,
    );
    trx.verify(&sc, &d.pp, &[d.spks[0].clone()], &d.eks, &NoAux, &mut rng)
        .expect("PVSS transcript failed verification");

    // Test transcript (de)serialization
    let trx_deserialized = T::try_from(trx.to_bytes().as_slice())
        .expect("serialized transcript should deserialize correctly");

    assert_eq!(trx, trx_deserialized);
    if d.dsk != reconstruct_dealt_secret_key_randomly::<StdRng, T>(sc, &mut rng, &d.dks, trx, &d.pp)
    {
        panic!("Reconstructed SK did not match");
    }
}

/// Runs deal, verify, (de)serialization roundtrip, and secret reconstruction for a weighted PVSS
/// transcript. Caller provides pre-built `DealingArgs` (e.g. from `setup_dealing_chunky_all_four` or
/// `setup_dealing_weighted`).
#[cfg(test)]
fn run_weighted_pvss_deal_verify_reconstruct<E, T, R>(
    sc: &WeightedConfigArkworks<E::ScalarField>,
    d: test_utils::DealingArgs<T>,
    rng: &mut R,
) where
    E: Pairing,
    R: RngCore + CryptoRng,
    T: HasAggregatableSubtranscript
        + Transcript<SecretSharingConfig = WeightedConfigArkworks<E::ScalarField>>,
{
    // Test dealing
    let trx = T::deal(
        sc,
        &d.pp,
        &d.ssks[0],
        &d.spks[0],
        &d.eks,
        &d.s,
        &NoAux,
        &sc.get_player(0),
        rng,
    );
    trx.verify(sc, &d.pp, &d.spks, &d.eks, &NoAux, rng)
        .expect("PVSS transcript failed verification");

    // Test transcript (de)serialization
    let trx_deserialized = T::try_from(trx.to_bytes().as_slice())
        .expect("serialized transcript should deserialize correctly");
    assert_eq!(trx, trx_deserialized);

    if d.dsk != reconstruct_dealt_secret_key_randomly::<R, T>(sc, rng, &d.dks, trx.clone(), &d.pp) {
        panic!("Reconstructed SK did not match");
    }
    // Verify that it doesn't matter if we use the subtranscript:
    let trx_again = trx.get_subtranscript();
    if d.dsk != reconstruct_dealt_secret_key_randomly::<R, _>(sc, rng, &d.dks, trx_again, &d.pp) {
        panic!("Reconstructed SK from subtranscript did not match");
    }
}

#[cfg(test)]
fn actual_transcript_size<T: Transcript>(sc: &T::SecretSharingConfig) -> usize {
    let mut rng = thread_rng();

    let trx = T::generate(
        &sc,
        &T::PublicParameters::with_max_num_shares_for_generate(sc.get_total_num_shares()),
        &mut rng,
    );
    let actual_size = trx.to_bytes().len();

    actual_size
}

#[cfg(test)]
fn expected_transcript_size<T: Transcript<SecretSharingConfig = ThresholdConfigBlstrs>>(
    sc: &ThresholdConfigBlstrs,
) -> usize {
    if T::scheme_name() == unweighted_protocol::DAS_SK_IN_G1 {
        G2_PROJ_NUM_BYTES
            + (sc.get_total_num_players() + 1) * (G2_PROJ_NUM_BYTES + G1_PROJ_NUM_BYTES)
    } else {
        panic!("Did not implement support for '{}' yet", T::scheme_name())
    }
}
