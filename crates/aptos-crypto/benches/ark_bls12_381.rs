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
use ark_bls12_381::{Fr, G1Affine};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};

#[derive(Debug, CryptoHasher, BCSCryptoHash, Serialize, Deserialize)]
struct TestAptosCrypto(String);

fn random_bytes(len: usize) -> Vec<u8> {
    thread_rng().sample_iter(&distributions::Alphanumeric)
        .take(len)
        .map(|c|c as u8)
        .collect()
}

fn bench_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("ark_bls12_381");

    group.bench_function("fr_serialize", move |b| {
        b.iter_with_setup(
            || {
                Fr::rand(&mut test_rng())
            },
            |k| {
                let mut buf = vec![];
                k.serialize_uncompressed(&mut buf).unwrap();
            }
        )
    });

    group.bench_function("g1_affine_serialize_uncomp", move |b| {
        b.iter_with_setup(
            || {
                let k = Fr::rand(&mut test_rng());
                G1Affine::prime_subgroup_generator().mul(k).into_affine()
            },
            |p_affine| {
                let mut buf = vec![];
                p_affine.serialize_uncompressed(&mut buf).unwrap();
            }
        )
    });

    group.bench_function("g1_affine_deserialize_uncomp", move |b| {
        b.iter_with_setup(
            || {
                let k = Fr::rand(&mut test_rng());
                let mut buf = vec![];
                G1Affine::prime_subgroup_generator().mul(k).into_affine().serialize_uncompressed(&mut buf);
                buf
            },
            |buf| {
                let _p = G1Affine::deserialize_unchecked(buf.as_slice()).unwrap();
            }
        )
    });

    group.bench_function("g1_proj_scalar_mul", move |b| {
        b.iter_with_setup(
            || {
                let p_proj = ark_bls12_381::G1Projective::rand(&mut test_rng());
                let scalar = Fr::rand(&mut test_rng());
                (p_proj, scalar)
            },
            |(p_proj, scalar)| {
                let _ = p_proj.mul(scalar.into_repr());
            }
        )
    });

    group.bench_function("g1_proj_add", move |b| {
        b.iter_with_setup(
            || {
                let p_proj_0 = ark_bls12_381::G1Projective::rand(&mut test_rng());
                let p_proj_1 = ark_bls12_381::G1Projective::rand(&mut test_rng());
                (p_proj_0, p_proj_1)
            },
            |(p_proj_0, p_proj_1)| {
                let _ = p_proj_0 + p_proj_1;
            }
        )
    });

    group.bench_function("g1_proj_to_affine", move |b| {
        b.iter_with_setup(
            || {
                ark_bls12_381::G1Projective::rand(&mut test_rng())
            },
            |p_proj| {
                let _ = p_proj.into_affine();
            }
        )
    });

    group.bench_function("g1_affine_to_proj", move |b| {
        b.iter_with_setup(
            || {
                G1Affine::prime_subgroup_generator().mul(Fr::rand(&mut test_rng())).into_affine()
            },
            |p_affine| {
                let _ = p_affine.into_projective();
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
                                    .mul(Fr::rand(&mut test_rng()))
                                    .into_affine();
                                let p1p = ark_ec::prepare_g1::<ark_bls12_381::Bls12_381>(p1);
                                let p2 = ark_bls12_381::G2Affine::prime_subgroup_generator()
                                    .mul(Fr::rand(&mut test_rng()))
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
