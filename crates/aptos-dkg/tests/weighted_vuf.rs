use aptos_dkg::pvss;
use aptos_dkg::pvss::test_utils::NoAux;
use aptos_dkg::pvss::traits::{Convert, SecretSharingConfig, Transcript};
use aptos_dkg::pvss::{test_utils, Player, ThresholdConfig, WeightedConfig, WeightedTranscript};
use aptos_dkg::utils::random::random_scalar;
use aptos_dkg::weighted_vuf::gjm21_insecure;
use aptos_dkg::weighted_vuf::pinkas::PinkasWUF;
use aptos_dkg::weighted_vuf::traits::WeightedVUF;
use rand::rngs::StdRng;
use rand::thread_rng;
use rand_core::SeedableRng;
use sha3::{Digest, Sha3_256};

#[test]
fn all_weighted_vuf_bvt() {
    weighted_wvuf_bvt::<pvss::das::Transcript, PinkasWUF>();

    weighted_wvuf_bvt::<pvss::scrape::Transcript, gjm21_insecure::g2::GjmInsecureWVUF>();
    weighted_wvuf_bvt::<pvss::das::Transcript, gjm21_insecure::g1::GjmInsecureWVUF>();
}

fn weighted_wvuf_bvt<
    T: Transcript<SecretSharingConfig = ThresholdConfig>,
    WVUF: WeightedVUF<
        SecretKey = T::DealtSecretKey,
        PubKey = T::DealtPubKey,
        PubKeyShare = Vec<T::DealtPubKeyShare>,
        SecretKeyShare = Vec<T::DealtSecretKeyShare>,
    >,
>()
where
    WVUF::PublicParameters: for<'a> From<&'a T::PublicParameters>,
{
    let mut rng = thread_rng();
    let seed = random_scalar(&mut rng);

    // Do a weighted PVSS
    let mut rng = StdRng::from_seed(seed.to_bytes_le());

    // TODO: add more weighted config cases
    let (wc, pvss_pp, dks, sk, pk, trx) = weighted_pvss::<T>(&mut rng);

    // Test decrypting SK shares, creating VUF proof shares, and aggregating those shares into a VUF
    wvuf_aggregation_test::<T, WVUF, StdRng>(&wc, &sk, &pk, &dks, &pvss_pp, &trx, &mut rng);

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
    T::DealtPubKey,
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

    let pk: <T as Transcript>::DealtPubKey = s.to(&pvss_pp);

    // Make sure the PVSS dealt correctly
    trx.verify(&wc, &pvss_pp, &vec![spks[0].clone()], &eks, &vec![NoAux])
        .expect("PVSS transcript failed verification");
    (wc, pvss_pp, dks, sk, pk, trx)
}

/// 1. Evaluates the VUF using the `sk` directly.
/// 2. Picks a random eligible subset of players and aggregates a VUF from it.
/// 3. Checks that the evaluation is the same as that from `sk`.
///
/// `T` is a (non-weighted) `pvss::traits::Transcript` type.
fn wvuf_aggregation_test<
    T: Transcript<SecretSharingConfig = ThresholdConfig>,
    WVUF: WeightedVUF<
        SecretKey = T::DealtSecretKey,
        PubKey = T::DealtPubKey,
        PubKeyShare = Vec<T::DealtPubKeyShare>,
        SecretKeyShare = Vec<T::DealtSecretKeyShare>,
    >,
    R: rand_core::RngCore + rand_core::CryptoRng,
>(
    wc: &WeightedConfig,
    sk: &T::DealtSecretKey,
    pk: &T::DealtPubKey,
    dks: &Vec<T::DecryptPrivKey>,
    pvss_pp: &T::PublicParameters,
    trx: &WeightedTranscript<T>,
    rng: &mut R,
) where
    WVUF::PublicParameters: for<'a> From<&'a T::PublicParameters>,
{
    // Note: A WVUF scheme needs to implement conversion from all PVSS's public parameters to its own.
    let vuf_pp = WVUF::PublicParameters::from(&pvss_pp);

    let msg = b"some msg";
    let eval = WVUF::eval(&sk, msg.as_slice());

    let (mut sks, pks) : (Vec<WVUF::SecretKeyShare>, Vec<WVUF::PubKeyShare>)= (0..wc.get_total_num_players()).map(|p| {
        let (sk, pk) = trx.decrypt_own_share(&wc, &wc.get_player(p), &dks[p]);
        (sk, pk)
    }).collect::<Vec<(WVUF::SecretKeyShare, WVUF::PubKeyShare)>>().into_iter().unzip();

    // we are going to be popping the SKs in reverse below (simplest way to move them out of the Vec)
    sks.reverse();
    let augmented_key_pairs = (0..wc.get_total_num_players()).map(|p| {
        let sk = sks.pop().unwrap();
        let pk = pks[p].clone();
        let (ask, apk) = WVUF::augment_key_pair(&vuf_pp, sk, pk.clone(), rng);

        // Test that pubkey augmentation works
        let delta = WVUF::get_public_delta(&apk);
        assert_eq!(
            apk,
            WVUF::augment_pubkey(&vuf_pp, pk, delta.clone()).unwrap()
        );

        (ask, apk)
    }).collect::<Vec<(WVUF::AugmentedSecretKeyShare, WVUF::AugmentedPubKeyShare)>>();

    let apks = augmented_key_pairs.iter().map(|(_, apk)| Some(apk.clone())).collect::<Vec<Option<WVUF::AugmentedPubKeyShare>>>();

    let apks_and_proofs = wc
        .get_random_eligible_subset_of_players(rng)
        .into_iter()
        .map(|p| {
            let ask = &augmented_key_pairs[p.id].0;
            let apk = augmented_key_pairs[p.id].1.clone();

            let proof = WVUF::create_share(ask, msg);
            WVUF::verify_share(&vuf_pp, &apk, msg, &proof).expect("WVUF proof share should verify");

            (p, apk, proof)
        })
        .collect::<Vec<(Player, WVUF::AugmentedPubKeyShare, WVUF::ProofShare)>>();

    // Aggregate the VUF from the subset of capable players
    let proof = WVUF::aggregate_shares(&wc, &apks_and_proofs);

    // Make sure the aggregated proof is valid
    WVUF::verify_proof(&vuf_pp, pk, &apks[..], msg, &proof)
        .expect("WVUF aggregated proof should verify");

    // Derive the VUF evaluation
    let eval_aggr = WVUF::derive_eval(&wc, &vuf_pp, msg, &apks[..], &proof).expect("WVUF derivation was expected to succeed");

    // TODO: we are not yet testing proof verification and derivation with missing APKs

    // Test that we can hash this via, say, SHA3
    let eval_bytes = bcs::to_bytes(&eval).unwrap();
    let _hash = Sha3_256::digest(eval_bytes.as_slice()).to_vec();

    assert_eq!(eval_aggr, eval);
}
