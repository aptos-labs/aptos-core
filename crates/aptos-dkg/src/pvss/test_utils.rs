// Copyright © Aptos Foundation

use crate::pvss::{
    traits::{transcript::Transcript, Convert, HasEncryptionPublicParams, SecretSharingConfig},
    Player, ThresholdConfig, WeightedConfig,
};
use aptos_crypto::{hash::CryptoHash, SigningKey, Uniform};
use num_traits::Zero;
use rand::{prelude::ThreadRng, thread_rng};
use serde::Serialize;
use std::ops::AddAssign;

/// Type used to indicate that dealears are not including any auxiliary data in their PVSS transcript
/// signatures.
#[derive(Clone, Serialize)]
pub struct NoAux;

/// Helper function that, given a sharing configuration for `n` players, returns an a tuple of:
///  - public parameters
///  - a vector of `n` signing SKs
///  - a vector of `n` signing PKs
///  - a vector of `n` decryption SKs
///  - a vector of `n` encryption PKs
///  - a vector of `n` input secrets, denoted by `iss`
///  - the aggregated dealt secret key from `\sum_i iss[i]`
/// Useful in tests and benchmarks when wanting to quickly deal & verify a transcript.
pub fn setup_dealing<T: Transcript, R: rand_core::RngCore + rand_core::CryptoRng>(
    sc: &T::SecretSharingConfig,
    mut rng: &mut R,
) -> (
    T::PublicParameters,
    Vec<T::SigningSecretKey>,
    Vec<T::SigningPubKey>,
    Vec<T::DecryptPrivKey>,
    Vec<T::EncryptPubKey>,
    Vec<T::InputSecret>,
    T::InputSecret,
    T::DealtSecretKey,
) {
    println!(
        "Setting up dealing for {} PVSS, with {}",
        T::scheme_name(),
        sc
    );

    let pp = T::PublicParameters::default();

    let ssks = (0..sc.get_total_num_players())
        .map(|_| T::SigningSecretKey::generate(&mut rng))
        .collect::<Vec<T::SigningSecretKey>>();
    let spks = ssks
        .iter()
        .map(|ssk| ssk.verifying_key())
        .collect::<Vec<T::SigningPubKey>>();

    let dks = (0..sc.get_total_num_players())
        .map(|_| T::DecryptPrivKey::generate(&mut rng))
        .collect::<Vec<T::DecryptPrivKey>>();
    let eks = dks
        .iter()
        .map(|dk| dk.to(&pp.get_encryption_public_params()))
        .collect();

    // println!();
    // println!("DKs: {:?}", dks);
    // println!("EKs: {:?}", eks);

    let iss = (0..sc.get_total_num_players())
        .map(|_| T::InputSecret::generate(&mut rng))
        .collect::<Vec<T::InputSecret>>();

    let mut s = T::InputSecret::zero();
    for is in &iss {
        s.add_assign(is)
    }
    let sk: <T as Transcript>::DealtSecretKey = s.to(&pp);
    // println!("Dealt SK: {:?}", sk);

    (pp, ssks, spks, dks, eks, iss, s, sk)
}

/// Useful for printing types of variables without too much hassle.
pub fn print_type_of<T>(_: &T) {
    println!("{}", std::any::type_name::<T>())
}

pub fn get_threshold_config_and_rng(t: usize, n: usize) -> (ThresholdConfig, ThreadRng) {
    let sc = ThresholdConfig::new(t, n).unwrap();

    (sc, thread_rng())
}

#[allow(unused)]
macro_rules! vec_to_str {
    ($vec:ident) => {
        $vec.iter()
            .map(|e| format!("{}", e))
            .collect::<Vec<String>>()
            .join(", ")
    };
}

use crate::pvss::traits::Reconstructable;
#[allow(unused)]
pub(crate) use vec_to_str;

pub fn get_threshold_configs_for_testing() -> Vec<ThresholdConfig> {
    let mut tcs = vec![];

    for t in 1..8 {
        for n in t..8 {
            let tc = ThresholdConfig::new(t, n).unwrap();
            tcs.push(tc)
        }
    }

    tcs
}

pub fn get_weighted_configs_for_testing() -> Vec<WeightedConfig> {
    let mut wcs = vec![];

    // 1-out-of-1 weighted
    wcs.push(WeightedConfig::new(1, vec![1]).unwrap());

    // 1-out-of-2, weights 2 0
    wcs.push(WeightedConfig::new(1, vec![2]).unwrap());
    // 1-out-of-2, weights 1 1
    wcs.push(WeightedConfig::new(1, vec![1, 1]).unwrap());
    // 2-out-of-2, weights 1 1
    wcs.push(WeightedConfig::new(2, vec![1, 1]).unwrap());

    // 1-out-of-3, weights 1 1 1
    wcs.push(WeightedConfig::new(1, vec![1, 1, 1]).unwrap());
    // 2-out-of-3, weights 1 1 1
    wcs.push(WeightedConfig::new(2, vec![1, 1, 1]).unwrap());
    // 3-out-of-3, weights 1 1 1
    wcs.push(WeightedConfig::new(3, vec![1, 1, 1]).unwrap());

    // 3-out-of-5, weights 2 1 2
    wcs.push(WeightedConfig::new(3, vec![2, 1, 2]).unwrap());

    // 3-out-of-7, weights 2 3 2
    wcs.push(WeightedConfig::new(3, vec![2, 3, 2]).unwrap());

    // 50-out-of-100, weights [11, 13, 9, 10, 12, 8, 7, 14, 10, 6]
    wcs.push(WeightedConfig::new(50, vec![11, 13, 9, 10, 12, 8, 7, 14, 10, 6]).unwrap());

    wcs
}

pub fn get_threshold_configs_for_benchmarking() -> Vec<ThresholdConfig> {
    vec![
        ThresholdConfig::new(333, 1_000).unwrap(),
        ThresholdConfig::new(666, 1_000).unwrap(),
        ThresholdConfig::new(3_333, 10_000).unwrap(),
        ThresholdConfig::new(6_666, 10_000).unwrap(),
    ]
}

pub fn get_weighted_configs_for_benchmarking() -> Vec<WeightedConfig> {
    let mut wcs = vec![];

    // Total weight is 9230
    let weights = vec![
        17, 17, 11, 11, 11, 74, 40, 11, 11, 11, 11, 11, 218, 218, 218, 218, 218, 218, 218, 170, 11,
        11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 18, 11, 11, 11, 192, 218, 11, 11, 52, 11,
        161, 24, 11, 11, 11, 11, 218, 218, 161, 175, 80, 13, 103, 11, 11, 11, 11, 40, 40, 40, 14,
        218, 218, 11, 218, 11, 11, 218, 11, 218, 71, 55, 218, 184, 170, 11, 218, 218, 164, 177,
        171, 18, 209, 11, 20, 12, 147, 18, 169, 13, 35, 208, 13, 218, 218, 218, 218, 218, 218, 163,
        73, 26,
    ];
    wcs.push(WeightedConfig::new(3087, weights.clone()).unwrap());
    wcs.push(WeightedConfig::new(6162, weights).unwrap());

    // Total weight is 850
    let weights = vec![
        2, 2, 1, 1, 1, 7, 4, 1, 1, 1, 1, 1, 20, 20, 20, 20, 20, 20, 20, 16, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 2, 1, 1, 1, 18, 20, 1, 1, 5, 1, 15, 2, 1, 1, 1, 1, 20, 20, 15, 16, 7, 1, 9,
        1, 1, 1, 1, 4, 4, 4, 1, 20, 20, 1, 20, 1, 1, 20, 1, 20, 7, 5, 20, 17, 16, 1, 20, 20, 15,
        16, 16, 2, 19, 1, 2, 1, 13, 2, 16, 1, 3, 19, 1, 20, 20, 20, 20, 20, 20, 15, 7, 2,
    ];
    wcs.push(WeightedConfig::new(290, weights.clone()).unwrap());
    wcs.push(WeightedConfig::new(573, weights).unwrap());

    wcs
}

pub fn reconstruct_dealt_secret_key_randomly<R, T: Transcript + CryptoHash>(
    sc: &<T as Transcript>::SecretSharingConfig,
    rng: &mut R,
    dks: &Vec<<T as Transcript>::DecryptPrivKey>,
    trx: T,
) -> <T as Transcript>::DealtSecretKey
where
    R: rand_core::RngCore,
{
    // Test reconstruction from t random shares
    let players_and_shares = sc
        .get_random_eligible_subset_of_players(rng)
        .into_iter()
        .map(|p| {
            let (sk, pk) = trx.decrypt_own_share(sc, &p, &dks[p.get_id()]);

            assert_eq!(pk, trx.get_public_key_share(sc, &p));

            (p, sk)
        })
        .collect::<Vec<(Player, T::DealtSecretKeyShare)>>();

    T::DealtSecretKey::reconstruct(sc, &players_and_shares)
}
