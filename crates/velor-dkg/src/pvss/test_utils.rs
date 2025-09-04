// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::pvss::{
    traits::{transcript::Transcript, Convert, HasEncryptionPublicParams, SecretSharingConfig},
    Player, ThresholdConfig, WeightedConfig,
};
use velor_crypto::{hash::CryptoHash, SigningKey, Uniform};
use num_traits::Zero;
use rand::{prelude::ThreadRng, thread_rng};
use serde::Serialize;
use std::ops::AddAssign;

/// Type used to indicate that dealears are not including any auxiliary data in their PVSS transcript
/// signatures.
#[derive(Clone, Serialize)]
pub struct NoAux;

/// Useful for gathering all the necessary args to deal inside tests & benchmarks.
pub struct DealingArgs<T: Transcript> {
    pub pp: T::PublicParameters,
    pub ssks: Vec<T::SigningSecretKey>,
    pub spks: Vec<T::SigningPubKey>,
    pub dks: Vec<T::DecryptPrivKey>,
    pub eks: Vec<T::EncryptPubKey>,
    pub iss: Vec<T::InputSecret>,
    pub s: T::InputSecret,
    pub dsk: T::DealtSecretKey,
    pub dpk: T::DealtPubKey,
}

/// Helper function that, given a sharing configuration for `n` players, returns a tuple of:
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
) -> DealingArgs<T> {
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

    let dpk: <T as Transcript>::DealtPubKey = s.to(&pp);
    let dsk: <T as Transcript>::DealtSecretKey = s.to(&pp);
    // println!("Dealt SK: {:?}", sk);

    assert_eq!(ssks.len(), spks.len());

    DealingArgs {
        pp,
        ssks,
        spks,
        dks,
        eks,
        iss,
        s,
        dsk,
        dpk,
    }
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

    // 7-out-of-15, weights [0, 0, 0, 2, 2, 2, 0, 0, 0, 3, 3, 3, 0, 0, 0]
    wcs.push(WeightedConfig::new(7, vec![0, 0, 0, 2, 2, 2, 0, 0, 0, 3, 3, 3, 0, 0, 0]).unwrap());

    wcs
}

pub fn get_threshold_configs_for_benchmarking() -> Vec<ThresholdConfig> {
    // [XDL+24] The Latency Price of Threshold Cryptosystem in Blockchains; by Zhuolun Xiang et al; 2024
    vec![
        ThresholdConfig::new(143, 254).unwrap(), // from XDL+24
        ThresholdConfig::new(184, 254).unwrap(), // from XDL+24
        ThresholdConfig::new(548, 821).unwrap(), // from initial deployment
        ThresholdConfig::new(333, 1_000).unwrap(),
        ThresholdConfig::new(666, 1_000).unwrap(),
        ThresholdConfig::new(3_333, 10_000).unwrap(),
        ThresholdConfig::new(6_666, 10_000).unwrap(),
    ]
}

pub fn get_weighted_configs_for_benchmarking() -> Vec<WeightedConfig> {
    let mut wcs = vec![];

    let weights = vec![
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 4, 4, 4,
        4, 4, 4, 5, 5, 5, 6, 6, 6, 7, 7, 7, 9, 10, 14, 14, 15, 15, 15, 15, 15, 15, 16, 16, 16, 17,
        17, 17, 17, 17, 17, 18, 18, 18, 18, 18, 18, 18, 18, 18, 18, 18, 18, 18, 18, 18, 19, 19, 20,
        20, 20, 20,
    ];
    let total_weight: usize = weights.iter().sum();
    let threshold = total_weight * 2 / 3 + 1;
    wcs.push(WeightedConfig::new(threshold, weights).unwrap());

    let weights = vec![
        3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
        3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4, 4, 5, 5, 5, 5, 5, 5, 6, 7, 7, 10, 11, 11,
        11, 11, 11, 13, 14, 14, 15, 18, 18, 20, 20, 20, 22, 28, 31, 42, 44, 44, 44, 45, 46, 46, 46,
        47, 47, 48, 50, 51, 51, 51, 51, 52, 54, 54, 54, 54, 54, 54, 54, 54, 54, 54, 54, 54, 54, 54,
        54, 57, 57, 60, 60, 60, 60,
    ];
    let total_weight: usize = weights.iter().sum();
    let threshold = total_weight * 2 / 3 + 1;
    wcs.push(WeightedConfig::new(threshold, weights).unwrap());

    let weights = vec![
        5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
        5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 6, 6, 6, 6, 6, 8, 8, 8, 8, 8, 9, 11, 11, 12, 16, 18,
        18, 18, 18, 19, 22, 23, 23, 25, 29, 30, 32, 33, 34, 36, 46, 51, 69, 72, 72, 73, 73, 76, 76,
        76, 77, 78, 79, 82, 84, 84, 84, 84, 86, 89, 89, 89, 89, 89, 89, 89, 89, 89, 89, 89, 89, 89,
        89, 89, 93, 94, 98, 98, 98, 98,
    ];
    let total_weight: usize = weights.iter().sum();
    let threshold = total_weight * 2 / 3 + 1;
    wcs.push(WeightedConfig::new(threshold, weights).unwrap());

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
