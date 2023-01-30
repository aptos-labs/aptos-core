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
use ark_ff::{Field, One, UniformRand, Zero};
use ark_ff::PrimeField;
use std::ops::{Mul, Neg};
use ark_bls12_381::{Fq12, Fr, G1Affine, G1Projective};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};

#[derive(Debug, CryptoHasher, BCSCryptoHash, Serialize, Deserialize)]
struct TestAptosCrypto(String);

fn random_bytes(len: usize) -> Vec<u8> {
    thread_rng().sample_iter(&distributions::Alphanumeric)
        .take(len)
        .map(|c|c as u8)
        .collect()
}

fn rand_g1_affine() -> G1Affine {
    let k = Fr::rand(&mut test_rng());
    G1Affine::prime_subgroup_generator().mul(k).into_affine()
}

fn bench_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("ark_bls12_381");

    group.bench_function("fr_add", move |b| {
        b.iter_with_setup(
            || {
                (Fr::rand(&mut test_rng()), Fr::rand(&mut test_rng()))
            },
            |(k_1, k_2)| {
                let _k_3 = k_1 + k_2;
            }
        )
    });

    group.bench_function("fr_deserialize", move |b| {
        b.iter_with_setup(
            || {
                let k = Fr::rand(&mut test_rng());
                let mut buf = vec![];
                k.serialize_uncompressed(&mut buf).unwrap();
                buf
            },
            |buf| {
                let _k = Fr::deserialize_uncompressed(buf.as_slice()).unwrap();
            }
        )
    });

    group.bench_function("fr_div", move |b| {
        b.iter_with_setup(
            || {
                (Fr::rand(&mut test_rng()), Fr::rand(&mut test_rng()))
            },
            |(k_1, k_2)| {
                let _k_3 = k_1 / k_2;
            }
        )
    });

    group.bench_function("fr_eq", move |b| {
        b.iter_with_setup(
            || {
                let k_1 = Fr::rand(&mut test_rng());
                let k_2 = k_1.clone();
                (k_1, k_2)
            },
            |(k_1, k_2)| {
                let _res = k_1 == k_2;
            }
        )
    });

    group.bench_function("fr_from_u128", move |b| {
        b.iter_with_setup(
            || {
                u128::rand(&mut test_rng())
            },
            |val| {
                let _k = Fr::from(val);
            }
        )
    });

    group.bench_function("fr_inv", move |b| {
        b.iter_with_setup(
            || {
                Fr::rand(&mut test_rng())
            },
            |k| {
                let _k_inv = k.inverse();
            }
        )
    });

    group.bench_function("fr_mul", move |b| {
        b.iter_with_setup(
            || {
                (Fr::rand(&mut test_rng()), Fr::rand(&mut test_rng()))
            },
            |(k_1, k_2)| {
                let _k_3 = k_1 * k_2;
            }
        )
    });

    group.bench_function("fr_neg", move |b| {
        b.iter_with_setup(
            || {
                Fr::rand(&mut test_rng())
            },
            |k| {
                let _k_inv = k.neg();
            }
        )
    });

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

    group.bench_function("fr_sub", move |b| {
        b.iter_with_setup(
            || {
                (Fr::rand(&mut test_rng()), Fr::rand(&mut test_rng()))
            },
            |(k_1, k_2)| {
                let _k_3 = k_1 - k_2;
            }
        )
    });

    group.bench_function("fr_to_repr", move |b| {
        b.iter_with_setup(
            || {
                Fr::rand(&mut test_rng())
            },
            |k| {
                let _s = k.into_repr();
            }
        )
    });

    group.bench_function("fq12_clone", move |b| {
        b.iter_with_setup(
            || {
                Fq12::rand(&mut test_rng())
            },
            |e| {
                let _e_another = e.clone();
            }
        )
    });

    group.bench_function("fq12_deserialize", move |b| {
        b.iter_with_setup(
            || {
                let e = Fq12::rand(&mut test_rng());
                let mut buf = vec![];
                e.serialize_uncompressed(&mut buf).unwrap();
                buf
            },
            |buf| {
                let _e = Fq12::deserialize_uncompressed(buf.as_slice()).unwrap();
            }
        )
    });

    group.bench_function("fq12_eq", move |b| {
        b.iter_with_setup(
            || {
                let e_1 = Fq12::rand(&mut test_rng());
                let e_2 = e_1.clone();
                (e_1, e_2)
            },
            |(e_1, e_2)| {
                let _res = e_1 == e_2;
            }
        )
    });

    group.bench_function("fq12_inv", move |b| {
        b.iter_with_setup(
            || {
                Fq12::rand(&mut test_rng())
            },
            |e| {
                let _e_inv = e.inverse();
            }
        )
    });

    group.bench_function("fq12_mul", move |b| {
        b.iter_with_setup(
            || {
                (Fq12::rand(&mut test_rng()), Fq12::rand(&mut test_rng()))
            },
            |(e_1, e_2)| {
                let _e_3 = e_1 * e_2;
            }
        )
    });

    group.bench_function("fq12_one", move |b| {
        b.iter(
            || {
                let _e = Fq12::one();
            }
        )
    });

    group.bench_function("fq12_pow_fr", move |b| {
        b.iter_with_setup(
            || {
                let e = Fq12::rand(&mut test_rng());
                let k = Fr::rand(&mut test_rng()).into_repr();
                (e, k)
            },
            |(e, k)| {
                let _res = e.pow(k);
            }
        )
    });

    group.bench_function("fq12_serialize", move |b| {
        b.iter_with_setup(
            || {
                Fq12::rand(&mut test_rng())
            },
            |e| {
                let mut buf = vec![];
                e.serialize_uncompressed(&mut buf).unwrap();
            }
        )
    });

    group.bench_function("fq12_square", move |b| {
        b.iter_with_setup(
            || {
                Fq12::rand(&mut test_rng())
            },
            |e| {
                let _res = e.square();
            }
        )
    });

    group.bench_function("g1_affine_add", move |b| {
        b.iter_with_setup(
            || {
                (rand_g1_affine(), rand_g1_affine())
            },
            |(p1, p2)| {
                let _p3 = p1 + p2;
            }
        )
    });

    group.bench_function("g1_affine_deserialize_comp", move |b| {
        b.iter_with_setup(
            || {
                let p = rand_g1_affine();
                let mut buf = vec![];
                p.serialize(&mut buf).unwrap();
                buf
            },
            |buf| {
                let _p = G1Affine::deserialize_uncompressed(buf.as_slice());
            }
        )
    });

    group.bench_function("g1_affine_deserialize_uncomp", move |b| {
        b.iter_with_setup(
            || {
                let p = rand_g1_affine();
                let mut buf = vec![];
                p.serialize_uncompressed(&mut buf).unwrap();
                buf
            },
            |buf| {
                let _p = G1Affine::deserialize_uncompressed(buf.as_slice());
            }
        )
    });

    group.bench_function("g1_affine_eq", move |b| {
        b.iter_with_setup(
            || {
                let p1 = rand_g1_affine();
                let p2 = p1.clone();
                (p1, p2)
            },
            |(p1, p2)| {
                let _res = p1 == p2;
            }
        )
    });

    group.bench_function("g1_affine_generator", move |b| {
        b.iter(
            || {
                let _res = G1Affine::prime_subgroup_generator();
            }
        )
    });

    group.bench_function("g1_affine_infinity", move |b| {
        b.iter(
            || {
                let _res = G1Affine::zero();
            }
        )
    });

    group.bench_function("g1_affine_scalar_mul_to_proj", move |b| {
        b.iter_with_setup(
            || {
                (rand_g1_affine(), Fr::rand(&mut test_rng()))
            },
            |(p, k)| {
                let _res = p.mul(k);
            }
        )
    });

    group.bench_function("g1_affine_neg", move |b| {
        b.iter_with_setup(
            || {
                rand_g1_affine()
            },
            |p| {
                let _res = p.neg();
            }
        )
    });

    group.bench_function("g1_affine_ser_comp", move |b| {
        b.iter_with_setup(
            || {
                rand_g1_affine()
            },
            |p_affine| {
                let mut buf = vec![];
                p_affine.serialize(&mut buf).unwrap();
            }
        )
    });

    group.bench_function("g1_affine_ser_uncomp", move |b| {
        b.iter_with_setup(
            || {
                rand_g1_affine()
            },
            |p_affine| {
                let mut buf = vec![];
                p_affine.serialize_uncompressed(&mut buf).unwrap();
            }
        )
    });

    group.bench_function("g1_affine_to_prepared", move |b| {
        b.iter_with_setup(
            || {
                rand_g1_affine()
            },
            |p_affine| {
                let _res = ark_ec::prepare_g1::<ark_bls12_381::Bls12_381>(p_affine);
            }
        )
    });

    group.bench_function("g1_affine_to_proj", move |b| {
        b.iter_with_setup(
            || {
                rand_g1_affine()
            },
            |p_affine| {
                let _res = p_affine.into_projective();
            }
        )
    });

    group.bench_function("g1_proj_add", move |b| {
        b.iter_with_setup(
            || {
                let p = G1Projective::rand(&mut test_rng());
                let q = G1Projective::rand(&mut test_rng());
                (p, q)
            },
            |(p, q)| {
                let _res = p + q;
            }
        )
    });

    group.bench_function("g1_proj_double", move |b| {
        b.iter_with_setup(
            || {
                G1Projective::rand(&mut test_rng())
            },
            |p| {
                let _q = ProjectiveCurve::double(&p);
            }
        )
    });

    group.bench_function("g1_proj_eq", move |b| {
        b.iter_with_setup(
            || {
                let p = G1Projective::rand(&mut test_rng());
                let q = p.clone();
                (p, q)
            },
            |(p, q)| {
                let _res = p == q;
            }
        )
    });

    group.bench_function("g1_proj_generator", move |b| {
        b.iter(
            || {
                let _res = G1Projective::prime_subgroup_generator();
            }
        )
    });

    group.bench_function("g1_proj_infinity", move |b| {
        b.iter(
            || {
                let _res = G1Projective::zero();
            }
        )
    });

    group.bench_function("g1_proj_neg", move |b| {
        b.iter_with_setup(
            || {
                G1Projective::rand(&mut test_rng())
            },
            |p| {
                let _q = p.neg();
            }
        )
    });

    group.bench_function("g1_proj_scalar_mul", move |b| {
        b.iter_with_setup(
            || {
                let p = G1Projective::rand(&mut test_rng());
                let k = Fr::rand(&mut test_rng()).into_repr();
                (p, k)
            },
            |(p, k)| {
                let _q = p.mul(k);
            }
        )
    });

    group.bench_function("g1_proj_sub", move |b| {
        b.iter_with_setup(
            || {
                let p = G1Projective::rand(&mut test_rng());
                let q = G1Projective::rand(&mut test_rng());
                (p, q)
            },
            |(p, q)| {
                let _r = p - q;
            }
        )
    });

    group.bench_function("g1_proj_to_affine", move |b| {
        b.iter_with_setup(
            || {
                G1Projective::rand(&mut test_rng())
            },
            |p_proj| {
                let _ = p_proj.into_affine();
            }
        )
    });

    group.bench_function("g1_proj_to_prepared", move |b| {
        b.iter_with_setup(
            || {
                G1Projective::rand(&mut test_rng())
            },
            |p| {
                let _res = ark_ec::prepare_g1::<ark_bls12_381::Bls12_381>(p);
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
