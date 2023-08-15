// Copyright © Aptos Foundation

use crate::pvss::traits::transcript::Transcript;
use crate::pvss::traits::{Convert, HasEncryptionPublicParams, SecretSharingConfig};
use crate::pvss::{ThresholdConfig, WeightedConfig};
use aptos_crypto::Uniform;
use rand::prelude::ThreadRng;
use rand::thread_rng;

/// Helper function that returns an `(pp, dks, eks, s, sk)` tuple. Useful in tests and benchmarks when
/// wanting to quickly deal & verify a transcript.
pub fn setup_dealing<T: Transcript, R: rand_core::RngCore + rand_core::CryptoRng>(
    sc: &T::SecretSharingConfig,
    mut rng: &mut R,
) -> (
    T::PvssPublicParameters,
    Vec<T::DecryptPrivKey>,
    Vec<T::EncryptPubKey>,
    T::InputSecret,
    T::DealtSecretKey,
) {
    let pp = T::PvssPublicParameters::default();
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

    let s = T::InputSecret::generate(&mut rng);
    let sk: <T as Transcript>::DealtSecretKey = s.to(&pp);
    // println!("Dealt SK: {:?}", sk);

    (pp, dks, eks, s, sk)
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
    for t in 1..20 {
        for n in t..20 {
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
