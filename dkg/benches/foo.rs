// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate criterion;

use std::sync::Arc;
use criterion::{BenchmarkId, Criterion};
use rand::{Rng, thread_rng};
use aptos_crypto::{bls12381, Uniform};
use aptos_crypto::bls12381::{PrivateKey, PublicKey};
use aptos_dkg_runtime::dkg_manager::setup_deal;
use aptos_types::dkg::DKGSessionMetadata;
use aptos_types::dkg::real_dkg::RealDKG;
use aptos_types::dkg::real_dkg::rounding::MAINNET_STAKES;
use aptos_types::on_chain_config::OnChainRandomnessConfig;
use aptos_types::validator_verifier::{ValidatorConsensusInfo, ValidatorConsensusInfoMoveStruct};
use move_core_types::account_address::AccountAddress;

fn setup_0(rand_config: OnChainRandomnessConfig) -> (AccountAddress, usize, Arc<PrivateKey>, DKGSessionMetadata) {
    let mut rng = thread_rng();
    let n = MAINNET_STAKES.len();
    let my_index = rng.gen_range(0, n);
    let addresses: Vec<AccountAddress> = (0..n).map(|_| AccountAddress::random()).collect();
    let private_keys: Vec<Arc<PrivateKey>> = (0..n).map(|_|Arc::new(PrivateKey::generate(&mut rng))).collect();
    let public_keys : Vec<PublicKey> = (0..n).map(|i|PublicKey::from(private_keys[i].as_ref())).collect();
    let validator_info_vec: Vec<ValidatorConsensusInfoMoveStruct> = (0..n).map(|i| ValidatorConsensusInfo::new(addresses[i], public_keys[i].clone(), MAINNET_STAKES[i].clone()).into()).collect();
    let session_metadata = DKGSessionMetadata {
        dealer_epoch: 999,
        randomness_config: rand_config.into(),
        dealer_validator_set: validator_info_vec.clone(),
        target_validator_set: validator_info_vec,
    };
    (addresses[my_index], my_index, private_keys[my_index].clone(), session_metadata)
}

fn bench_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("foo");
    group.bench_function("v1_setup_deal", move |b| {
        b.iter_with_setup(
            || {
                setup_0(OnChainRandomnessConfig::default_v1())
            },
            |(my_addr, my_index, my_sk, session_metadata)| {
                let _ = setup_deal::<RealDKG>(my_addr, my_index, my_sk, &session_metadata);
            }
        )
    });

    group.bench_function("v2_setup_deal", move |b| {
        b.iter_with_setup(
            || {
                setup_0(OnChainRandomnessConfig::default_enabled())
            },
            |(my_addr, my_index, my_sk, session_metadata)| {
                let _ = setup_deal::<RealDKG>(my_addr, my_index, my_sk, &session_metadata);
            }
        )
    });
    group.finish();
}

criterion_group!(
    name = foo_benches;
    config = Criterion::default();
    targets = bench_group);
criterion_main!(foo_benches);
