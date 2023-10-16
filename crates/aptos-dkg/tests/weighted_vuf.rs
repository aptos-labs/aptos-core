use aptos_dkg::pvss;
use aptos_dkg::pvss::test_utils::NoAux;
use aptos_dkg::pvss::traits::{SecretSharingConfig, Transcript};
use aptos_dkg::pvss::{test_utils, Player, ThresholdConfig, WeightedConfig, WeightedTranscript};
use aptos_dkg::utils::random::random_scalar;
use aptos_dkg::weighted_vuf::pinkas::PinkasWUF;
use aptos_dkg::weighted_vuf::traits::WeightedUF;
use rand::rngs::StdRng;
use rand::thread_rng;
use rand_core::SeedableRng;

#[test]
fn weighted_vuf_bvt() {
    let mut rng = thread_rng();
    let seed = random_scalar(&mut rng);

    // Do a weighted PVSS
    let mut rng = StdRng::from_seed(seed.to_bytes_le());

    // TODO: add more weighted config cases
    let (wc, pvss_pp, dks, sk, trx) = weighted_pvss::<pvss::das::Transcript>(&mut rng);

    // Test decrypting SK shares, creating VUF proof shares, and aggregating those shares into a VUF
    wuf_aggregation_test::<pvss::das::Transcript, PinkasWUF, StdRng>(
        &wc, &sk, &dks, &pvss_pp, &trx, &mut rng,
    );

    // TODO: Test verification of an aggregated VUF, if available

    // TODO: Test verification of a single-signer VUF, if available
}

fn weighted_pvss<T: Transcript<SecretSharingConfig = ThresholdConfig>>(
    mut rng: &mut StdRng,
) -> (
    WeightedConfig,
    T::PublicParameters,
    Vec<T::DecryptPrivKey>,
    T::DealtSecretKey,
    WeightedTranscript<T>,
) {
    let wc = WeightedConfig::new(10, vec![3, 5, 3, 4, 2, 1, 1, 7]).unwrap();

    let (pvss_pp, ssks, spks, dks, eks, _, s, sk) =
        test_utils::setup_dealing::<WeightedTranscript<T>, StdRng>(&wc, &mut rng);

    let trx = WeightedTranscript::<T>::deal(
        &wc,
        &pvss_pp,
        &ssks[0],
        &eks,
        &s,
        &NoAux,
        &wc.get_player(0),
        &mut rng,
    );

    // Make sure the PVSS dealt correctly
    trx.verify(&wc, &pvss_pp, &vec![spks[0].clone()], &eks, &vec![NoAux])
        .expect("PVSS transcript failed verification");
    (wc, pvss_pp, dks, sk, trx)
}

/// 1. Evaluates the VUF using the `sk` directly.
/// 2. Picks a random eligible subset of players and aggregates a VUF from it.
/// 3. Checks that the evaluation is the same as that from `sk`.
///
/// `T` is a (non-weighted) `pvss::traits::Transcript` type.
fn wuf_aggregation_test<
    T: Transcript<SecretSharingConfig = ThresholdConfig>,
    WUF: WeightedUF<
        SecretKey = T::DealtSecretKey,
        PubKeyShare = Vec<T::DealtPubKeyShare>,
        SecretKeyShare = Vec<T::DealtSecretKeyShare>,
    >,
    R: rand_core::RngCore + rand_core::CryptoRng,
>(
    wc: &WeightedConfig,
    sk: &T::DealtSecretKey,
    dks: &Vec<T::DecryptPrivKey>,
    pvss_pp: &T::PublicParameters,
    trx: &WeightedTranscript<T>,
    rng: &mut R,
) where
    WUF::PublicParameters: for<'a> From<&'a T::PublicParameters>,
{
    // Note: A WVUF scheme needs to implement conversion from all PVSS's public parameters to its own.
    let vuf_pp = WUF::PublicParameters::from(&pvss_pp);

    let msg = b"some msg";
    let eval = WUF::eval(&sk, msg.as_slice());

    let apks_and_proofs = wc
        .get_random_eligible_subset_of_players(rng)
        .into_iter()
        .map(|p| {
            let (sk, pk) = trx.decrypt_own_share(&wc, &p, &dks[p.get_id()]);

            let (ask, apk) = WUF::augment_key_pair(&vuf_pp, sk, pk.clone(), rng);

            // Test that pubkey augmentation works
            let delta = WUF::get_public_delta(&apk);
            assert_eq!(
                apk,
                WUF::augment_pubkey(&vuf_pp, pk, delta.clone()).unwrap()
            );

            let proof = WUF::create_share(&ask, msg);
            assert!(WUF::verify_share(&vuf_pp, &apk, msg, &proof).is_ok());

            (p, apk, proof)
        })
        .collect::<Vec<(Player, WUF::AugmentedPubKeyShare, WUF::ProofShare)>>();

    // Aggregate the VUF from the subset of capable players
    let proof = WUF::aggregate_shares(&wc, &apks_and_proofs);
    let eval_aggr = WUF::derive_eval(msg, &proof);

    assert_eq!(eval_aggr, eval);
}
