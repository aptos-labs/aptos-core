// Copyright © Aptos Foundation

//! PVSS scheme-independent testing
use aptos_dkg::constants::{
    BEST_CASE_N, BEST_CASE_THRESHOLD, DST_PVSS_TESTING_APP, G1_PROJ_NUM_BYTES, G2_PROJ_NUM_BYTES,
    WORST_CASE_N, WORST_CASE_THRESHOLD,
};
use aptos_dkg::pvss;
use aptos_dkg::pvss::traits::transcript::Transcript;
use aptos_dkg::pvss::traits::{Reconstructable, SecretSharingConfig};
use aptos_dkg::pvss::{das, scrape, test_utils, WeightedConfig, WeightedTranscript};
use aptos_dkg::pvss::{Player, ThresholdConfig};
use aptos_dkg::utils::random::random_scalar;
use rand::rngs::{StdRng, ThreadRng};
use rand::thread_rng;
use rand_core::SeedableRng;
use aptos_crypto::{bls12381, SigningKey, Uniform};

#[test]
fn all_unweighted_pvss_bvt() {
    let mut rng = thread_rng();

    //
    // Unweighted PVSS tests
    //
    for tc in test_utils::get_threshold_configs_for_testing() {
        println!("\nTesting {tc} PVSS");

        let seed = random_scalar(&mut rng);

        // SCRAPE
        pvss_deal_verify_aggr_and_reconstruct::<pvss::scrape::Transcript>(&tc, seed.to_bytes_le());

        // Das
        pvss_deal_verify_aggr_and_reconstruct::<pvss::das::Transcript>(&tc, seed.to_bytes_le());
    }
}

#[test]
fn all_weighted_pvss_bvt() {
    let mut rng = thread_rng();

    //
    // PVSS weighted tests
    //
    for wc in test_utils::get_weighted_configs_for_testing() {
        println!("\nTesting {wc} PVSS");

        // SCRAPE
        let seed = random_scalar(&mut rng);
        pvss_deal_verify_aggr_and_reconstruct::<WeightedTranscript<pvss::scrape::Transcript>>(
            &wc,
            seed.to_bytes_le(),
        );

        // Das
        pvss_deal_verify_aggr_and_reconstruct::<WeightedTranscript<pvss::das::Transcript>>(
            &wc,
            seed.to_bytes_le(),
        );
    }
}

#[test]
/// Currently, this test no longer fails because we wrapped all buggy `G1Projective::multi_exp` and
/// `G2Projective::multi_exp` calls in the code to deal with the size-1 bug.
fn weighted_fail_due_to_blst_bug() {
    // TODO(blst_hell): See README.md. If I comment this out, everything works. If I don't, things fail
    // non-deterministically because of a `blst` bug for size-1 multiexps. (God only knows why it only
    // gets triggered sometimes when this code was NOT commented.)
    for _tc in test_utils::get_threshold_configs_for_testing() {
        println!("Creating a {_tc} config");
    }

    //
    // SCRAPE weighted tests
    //
    let mut rng = thread_rng();
    let mut attempt = 1;
    let wc = WeightedConfig::new(1, vec![1]).unwrap();
    while attempt < 1000 {
        println!("\nTesting {wc} PVSS: Attempt {attempt}");
        attempt += 1;

        let seed = random_scalar(&mut rng);
        pvss_deal_verify_aggr_and_reconstruct::<WeightedTranscript<pvss::scrape::Transcript>>(
            &wc,
            seed.to_bytes_le(),
        );
    }
}

#[test]
fn transcript_can_be_signed() {
    let mut rng = thread_rng();

    let sc = ThresholdConfig::new(10, 20).unwrap();
    let (pp, _, eks, s, _) = test_utils::setup_dealing::<das::Transcript, ThreadRng>(&sc, &mut rng);
    let trx = das::Transcript::deal(&sc, &pp, &eks, &s, &DST_PVSS_TESTING_APP[..], &mut rng);

    let sk = bls12381::PrivateKey::generate(&mut rng);
    let _ = sk.sign(&trx).unwrap();
}

#[test]
fn transcript_size() {
    for (t, n) in [
        (BEST_CASE_THRESHOLD, BEST_CASE_N),
        (WORST_CASE_THRESHOLD, WORST_CASE_N),
    ] {
        println!();
        print_transcript_size::<pvss::scrape::Transcript>(t, n);
        println!();
        print_transcript_size::<pvss::das::Transcript>(t, n);
    }
}

#[test]
fn pok_of_input_secret_test() {
    panic!("This test is just a reminder that all implemented PVSS schemes are not safe for a PVSS-based DKG without a PoK of the dealt input secret. Consult [GJM+21e] in README.md.")
}

fn print_transcript_size<T: Transcript<SecretSharingConfig = ThresholdConfig>>(t: usize, n: usize) {
    let name = T::scheme_name();
    let expected_size = expected_transcript_size::<T>(t, n);
    let actual_size = actual_transcript_size::<T>(t, n);

    println!("Expected transcript size for {t}-out-of-{n} {name}: {expected_size} bytes");
    println!("Actual   transcript size for {t}-out-of-{n} {name}: {actual_size} bytes");
}

//
// Helper functions
//

/// Basic viability test for a PVSS transcript (weighted or unweighted):
///  1. Deals a secret, creating a transcript
///  2. Verifies the transcript.
///  3. Ensures the a sufficiently-large random subset of the players can recover the dealt secret
fn pvss_deal_verify_aggr_and_reconstruct<T: Transcript>(
    sc: &T::SecretSharingConfig,
    seed_bytes: [u8; 32],
) {
    // println!();
    // println!("Seed: {}", hex::encode(seed_bytes.as_slice()));
    let mut rng = StdRng::from_seed(seed_bytes);

    // TODO: Change this to return multiple InputSecrets, and their sum as the DealtSK. Then, test the secret reconstruction from the aggregated transcript shares.
    let (pp, dks, eks, s, sk) = test_utils::setup_dealing::<T, StdRng>(sc, &mut rng);

    let mut trx1 = T::deal(&sc, &pp, &eks, &s, &DST_PVSS_TESTING_APP[..], &mut rng);
    let trx2 = T::deal(&sc, &pp, &eks, &s, &DST_PVSS_TESTING_APP[..], &mut rng);
    trx1.verify(&sc, &pp, &eks, &DST_PVSS_TESTING_APP[..])
        .expect("PVSS transcript failed verification");

    // Test transcript (de)serialization
    let serialized = trx1.to_bytes();
    let deserialized = T::try_from(serialized.as_slice())
        .expect("serialized transcript should deserialize correctly");

    assert_eq!(trx1, deserialized);

    // Test reconstruction from t random shares
    let players_and_shares = sc
        .get_random_subset_of_capable_players(&mut rng)
        .into_iter()
        .map(|p| {
            let (sk, _) = trx1.decrypt_own_share(&sc, &p, &dks[p.get_id()]);

            (p, sk)
        })
        .collect::<Vec<(Player, T::DealtSecretKeyShare)>>();

    let sk_reconstruct = T::DealtSecretKey::reconstruct(&sc, &players_and_shares);

    // println!();
    assert_eq!(sk, sk_reconstruct);
    // println!("Reconstructed {:?}", sk_reconstruct);

    // Test aggregation
    trx1.aggregate_with(sc, &trx2);
    trx1.verify(sc, &pp, &eks, &DST_PVSS_TESTING_APP[..])
        .expect("aggregated PVSS transcript failed verification");
}

fn actual_transcript_size<T: Transcript<SecretSharingConfig = ThresholdConfig>>(
    t: usize,
    n: usize,
) -> usize {
    let (sc, mut rng) = test_utils::get_threshold_config_and_rng(t, n);

    let trx = T::generate(&sc, &mut rng);
    let actual_size = trx.to_bytes().len();

    actual_size
}

fn expected_transcript_size<T: Transcript<SecretSharingConfig = ThresholdConfig>>(
    _t: usize,
    n: usize,
) -> usize {
    if T::scheme_name() == scrape::SCRAPE_SK_IN_G2 {
        2 * G2_PROJ_NUM_BYTES + n * (G2_PROJ_NUM_BYTES + G1_PROJ_NUM_BYTES)
    } else if T::scheme_name() == das::DAS_SK_IN_G1 {
        G2_PROJ_NUM_BYTES + (n + 1) * (G2_PROJ_NUM_BYTES + G1_PROJ_NUM_BYTES)
    } else {
        panic!("Did not implement support for '{}' yet", T::scheme_name())
    }
}
