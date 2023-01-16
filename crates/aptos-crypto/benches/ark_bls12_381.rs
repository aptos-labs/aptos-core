// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate criterion;

use blst::blst_p1;
use aptos_crypto::{
    bls12381,
    bls12381::ProofOfPossession,
    test_utils::{random_keypairs, random_subset, KeyPair},
    traits::{Signature, SigningKey, Uniform},
    PrivateKey,
};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use criterion::{
    measurement::Measurement, AxisScale, BatchSize, BenchmarkGroup, BenchmarkId, Criterion,
    PlotConfiguration, Throughput,
};
use rand::{distributions, rngs::ThreadRng, thread_rng, Rng};
use serde::{Deserialize, Serialize};
use ark_std::test_rng;
use ark_ec::AffineCurve;
use ark_ec::group::Group;
use ark_ec::PairingEngine;
use ark_ec::ProjectiveCurve;
use ark_ff::UniformRand;
use ark_ff::PrimeField;
use std::ops::Mul;

#[derive(Debug, CryptoHasher, BCSCryptoHash, Serialize, Deserialize)]
struct TestAptosCrypto(String);

fn random_bytes(len: usize) -> Vec<u8> {
    thread_rng().sample_iter(&distributions::Alphanumeric)
        .take(len)
        .map(|c|c as u8)
        .collect()
}

fn random_p1() -> blst::blst_p1 {
    let msg = random_bytes(64);
    let dst = random_bytes(64);
    let aug = random_bytes(64);
    let mut point = blst_p1::default();
    unsafe { blst::blst_hash_to_g1(&mut point, msg.as_ptr(), msg.len(), dst.as_ptr(), dst.len(), aug.as_ptr(), aug.len()); }
    point
}

fn bench_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("ark_bls12_381");

    group.throughput(Throughput::Elements(1));

    group.bench_function("g1_proj_scalar_mul", move |b| {
        b.iter_with_setup(
            || {
            },
            |_| {
                let p = ark_bls12_381::G1Projective::rand(&mut test_rng());
                let s = ark_bls12_381::Fr::rand(&mut test_rng());
                let _ = p.mul(s.into_repr());
            }
        )
    });

    for num_pairs in [1,2,4,8] {
        group.bench_function(
            format!("{num_pairs}_pairing_product").as_str(),
            |b| {
                b.iter_with_setup(
                    ||{
                        let inputs: Vec<(ark_ec::models::bls12::g1::G1Prepared<ark_bls12_381::Parameters>, ark_ec::models::bls12::g2::G2Prepared<ark_bls12_381::Parameters>)> = (0
                            ..num_pairs)
                            .map(|i| {
                                let p1 = ark_bls12_381::G1Affine::prime_subgroup_generator()
                                    .mul(ark_bls12_381::Fr::rand(&mut test_rng()))
                                    .into_affine();
                                let p1p = ark_ec::prepare_g1::<ark_bls12_381::Bls12_381>(p1);
                                let p2 = ark_bls12_381::G2Affine::prime_subgroup_generator()
                                    .mul(ark_bls12_381::Fr::rand(&mut test_rng()))
                                    .into_affine();
                                let p2p = ark_ec::prepare_g2::<ark_bls12_381::Bls12_381>(p2);
                                (p1p, p2p)
                            })
                            .collect();
                        inputs
                    },
                    |inputs| {
                        let _product = ark_bls12_381::Bls12_381::product_of_pairings(inputs.as_slice());
                    }
                );
            },
        );
    }

    group.finish();
}

criterion_group!(
    name = ark_bls12_381_benches;
    config = Criterion::default(); //.measurement_time(Duration::from_secs(100));
    targets = bench_group);
criterion_main!(ark_bls12_381_benches);
