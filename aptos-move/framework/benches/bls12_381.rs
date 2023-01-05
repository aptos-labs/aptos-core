// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use ark_bls12_381::Parameters;
use ark_ec::bls12::{G1Prepared, G2Prepared};
use ark_ec::{AffineCurve, PairingEngine, ProjectiveCurve};
use ark_ff::{Field, PrimeField, UniformRand};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::test_rng;
use criterion::measurement::Measurement;
use criterion::{criterion_group, criterion_main, BenchmarkGroup, Criterion};
use num_traits::Zero;
use rand::thread_rng;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, SubAssign};
use std::time::Duration;

/*
All variables:
    u64_rand
    fr_add
    fr_addassign
    fr_deserialize
    fr_div
    fr_divassign
    fr_eq_false
    fr_eq_true
    fr_from_u64
    fr_inv
    fr_invassign
    fr_mul
    fr_mulassign
    fr_neg
    fr_rand
    fr_serialize
    fr_sub
    fr_subassign
    g1_affine_add
    g1_affine_addassign
    g1_affine_deserialize_compressed
    g1_affine_deserialize_uncompressed
    g1_affine_eq_proj_true
    g1_affine_infinity
    g1_affine_generator
    g1_affine_generator_mul_to_proj
    g1_affine_mul_to_proj
    g1_affine_neg
    g1_affine_serialize_compressed
    g1_affine_serialize_uncompressed
    g1_affine_to_prepared
    g1_proj_add
    g1_proj_addassign
    g1_proj_eq_proj_true
    g1_proj_eq_proj_false
    g1_proj_generator
    g1_proj_generator_mul
    g1_proj_infinity
    g1_proj_mul
    g1_proj_mulassign
    g1_proj_neg
    g1_proj_sub
    g1_proj_subassign
    g1_proj_to_affine
    g1_proj_to_prepared
    gen_1_input_pairs
    gen_2_input_pairs
    gen_3_input_pairs
    pairing_product_of_1
    pairing_product_of_2
    pairing_product_of_3
 */

fn g1_proj_rand() -> ark_bls12_381::G1Projective {
    let x = ark_bls12_381::Fr::rand(&mut test_rng());
    let g = ark_bls12_381::G1Projective::prime_subgroup_generator();
    let p = g.mul(x.into_repr());
    p
}

pub fn bench_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("bls12_381");

    group.bench_function(format!("u64_rand").as_str(), |b| {
        b.iter(|| {
            let _s = rand::random::<u64>();
        });
    });

    group.bench_function(format!("u64_rand + fr_from_u64").as_str(), |b| {
        b.iter(|| {
            let s = rand::random::<u64>();
            let _x = ark_bls12_381::Fr::from(s);
        });
    });

    group.bench_function(format!("fr_rand").as_str(), |b| {
        b.iter(|| {
            let _x = ark_bls12_381::Fr::rand(&mut test_rng());
        });
    });

    group.bench_function(format!("fr_rand + fr_serialize").as_str(), |b| {
        b.iter(|| {
            let x = ark_bls12_381::Fr::rand(&mut test_rng());
            let mut buf = vec![];
            x.serialize(&mut buf);
        });
    });

    group.bench_function(
        format!("fr_rand + fr_serialize + fr_deserialize").as_str(),
        |b| {
            b.iter(|| {
                let x = ark_bls12_381::Fr::rand(&mut test_rng());
                let mut buf = vec![];
                x.serialize(&mut buf);
                let _x_another =
                    ark_bls12_381::Fr::deserialize_uncompressed(buf.as_slice()).unwrap();
            });
        },
    );

    group.bench_function(format!("fr_rand + fr_neg").as_str(), |b| {
        b.iter(|| {
            let x = ark_bls12_381::Fr::rand(&mut test_rng());
            let _x_neg = x.neg();
        });
    });

    group.bench_function(format!("fr_rand * 2 + fr_add").as_str(), |b| {
        b.iter(|| {
            let x = ark_bls12_381::Fr::rand(&mut test_rng());
            let y = ark_bls12_381::Fr::rand(&mut test_rng());
            let _z = x.add(y);
        });
    });

    group.bench_function(format!("fr_rand * 2 + fr_addassign").as_str(), |b| {
        b.iter(|| {
            let mut x = ark_bls12_381::Fr::rand(&mut test_rng());
            let y = ark_bls12_381::Fr::rand(&mut test_rng());
            x.add_assign(&y);
        });
    });

    group.bench_function(format!("fr_rand * 2 + fr_sub").as_str(), |b| {
        b.iter(|| {
            let x = ark_bls12_381::Fr::rand(&mut test_rng());
            let y = ark_bls12_381::Fr::rand(&mut test_rng());
            let _z = x - y;
        });
    });

    group.bench_function(format!("fr_rand * 2 + fr_subassign").as_str(), |b| {
        b.iter(|| {
            let mut x = ark_bls12_381::Fr::rand(&mut test_rng());
            let y = ark_bls12_381::Fr::rand(&mut test_rng());
            x.sub_assign(&y);
        });
    });

    group.bench_function(format!("fr_rand * 2 + fr_mul").as_str(), |b| {
        b.iter(|| {
            let x = ark_bls12_381::Fr::rand(&mut test_rng());
            let y = ark_bls12_381::Fr::rand(&mut test_rng());
            let _z = x * y;
        });
    });

    group.bench_function(format!("fr_rand * 2 + fr_mulassign").as_str(), |b| {
        b.iter(|| {
            let mut x = ark_bls12_381::Fr::rand(&mut test_rng());
            let y = ark_bls12_381::Fr::rand(&mut test_rng());
            x.mul_assign(&y);
        });
    });

    group.bench_function(format!("fr_rand * 2 + fr_div").as_str(), |b| {
        b.iter(|| {
            let x = ark_bls12_381::Fr::rand(&mut test_rng());
            let y = ark_bls12_381::Fr::rand(&mut test_rng());
            let _z = x / y;
        });
    });

    group.bench_function(format!("fr_rand * 2 + fr_divassign").as_str(), |b| {
        b.iter(|| {
            let mut x = ark_bls12_381::Fr::rand(&mut test_rng());
            let y = ark_bls12_381::Fr::rand(&mut test_rng());
            x.div_assign(&y);
        });
    });

    group.bench_function(format!("fr_rand + fr_inv").as_str(), |b| {
        b.iter(|| {
            let x = ark_bls12_381::Fr::rand(&mut test_rng());
            let _x_inv = x.inverse();
        });
    });

    group.bench_function(format!("fr_rand + fr_invassign").as_str(), |b| {
        b.iter(|| {
            let mut x = ark_bls12_381::Fr::rand(&mut test_rng());
            x.inverse_in_place();
        });
    });

    group.bench_function(format!("fr_rand * 2 + fr_eq_false").as_str(), |b| {
        b.iter(|| {
            let x = ark_bls12_381::Fr::rand(&mut test_rng());
            let y = ark_bls12_381::Fr::rand(&mut test_rng());
            let _z = x == y;
        });
    });

    group.bench_function(
        format!("fr_rand + fr_serialize + fr_deserialize + fr_eq_true").as_str(),
        |b| {
            b.iter(|| {
                let x = ark_bls12_381::Fr::rand(&mut test_rng());
                let mut buf = vec![];
                x.serialize_uncompressed(&mut buf);
                let x_another =
                    ark_bls12_381::Fr::deserialize_uncompressed(buf.as_slice()).unwrap();
                let _z = x == x_another;
            });
        },
    );

    group.bench_function(format!("g1_affine_generator").as_str(), |b| {
        b.iter(|| {
            let _p = ark_bls12_381::G1Affine::prime_subgroup_generator();
        });
    });

    group.bench_function(format!("g1_proj_generator").as_str(), |b| {
        b.iter(|| {
            let _p = ark_bls12_381::G1Projective::prime_subgroup_generator();
        });
    });

    group.bench_function(format!("g1_affine_infinity").as_str(), |b| {
        b.iter(|| {
            let _p = ark_bls12_381::G1Affine::zero();
        });
    });

    group.bench_function(format!("g1_proj_infinity").as_str(), |b| {
        b.iter(|| {
            let _p = ark_bls12_381::G1Projective::zero();
        });
    });

    group.bench_function(
        format!("fr_rand + g1_affine_generator + g1_affine_generator_mul_to_proj").as_str(),
        |b| {
            b.iter(|| {
                let x = ark_bls12_381::Fr::rand(&mut test_rng());
                let g = ark_bls12_381::G1Affine::prime_subgroup_generator();
                let _p = g.mul(x);
            });
        },
    );

    group.bench_function(
        format!("fr_rand + g1_proj_generator + g1_proj_generator_mul").as_str(),
        |b| {
            b.iter(|| {
                let x = ark_bls12_381::Fr::rand(&mut test_rng());
                let g = ark_bls12_381::G1Projective::prime_subgroup_generator();
                let p = g.mul(x.into_repr());
            });
        },
    );

    group.bench_function(
        format!(
            "fr_rand + g1_affine_generator + g1_affine_generator_mul_to_proj + g1_proj_to_affine"
        )
        .as_str(),
        |b| {
            b.iter(|| {
                let x = ark_bls12_381::Fr::rand(&mut test_rng());
                let g = ark_bls12_381::G1Affine::prime_subgroup_generator();
                let p_proj = g.mul(x);
                let p_affine = p_proj.into_affine();
            });
        },
    );

    group.bench_function(
        format!("fr_rand + g1_proj_generator + g1_proj_generator_mul + g1_proj_to_affine").as_str(),
        |b| {
            b.iter(|| {
                let x = ark_bls12_381::Fr::rand(&mut test_rng());
                let g = ark_bls12_381::G1Projective::prime_subgroup_generator();
                let p_proj = g.mul(x.into_repr());
                let _p_affine = p_proj.into_affine();
            });
        },
    );

    group.bench_function(format!("fr_rand + g1_affine_generator + g1_affine_generator_mul_to_proj + g1_proj_to_affine + g1_affine_serialize_uncompressed").as_str(), |b| {
        b.iter(||{
            let x = ark_bls12_381::Fr::rand(&mut test_rng());
            let g = ark_bls12_381::G1Affine::prime_subgroup_generator();
            let p_proj = g.mul(x);
            let p_affine = p_proj.into_affine();
            let mut buf = vec![]; p_affine.serialize_uncompressed(&mut buf);
        });
    });

    group.bench_function(format!("fr_rand + g1_affine_generator + g1_affine_generator_mul_to_proj + g1_proj_to_affine + g1_affine_serialize_uncompressed + g1_affine_deserialize_uncompressed").as_str(), |b| {
        b.iter(||{
            let x = ark_bls12_381::Fr::rand(&mut test_rng());
            let g = ark_bls12_381::G1Affine::prime_subgroup_generator();
            let p_proj = g.mul(x);
            let p_affine = p_proj.into_affine();
            let mut buf = vec![]; p_affine.serialize_uncompressed(&mut buf);
            let _p_affine_another = ark_bls12_381::G1Affine::deserialize_uncompressed(buf.as_slice()).unwrap();
        });
    });

    group.bench_function(format!("fr_rand + g1_affine_generator + g1_affine_generator_mul_to_proj + g1_proj_to_affine + g1_affine_serialize_compressed").as_str(), |b| {
        b.iter(||{
            let x = ark_bls12_381::Fr::rand(&mut test_rng());
            let g = ark_bls12_381::G1Affine::prime_subgroup_generator();
            let p_proj = g.mul(x);
            let p_affine = p_proj.into_affine();
            let mut buf = vec![]; p_affine.serialize(&mut buf);
        });
    });

    group.bench_function(format!("fr_rand + g1_affine_generator + g1_affine_generator_mul_to_proj + g1_proj_to_affine + g1_affine_serialize_compressed + g1_affine_deserialize_compressed").as_str(), |b| {
        b.iter(||{
            let x = ark_bls12_381::Fr::rand(&mut test_rng());
            let g = ark_bls12_381::G1Affine::prime_subgroup_generator();
            let p_proj = g.mul(x);
            let p_affine = p_proj.into_affine();
            let mut buf = vec![]; p_affine.serialize(&mut buf);
            let _p_affine_another = ark_bls12_381::G1Affine::deserialize(buf.as_slice()).unwrap();
        });
    });

    group.bench_function(
        format!("fr_rand + g1_affine_generator + g1_affine_generator_mul_to_proj + g1_proj_neg")
            .as_str(),
        |b| {
            b.iter(|| {
                let x = ark_bls12_381::Fr::rand(&mut test_rng());
                let g = ark_bls12_381::G1Affine::prime_subgroup_generator();
                let p = g.mul(x);
                let _minus_p = p.neg();
            });
        },
    );

    group.bench_function(format!("fr_rand + g1_affine_generator + g1_affine_generator_mul_to_proj + g1_proj_to_affine + g1_affine_neg").as_str(), |b| {
        b.iter(||{
            let x = ark_bls12_381::Fr::rand(&mut test_rng());
            let g = ark_bls12_381::G1Affine::prime_subgroup_generator();
            let p_proj = g.mul(x);
            let p_affine = p_proj.into_affine();
            let _minus_p = p_affine.neg();
        });
    });

    group.bench_function(format!("fr_rand * 2 + g1_affine_generator + g1_affine_generator_mul_to_proj * 2 + g1_proj_to_affine * 2 + g1_affine_add").as_str(), |b| {
        b.iter(||{
            let x1 = ark_bls12_381::Fr::rand(&mut test_rng());
            let x2 = ark_bls12_381::Fr::rand(&mut test_rng());
            let g = ark_bls12_381::G1Affine::prime_subgroup_generator();
            let p1_proj = g.mul(x1);
            let p2_proj = g.mul(x2);
            let p1_affine = p1_proj.into_affine();
            let p2_affine = p2_proj.into_affine();
            let _p3 = p1_affine + p2_affine;
        });
    });

    group.bench_function(format!("fr_rand * 2 + g1_affine_generator + g1_affine_generator_mul_to_proj * 2 + g1_proj_to_affine * 2 + g1_affine_addassign").as_str(), |b| {
        b.iter(||{
            let x1 = ark_bls12_381::Fr::rand(&mut test_rng());
            let x2 = ark_bls12_381::Fr::rand(&mut test_rng());
            let g = ark_bls12_381::G1Affine::prime_subgroup_generator();
            let p1_proj = g.mul(x1);
            let p2_proj = g.mul(x2);
            let mut p1_affine = p1_proj.into_affine();
            let p2_affine = p2_proj.into_affine();
            p1_affine.add_assign(&p2_affine);
        });
    });

    group.bench_function(
        format!(
            "fr_rand * 2 + g1_affine_generator + g1_affine_generator_mul_to_proj * 2 + g1_proj_add"
        )
        .as_str(),
        |b| {
            b.iter(|| {
                let x1 = ark_bls12_381::Fr::rand(&mut test_rng());
                let x2 = ark_bls12_381::Fr::rand(&mut test_rng());
                let g = ark_bls12_381::G1Affine::prime_subgroup_generator();
                let p1_proj = g.mul(x1);
                let p2_proj = g.mul(x2);
                let _p3 = p1_proj + p2_proj;
            });
        },
    );

    group.bench_function(format!("fr_rand * 2 + g1_affine_generator + g1_affine_generator_mul_to_proj * 2 + g1_proj_addassign").as_str(), |b| {
        b.iter(||{
            let x1 = ark_bls12_381::Fr::rand(&mut test_rng());
            let x2 = ark_bls12_381::Fr::rand(&mut test_rng());
            let g = ark_bls12_381::G1Affine::prime_subgroup_generator();
            let mut p1_proj = g.mul(x1);
            let p2_proj = g.mul(x2);
            p1_proj.add_assign(&p2_proj);
        });
    });

    group.bench_function(
        format!(
            "fr_rand * 2 + g1_affine_generator + g1_affine_generator_mul_to_proj * 2 + g1_proj_sub"
        )
        .as_str(),
        |b| {
            b.iter(|| {
                let x1 = ark_bls12_381::Fr::rand(&mut test_rng());
                let x2 = ark_bls12_381::Fr::rand(&mut test_rng());
                let g = ark_bls12_381::G1Affine::prime_subgroup_generator();
                let p1_proj = g.mul(x1);
                let p2_proj = g.mul(x2);
                let _p3 = p1_proj - p2_proj;
            });
        },
    );

    group.bench_function(format!("fr_rand * 2 + g1_affine_generator + g1_affine_generator_mul_to_proj * 2 + g1_proj_subassign").as_str(), |b| {
        b.iter(||{
            let x1 = ark_bls12_381::Fr::rand(&mut test_rng());
            let x2 = ark_bls12_381::Fr::rand(&mut test_rng());
            let g = ark_bls12_381::G1Affine::prime_subgroup_generator();
            let mut p1_proj = g.mul(x1);
            let p2_proj = g.mul(x2);
            p1_proj.sub_assign(p2_proj);
        });
    });

    group.bench_function(
        format!(
            "fr_rand * 2 + g1_affine_generator + g1_affine_generator_mul_to_proj + g1_proj_mul"
        )
        .as_str(),
        |b| {
            b.iter(|| {
                let x1 = ark_bls12_381::Fr::rand(&mut test_rng());
                let x2 = ark_bls12_381::Fr::rand(&mut test_rng());
                let g = ark_bls12_381::G1Affine::prime_subgroup_generator();
                let p1_proj = g.mul(x1);
                let _p3 = p1_proj.mul(x2.into_repr());
            });
        },
    );

    group.bench_function(format!("fr_rand * 2 + g1_affine_generator + g1_affine_generator_mul_to_proj + g1_proj_mulassign").as_str(), |b| {
        b.iter(||{
            let x1 = ark_bls12_381::Fr::rand(&mut test_rng());
            let x2 = ark_bls12_381::Fr::rand(&mut test_rng());
            let g = ark_bls12_381::G1Affine::prime_subgroup_generator();
            let mut p1_proj = g.mul(x1);
            p1_proj.mul_assign(x2);
        });
    });

    group.bench_function(format!("fr_rand * 2 + g1_affine_generator + g1_affine_generator_mul_to_proj + g1_proj_to_affine + g1_affine_mul_to_proj").as_str(), |b| {
        b.iter(||{
            let x1 = ark_bls12_381::Fr::rand(&mut test_rng());
            let x2 = ark_bls12_381::Fr::rand(&mut test_rng());
            let g = ark_bls12_381::G1Affine::prime_subgroup_generator();
            let p1_proj = g.mul(x1);
            let p1_affine = p1_proj.into_affine();
            let _p3 = p1_affine.mul(x2);
        });
    });

    group.bench_function(format!("fr_rand + g1_affine_generator + g1_affine_generator_mul_to_proj + g1_proj_to_affine + g1_affine_serialize_uncompressed + g1_affine_deserialize_uncompressed + g1_affine_eq_proj_true").as_str(), |b| {
        b.iter(||{
            let x1 = ark_bls12_381::Fr::rand(&mut test_rng());
            let g = ark_bls12_381::G1Affine::prime_subgroup_generator();
            let p1_proj = g.mul(x1);
            let p1_affine = p1_proj.into_affine();
            let mut buf = vec![]; p1_affine.serialize_uncompressed(&mut buf);
            let p1_affine_another = ark_bls12_381::G1Affine::deserialize_uncompressed(buf.as_slice()).unwrap();
            let _res = p1_affine_another == p1_proj;
        });
    });

    group.bench_function(format!("fr_rand * 2 + g1_affine_generator + g1_affine_generator_mul_to_proj * 2 + g1_proj_eq_proj_false").as_str(), |b| {
        b.iter(||{
            let x1 = ark_bls12_381::Fr::rand(&mut test_rng());
            let x2 = ark_bls12_381::Fr::rand(&mut test_rng());
            let g = ark_bls12_381::G1Affine::prime_subgroup_generator();
            let p1_proj = g.mul(x1);
            let p2_proj = g.mul(x2);
            let _res = p1_proj == p2_proj;
        });
    });

    group.bench_function(format!("fr_rand + fr_serialize + fr_deserialize + g1_affine_generator + g1_affine_generator_mul_to_proj * 2 + g1_proj_eq_proj_true").as_str(), |b| {
        b.iter(||{
            let x1 = ark_bls12_381::Fr::rand(&mut test_rng());
            let mut buf = vec![]; x1.serialize_uncompressed(&mut buf);
            let x1_another = ark_bls12_381::Fr::deserialize_uncompressed(buf.as_slice()).unwrap();
            let g = ark_bls12_381::G1Affine::prime_subgroup_generator();
            let p1_proj = g.mul(x1);
            let p1_proj_another = g.mul(x1_another);
            let _res = p1_proj == p1_proj_another;
        });
    });

    group.bench_function(format!("fr_rand + g1_affine_generator + g1_affine_generator_mul_to_proj + g1_proj_to_affine + g1_affine_to_prepared").as_str(), |b| {
        b.iter(||{
            let x = ark_bls12_381::Fr::rand(&mut test_rng());
            let g = ark_bls12_381::G1Affine::prime_subgroup_generator();
            let p1_proj = g.mul(x);
            let p1_affine = p1_proj.into_affine();
            let _p1_prep: G1Prepared<Parameters> = ark_ec::prepare_g1::<ark_bls12_381::Bls12_381>(p1_affine);
        });
    });

    group.bench_function(
        format!(
            "fr_rand + g1_affine_generator + g1_affine_generator_mul_to_proj + g1_proj_to_prepared"
        )
        .as_str(),
        |b| {
            b.iter(|| {
                let x = ark_bls12_381::Fr::rand(&mut test_rng());
                let g = ark_bls12_381::G1Affine::prime_subgroup_generator();
                let p1_proj = g.mul(x);
                let _p1_prep: G1Prepared<Parameters> =
                    ark_ec::prepare_g1::<ark_bls12_381::Bls12_381>(p1_proj);
            });
        },
    );

    for num_pairs in 1..4 {
        group.bench_function(format!("gen_{num_pairs}_input_pairs").as_str(), |b| {
            b.iter(|| {
                let _inputs: Vec<(G1Prepared<Parameters>, G2Prepared<Parameters>)> = (0..num_pairs)
                    .map(|i| {
                        let p1 = ark_bls12_381::G1Affine::prime_subgroup_generator()
                            .mul(ark_bls12_381::Fr::rand(&mut test_rng()))
                            .into_affine();
                        let p1p: G1Prepared<Parameters> =
                            ark_ec::prepare_g1::<ark_bls12_381::Bls12_381>(p1);
                        let p2 = ark_bls12_381::G2Affine::prime_subgroup_generator()
                            .mul(ark_bls12_381::Fr::rand(&mut test_rng()))
                            .into_affine();
                        let p2p: G2Prepared<Parameters> =
                            ark_ec::prepare_g2::<ark_bls12_381::Bls12_381>(p2);
                        (p1p, p2p)
                    })
                    .collect();
            });
        });

        group.bench_function(
            format!("gen_{num_pairs}_input_pairs + pairing_product_of_{num_pairs}").as_str(),
            |b| {
                b.iter(|| {
                    let inputs: Vec<(G1Prepared<Parameters>, G2Prepared<Parameters>)> = (0
                        ..num_pairs)
                        .map(|i| {
                            let p1 = ark_bls12_381::G1Affine::prime_subgroup_generator()
                                .mul(ark_bls12_381::Fr::rand(&mut test_rng()))
                                .into_affine();
                            let p1p: G1Prepared<Parameters> =
                                ark_ec::prepare_g1::<ark_bls12_381::Bls12_381>(p1);
                            let p2 = ark_bls12_381::G2Affine::prime_subgroup_generator()
                                .mul(ark_bls12_381::Fr::rand(&mut test_rng()))
                                .into_affine();
                            let p2p: G2Prepared<Parameters> =
                                ark_ec::prepare_g2::<ark_bls12_381::Bls12_381>(p2);
                            (p1p, p2p)
                        })
                        .collect();
                    let _product = ark_bls12_381::Bls12_381::product_of_pairings(inputs.as_slice());
                });
            },
        );
    }

    // group.bench_function("ristretto_scalar_rand + ristretto_point_rand", |b|{
    //     b.iter(||{
    //         let _p = curve25519_dalek::ristretto::RistrettoPoint::random(&mut thread_rng());
    //         let _s = curve25519_dalek::scalar::Scalar::random(&mut thread_rng());
    //     });
    // });
    //
    // group.bench_function("ristretto_scalar_rand + ristretto_point_rand + ristretto_point_mul", |b|{
    //     b.iter(||{
    //         let p = curve25519_dalek::ristretto::RistrettoPoint::random(&mut thread_rng());
    //         let s = curve25519_dalek::scalar::Scalar::random(&mut thread_rng());
    //         let _r = p.mul(s);
    //     });
    // });
    //
    // group.bench_function("ristretto_scalar_rand", |b|{
    //     b.iter(||{
    //         let _s = curve25519_dalek::scalar::Scalar::random(&mut thread_rng());
    //     });
    // });
    //
    // group.bench_function("ristretto_scalar_rand + ristretto_scalar_inverse", |b|{
    //     b.iter(||{
    //         let s = curve25519_dalek::scalar::Scalar::random(&mut thread_rng());
    //         let _s_inv = s.invert();
    //     });
    // });

    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default();
    targets = bench_group);

criterion_main!(benches);
