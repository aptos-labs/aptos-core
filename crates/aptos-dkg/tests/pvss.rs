// Copyright © Aptos Foundation

//! PVSS scheme-independent testing
use aptos_crypto::hash::CryptoHash;
use aptos_dkg::constants::{
    BEST_CASE_N, BEST_CASE_THRESHOLD, G1_PROJ_NUM_BYTES, G2_PROJ_NUM_BYTES, WORST_CASE_N,
    WORST_CASE_THRESHOLD,
};
use aptos_dkg::pvss;
use aptos_dkg::pvss::test_utils::NoAux;
use aptos_dkg::pvss::traits::transcript::Transcript;
use aptos_dkg::pvss::traits::{Reconstructable, SecretSharingConfig};
use aptos_dkg::pvss::{das, scrape, test_utils, WeightedConfig, WeightedTranscript};
use aptos_dkg::pvss::{Player, ThresholdConfig};
use aptos_dkg::utils::random::random_scalar;
use rand::rngs::StdRng;
use rand::thread_rng;
use rand_core::SeedableRng;

#[test]
fn all_unweighted_pvss_bvt() {
    let mut rng = thread_rng();

    //
    // Unweighted PVSS tests
    //
    let tcs = test_utils::get_threshold_configs_for_testing();
    for tc in tcs {
        println!("\nTesting {tc} PVSS");

        let seed = random_scalar(&mut rng);

        // Das
        pvss_deal_verify_and_reconstruct::<pvss::das::Transcript>(&tc, seed.to_bytes_le());

        // SCRAPE
        pvss_deal_verify_and_reconstruct::<pvss::scrape::Transcript>(&tc, seed.to_bytes_le());
    }
}

#[test]
fn all_weighted_pvss_bvt() {
    let mut rng = thread_rng();

    //
    // PVSS weighted tests
    //
    let wcs = test_utils::get_weighted_configs_for_testing();
    for wc in wcs {
        println!("\nTesting {wc} PVSS");

        // SCRAPE
        let seed = random_scalar(&mut rng);
        pvss_deal_verify_and_reconstruct::<WeightedTranscript<pvss::scrape::Transcript>>(
            &wc,
            seed.to_bytes_le(),
        );

        // Das
        pvss_deal_verify_and_reconstruct::<WeightedTranscript<pvss::das::Transcript>>(
            &wc,
            seed.to_bytes_le(),
        );
    }
}

#[test]
fn all_unweighted_dkg_bvt() {
    let mut rng = thread_rng();
    let tcs = test_utils::get_threshold_configs_for_testing();
    let seed = random_scalar(&mut rng);

    aggregatable_dkg::<pvss::das::Transcript>(tcs.last().unwrap(), seed.to_bytes_le());
    aggregatable_dkg::<pvss::scrape::Transcript>(tcs.last().unwrap(), seed.to_bytes_le());
}

#[test]
fn all_weighted_dkg_bvt() {
    let mut rng = thread_rng();
    let wcs = test_utils::get_weighted_configs_for_testing();
    let seed = random_scalar(&mut rng);

    aggregatable_dkg::<WeightedTranscript<pvss::das::Transcript>>(
        wcs.last().unwrap(),
        seed.to_bytes_le(),
    );
    aggregatable_dkg::<WeightedTranscript<pvss::scrape::Transcript>>(
        wcs.last().unwrap(),
        seed.to_bytes_le(),
    );
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
        pvss_deal_verify_and_reconstruct::<WeightedTranscript<pvss::scrape::Transcript>>(
            &wc,
            seed.to_bytes_le(),
        );
    }
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
fn scrape_sok_of_input_secret_test() {
    panic!("This test is just a reminder that SCRAPE PVSS is not safe to use in a DKG without a SoK of the dealt input secret. Currently, only Das PVSS has this SoK implemented.")
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
fn pvss_deal_verify_and_reconstruct<T: Transcript + CryptoHash>(
    sc: &T::SecretSharingConfig,
    seed_bytes: [u8; 32],
) {
    // println!();
    // println!("Seed: {}", hex::encode(seed_bytes.as_slice()));
    let mut rng = StdRng::from_seed(seed_bytes);

    let (pp, ssks, spks, dks, eks, _, s, sk) = test_utils::setup_dealing::<T, StdRng>(sc, &mut rng);

    // Test dealing
    let trx = T::deal(
        &sc,
        &pp,
        &ssks[0],
        &eks,
        &s,
        &NoAux,
        &sc.get_player(0),
        &mut rng,
    );
    trx.verify(&sc, &pp, &vec![spks[0].clone()], &eks, &vec![NoAux])
        .expect("PVSS transcript failed verification");

    // Test transcript (de)serialization
    let trx_deserialized = T::try_from(trx.to_bytes().as_slice())
        .expect("serialized transcript should deserialize correctly");

    assert_eq!(trx, trx_deserialized);

    assert_dsk_reconstructs(&sc, &mut rng, &dks, sk, trx);
    // println!("Reconstructed {:?}", sk_reconstructed);
}

fn assert_dsk_reconstructs<T: Transcript + CryptoHash>(
    sc: &&<T as Transcript>::SecretSharingConfig,
    mut rng: &mut StdRng,
    dks: &Vec<<T as Transcript>::DecryptPrivKey>,
    sk: <T as Transcript>::DealtSecretKey,
    trx: T,
) {
    // Test reconstruction from t random shares
    let players_and_shares = sc
        .get_random_eligible_subset_of_players(&mut rng)
        .into_iter()
        .map(|p| {
            let (sk, _) = trx.decrypt_own_share(&sc, &p, &dks[p.get_id()]);

            (p, sk)
        })
        .collect::<Vec<(Player, T::DealtSecretKeyShare)>>();

    let sk_reconstructed = T::DealtSecretKey::reconstruct(&sc, &players_and_shares);

    // println!();
    assert_eq!(sk, sk_reconstructed);
}

/// Deals `n` times, aggregates all transcripts, and attempts to reconstruct the secret dealt in this
/// aggregated transcript.
fn aggregatable_dkg<T: Transcript + CryptoHash>(sc: &T::SecretSharingConfig, seed_bytes: [u8; 32]) {
    let mut rng = StdRng::from_seed(seed_bytes);

    let (pp, ssks, spks, dks, eks, iss, _, sk) =
        test_utils::setup_dealing::<T, StdRng>(sc, &mut rng);

    let mut trxs = vec![];

    // Deal `n` transcripts
    for i in 0..sc.get_total_num_players() {
        trxs.push(T::deal(
            &sc,
            &pp,
            &ssks[i],
            &eks,
            &iss[i],
            &NoAux,
            &sc.get_player(i),
            &mut rng,
        ));
    }

    // Aggregate all `n` transcripts
    let trx = T::aggregate(sc, trxs).unwrap();

    // Verify the aggregated transcript
    trx.verify(
        &sc,
        &pp,
        &spks,
        &eks,
        &(0..sc.get_total_num_players())
            .map(|_| NoAux)
            .collect::<Vec<NoAux>>(),
    )
    .expect("aggregated PVSS transcript failed verification");

    assert_dsk_reconstructs(&sc, &mut rng, &dks, sk, trx);
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
