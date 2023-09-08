// Copyright © Aptos Foundation

use crate::pvss::traits::transcript::Transcript;
use crate::pvss::traits::{Convert, HasEncryptionPublicParams, SecretSharingConfig};
use crate::pvss::{ThresholdConfig, WeightedConfig};
use aptos_crypto::{SigningKey, Uniform};
use num_traits::Zero;
use rand::prelude::ThreadRng;
use rand::thread_rng;
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
    T::PvssPublicParameters,
    Vec<T::SigningSecretKey>,
    Vec<T::SigningPubKey>,
    Vec<T::DecryptPrivKey>,
    Vec<T::EncryptPubKey>,
    Vec<T::InputSecret>,
    T::InputSecret,
    T::DealtSecretKey,
) {
    let pp = T::PvssPublicParameters::default();

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

#[allow(unused)]
pub(crate) use vec_to_str;

pub fn get_threshold_configs_for_testing() -> Vec<ThresholdConfig> {
    let mut tcs = vec![];

    // tcs.push(ThresholdConfig::new(1, 1).unwrap());
    // tcs.push(ThresholdConfig::new(1, 2).unwrap());
    // for t in [1, 2, 3, 4, 5, 6, 7, 8] {
    //     for n in t..3 * (t - 1) + 1 {
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

    // 1-out-of-2, weights 2
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
    wcs.push(WeightedConfig::new(2, vec![1, 1, 1]).unwrap());

    // 50-out-of-100, weights [11, 13, 9, 10, 12, 8, 7, 14, 10, 6]
    wcs.push(WeightedConfig::new(50, vec![11, 13, 9, 10, 12, 8, 7, 14, 10, 6]).unwrap());

    wcs
}
