// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::ptr_arg)]
#![allow(clippy::needless_borrow)]

use velor_dkg::{
    pvss,
    pvss::{
        test_utils,
        test_utils::{DealingArgs, NoAux},
        traits::{SecretSharingConfig, Transcript},
        Player, WeightedConfig,
    },
    utils::random::random_scalar,
    weighted_vuf::{pinkas::PinkasWUF, traits::WeightedVUF},
};
use velor_runtimes::spawn_rayon_thread_pool;
use rand::{rngs::StdRng, thread_rng};
use rand_core::SeedableRng;
use sha3::{Digest, Sha3_256};

#[test]
fn test_wvuf_basic_viability() {
    weighted_wvuf_bvt::<pvss::das::WeightedTranscript, PinkasWUF>();
}

fn weighted_wvuf_bvt<
    T: Transcript<SecretSharingConfig = WeightedConfig>,
    WVUF: WeightedVUF<
        SecretKey = T::DealtSecretKey,
        PubKey = T::DealtPubKey,
        PubKeyShare = T::DealtPubKeyShare,
        SecretKeyShare = T::DealtSecretKeyShare,
    >,
>()
where
    WVUF::PublicParameters: for<'a> From<&'a T::PublicParameters>,
{
    let mut rng = thread_rng();
    let seed = random_scalar(&mut rng);

    // Do a weighted PVSS
    let mut rng = StdRng::from_seed(seed.to_bytes_le());

    let (wc, d, trx) = weighted_pvss::<T>(&mut rng);

    // Test decrypting SK shares, creating VUF proof shares, and aggregating those shares into a VUF
    // proof, verifying that proof and finally deriving the VUF evaluation.
    wvuf_randomly_aggregate_verify_and_derive_eval::<T, WVUF, StdRng>(
        &wc, &d.dsk, &d.dpk, &d.dks, &d.pp, &trx, &mut rng,
    );
}

fn weighted_pvss<T: Transcript<SecretSharingConfig = WeightedConfig>>(
    rng: &mut StdRng,
) -> (WeightedConfig, DealingArgs<T>, T) {
    let wc = WeightedConfig::new(10, vec![3, 5, 3, 4, 2, 1, 1, 7]).unwrap();

    let d = test_utils::setup_dealing::<T, StdRng>(&wc, rng);

    let trx = T::deal(
        &wc,
        &d.pp,
        &d.ssks[0],
        &d.eks,
        &d.s,
        &NoAux,
        &wc.get_player(0),
        rng,
    );

    // Make sure the PVSS dealt correctly
    trx.verify(&wc, &d.pp, &vec![d.spks[0].clone()], &d.eks, &vec![NoAux])
        .expect("PVSS transcript failed verification");

    (wc, d, trx)
}

/// 1. Evaluates the VUF using the `sk` directly.
/// 2. Picks a random eligible subset of players and aggregates a VUF from it.
/// 3. Checks that the evaluation is the same as that from `sk`.
///
/// `T` is a (non-weighted) `pvss::traits::Transcript` type.
fn wvuf_randomly_aggregate_verify_and_derive_eval<
    T: Transcript<SecretSharingConfig = WeightedConfig>,
    WVUF: WeightedVUF<
        SecretKey = T::DealtSecretKey,
        PubKey = T::DealtPubKey,
        PubKeyShare = T::DealtPubKeyShare,
        SecretKeyShare = T::DealtSecretKeyShare,
    >,
    R: rand_core::RngCore + rand_core::CryptoRng,
>(
    wc: &WeightedConfig,
    sk: &T::DealtSecretKey,
    pk: &T::DealtPubKey,
    dks: &Vec<T::DecryptPrivKey>,
    pvss_pp: &T::PublicParameters,
    trx: &T,
    rng: &mut R,
) where
    WVUF::PublicParameters: for<'a> From<&'a T::PublicParameters>,
{
    // Note: A WVUF scheme needs to implement conversion from all PVSS's public parameters to its own.
    let vuf_pp = WVUF::PublicParameters::from(&pvss_pp);

    let msg = b"some msg";
    let eval = WVUF::eval(&sk, msg.as_slice());

    let (mut sks, pks): (Vec<WVUF::SecretKeyShare>, Vec<WVUF::PubKeyShare>) = (0..wc
        .get_total_num_players())
        .map(|p| {
            let (sk, pk) = trx.decrypt_own_share(&wc, &wc.get_player(p), &dks[p]);
            (sk, pk)
        })
        .collect::<Vec<(WVUF::SecretKeyShare, WVUF::PubKeyShare)>>()
        .into_iter()
        .unzip();

    // we are going to be popping the SKs in reverse below (simplest way to move them out of the Vec)
    sks.reverse();
    let augmented_key_pairs = (0..wc.get_total_num_players())
        .map(|p| {
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
        })
        .collect::<Vec<(WVUF::AugmentedSecretKeyShare, WVUF::AugmentedPubKeyShare)>>();

    let apks = augmented_key_pairs
        .iter()
        .map(|(_, apk)| Some(apk.clone()))
        .collect::<Vec<Option<WVUF::AugmentedPubKeyShare>>>();

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
    let eval_aggrs = [1, 32].map(|num_threads| {
        let pool = spawn_rayon_thread_pool("test-wvuf".to_string(), Some(num_threads));
        WVUF::derive_eval(&wc, &vuf_pp, msg, &apks[..], &proof, &pool)
            .expect("WVUF derivation was expected to succeed")
    });

    // TODO: When APKs are missing, not yet testing proof verification and derivation.

    // Test that we can hash this via, say, SHA3
    let eval_bytes = bcs::to_bytes(&eval).unwrap();
    let _hash = Sha3_256::digest(eval_bytes.as_slice()).to_vec();

    for (i, eval_aggr) in eval_aggrs.into_iter().enumerate() {
        println!("Checking WVUF evaluation #{}", i);
        assert_eq!(eval_aggr, eval);
    }
}
