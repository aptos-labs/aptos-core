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
use num_traits::{One, Zero};
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

    // G1
    {
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
            b.iter(|| {
                let x = ark_bls12_381::Fr::rand(&mut test_rng());
                let g = ark_bls12_381::G1Affine::prime_subgroup_generator();
                let p_proj = g.mul(x);
                let p_affine = p_proj.into_affine();
                let mut buf = vec![];
                p_affine.serialize_uncompressed(&mut buf);
            });
        });

        group.bench_function(format!("fr_rand + g1_affine_generator + g1_affine_generator_mul_to_proj + g1_proj_to_affine + g1_affine_serialize_uncompressed + g1_affine_deserialize_uncompressed").as_str(), |b| {
            b.iter(|| {
                let x = ark_bls12_381::Fr::rand(&mut test_rng());
                let g = ark_bls12_381::G1Affine::prime_subgroup_generator();
                let p_proj = g.mul(x);
                let p_affine = p_proj.into_affine();
                let mut buf = vec![];
                p_affine.serialize_uncompressed(&mut buf);
                let _p_affine_another = ark_bls12_381::G1Affine::deserialize_uncompressed(buf.as_slice()).unwrap();
            });
        });

        group.bench_function(format!("fr_rand + g1_affine_generator + g1_affine_generator_mul_to_proj + g1_proj_to_affine + g1_affine_serialize_compressed").as_str(), |b| {
            b.iter(|| {
                let x = ark_bls12_381::Fr::rand(&mut test_rng());
                let g = ark_bls12_381::G1Affine::prime_subgroup_generator();
                let p_proj = g.mul(x);
                let p_affine = p_proj.into_affine();
                let mut buf = vec![];
                p_affine.serialize(&mut buf);
            });
        });

        group.bench_function(format!("fr_rand + g1_affine_generator + g1_affine_generator_mul_to_proj + g1_proj_to_affine + g1_affine_serialize_compressed + g1_affine_deserialize_compressed").as_str(), |b| {
            b.iter(|| {
                let x = ark_bls12_381::Fr::rand(&mut test_rng());
                let g = ark_bls12_381::G1Affine::prime_subgroup_generator();
                let p_proj = g.mul(x);
                let p_affine = p_proj.into_affine();
                let mut buf = vec![];
                p_affine.serialize(&mut buf);
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
            b.iter(|| {
                let x = ark_bls12_381::Fr::rand(&mut test_rng());
                let g = ark_bls12_381::G1Affine::prime_subgroup_generator();
                let p_proj = g.mul(x);
                let p_affine = p_proj.into_affine();
                let _minus_p = p_affine.neg();
            });
        });

        group.bench_function(format!("fr_rand * 2 + g1_affine_generator + g1_affine_generator_mul_to_proj * 2 + g1_proj_to_affine * 2 + g1_affine_add").as_str(), |b| {
            b.iter(|| {
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
            b.iter(|| {
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
            b.iter(|| {
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
            b.iter(|| {
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
            b.iter(|| {
                let x1 = ark_bls12_381::Fr::rand(&mut test_rng());
                let x2 = ark_bls12_381::Fr::rand(&mut test_rng());
                let g = ark_bls12_381::G1Affine::prime_subgroup_generator();
                let mut p1_proj = g.mul(x1);
                p1_proj.mul_assign(x2);
            });
        });

        group.bench_function(format!("fr_rand * 2 + g1_affine_generator + g1_affine_generator_mul_to_proj + g1_proj_to_affine + g1_affine_mul_to_proj").as_str(), |b| {
            b.iter(|| {
                let x1 = ark_bls12_381::Fr::rand(&mut test_rng());
                let x2 = ark_bls12_381::Fr::rand(&mut test_rng());
                let g = ark_bls12_381::G1Affine::prime_subgroup_generator();
                let p1_proj = g.mul(x1);
                let p1_affine = p1_proj.into_affine();
                let _p3 = p1_affine.mul(x2);
            });
        });

        group.bench_function(format!("fr_rand + g1_affine_generator + g1_affine_generator_mul_to_proj + g1_proj_to_affine + g1_affine_serialize_uncompressed + g1_affine_deserialize_uncompressed + g1_affine_eq_proj_true").as_str(), |b| {
            b.iter(|| {
                let x1 = ark_bls12_381::Fr::rand(&mut test_rng());
                let g = ark_bls12_381::G1Affine::prime_subgroup_generator();
                let p1_proj = g.mul(x1);
                let p1_affine = p1_proj.into_affine();
                let mut buf = vec![];
                p1_affine.serialize_uncompressed(&mut buf);
                let p1_affine_another = ark_bls12_381::G1Affine::deserialize_uncompressed(buf.as_slice()).unwrap();
                let _res = p1_affine_another == p1_proj;
            });
        });

        group.bench_function(format!("fr_rand * 2 + g1_affine_generator + g1_affine_generator_mul_to_proj * 2 + g1_proj_eq_proj_false").as_str(), |b| {
            b.iter(|| {
                let x1 = ark_bls12_381::Fr::rand(&mut test_rng());
                let x2 = ark_bls12_381::Fr::rand(&mut test_rng());
                let g = ark_bls12_381::G1Affine::prime_subgroup_generator();
                let p1_proj = g.mul(x1);
                let p2_proj = g.mul(x2);
                let _res = p1_proj == p2_proj;
            });
        });

        group.bench_function(format!("fr_rand + fr_serialize + fr_deserialize + g1_affine_generator + g1_affine_generator_mul_to_proj * 2 + g1_proj_eq_proj_true").as_str(), |b| {
            b.iter(|| {
                let x1 = ark_bls12_381::Fr::rand(&mut test_rng());
                let mut buf = vec![];
                x1.serialize_uncompressed(&mut buf);
                let x1_another = ark_bls12_381::Fr::deserialize_uncompressed(buf.as_slice()).unwrap();
                let g = ark_bls12_381::G1Affine::prime_subgroup_generator();
                let p1_proj = g.mul(x1);
                let p1_proj_another = g.mul(x1_another);
                let _res = p1_proj == p1_proj_another;
            });
        });

        group.bench_function(format!("fr_rand + g1_affine_generator + g1_affine_generator_mul_to_proj + g1_proj_to_affine + g1_affine_to_prepared").as_str(), |b| {
            b.iter(|| {
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
    }

    // G2
    {
        group.bench_function(format!("g2_affine_generator").as_str(), |b| {
            b.iter(|| {
                let _p = ark_bls12_381::G2Affine::prime_subgroup_generator();
            });
        });

        group.bench_function(format!("g2_proj_generator").as_str(), |b| {
            b.iter(|| {
                let _p = ark_bls12_381::G2Projective::prime_subgroup_generator();
            });
        });

        group.bench_function(format!("g2_affine_infinity").as_str(), |b| {
            b.iter(|| {
                let _p = ark_bls12_381::G2Affine::zero();
            });
        });

        group.bench_function(format!("g2_proj_infinity").as_str(), |b| {
            b.iter(|| {
                let _p = ark_bls12_381::G2Projective::zero();
            });
        });

        group.bench_function(
            format!("fr_rand + g2_affine_generator + g2_affine_generator_mul_to_proj").as_str(),
            |b| {
                b.iter(|| {
                    let x = ark_bls12_381::Fr::rand(&mut test_rng());
                    let g = ark_bls12_381::G2Affine::prime_subgroup_generator();
                    let _p = g.mul(x);
                });
            },
        );

        group.bench_function(
            format!("fr_rand + g2_proj_generator + g2_proj_generator_mul").as_str(),
            |b| {
                b.iter(|| {
                    let x = ark_bls12_381::Fr::rand(&mut test_rng());
                    let g = ark_bls12_381::G2Projective::prime_subgroup_generator();
                    let p = g.mul(x.into_repr());
                });
            },
        );

        group.bench_function(
            format!(
                "fr_rand + g2_affine_generator + g2_affine_generator_mul_to_proj + g2_proj_to_affine"
            )
                .as_str(),
            |b| {
                b.iter(|| {
                    let x = ark_bls12_381::Fr::rand(&mut test_rng());
                    let g = ark_bls12_381::G2Affine::prime_subgroup_generator();
                    let p_proj = g.mul(x);
                    let p_affine = p_proj.into_affine();
                });
            },
        );

        group.bench_function(
            format!("fr_rand + g2_proj_generator + g2_proj_generator_mul + g2_proj_to_affine").as_str(),
            |b| {
                b.iter(|| {
                    let x = ark_bls12_381::Fr::rand(&mut test_rng());
                    let g = ark_bls12_381::G2Projective::prime_subgroup_generator();
                    let p_proj = g.mul(x.into_repr());
                    let _p_affine = p_proj.into_affine();
                });
            },
        );

        group.bench_function(format!("fr_rand + g2_affine_generator + g2_affine_generator_mul_to_proj + g2_proj_to_affine + g2_affine_serialize_uncompressed").as_str(), |b| {
            b.iter(|| {
                let x = ark_bls12_381::Fr::rand(&mut test_rng());
                let g = ark_bls12_381::G2Affine::prime_subgroup_generator();
                let p_proj = g.mul(x);
                let p_affine = p_proj.into_affine();
                let mut buf = vec![];
                p_affine.serialize_uncompressed(&mut buf);
            });
        });

        group.bench_function(format!("fr_rand + g2_affine_generator + g2_affine_generator_mul_to_proj + g2_proj_to_affine + g2_affine_serialize_uncompressed + g2_affine_deserialize_uncompressed").as_str(), |b| {
            b.iter(|| {
                let x = ark_bls12_381::Fr::rand(&mut test_rng());
                let g = ark_bls12_381::G2Affine::prime_subgroup_generator();
                let p_proj = g.mul(x);
                let p_affine = p_proj.into_affine();
                let mut buf = vec![];
                p_affine.serialize_uncompressed(&mut buf);
                let _p_affine_another = ark_bls12_381::G2Affine::deserialize_uncompressed(buf.as_slice()).unwrap();
            });
        });

        group.bench_function(format!("fr_rand + g2_affine_generator + g2_affine_generator_mul_to_proj + g2_proj_to_affine + g2_affine_serialize_compressed").as_str(), |b| {
            b.iter(|| {
                let x = ark_bls12_381::Fr::rand(&mut test_rng());
                let g = ark_bls12_381::G2Affine::prime_subgroup_generator();
                let p_proj = g.mul(x);
                let p_affine = p_proj.into_affine();
                let mut buf = vec![];
                p_affine.serialize(&mut buf);
            });
        });

        group.bench_function(format!("fr_rand + g2_affine_generator + g2_affine_generator_mul_to_proj + g2_proj_to_affine + g2_affine_serialize_compressed + g2_affine_deserialize_compressed").as_str(), |b| {
            b.iter(|| {
                let x = ark_bls12_381::Fr::rand(&mut test_rng());
                let g = ark_bls12_381::G2Affine::prime_subgroup_generator();
                let p_proj = g.mul(x);
                let p_affine = p_proj.into_affine();
                let mut buf = vec![];
                p_affine.serialize(&mut buf);
                let _p_affine_another = ark_bls12_381::G2Affine::deserialize(buf.as_slice()).unwrap();
            });
        });

        group.bench_function(
            format!("fr_rand + g2_affine_generator + g2_affine_generator_mul_to_proj + g2_proj_neg")
                .as_str(),
            |b| {
                b.iter(|| {
                    let x = ark_bls12_381::Fr::rand(&mut test_rng());
                    let g = ark_bls12_381::G2Affine::prime_subgroup_generator();
                    let p = g.mul(x);
                    let _minus_p = p.neg();
                });
            },
        );

        group.bench_function(format!("fr_rand + g2_affine_generator + g2_affine_generator_mul_to_proj + g2_proj_to_affine + g2_affine_neg").as_str(), |b| {
            b.iter(|| {
                let x = ark_bls12_381::Fr::rand(&mut test_rng());
                let g = ark_bls12_381::G2Affine::prime_subgroup_generator();
                let p_proj = g.mul(x);
                let p_affine = p_proj.into_affine();
                let _minus_p = p_affine.neg();
            });
        });

        group.bench_function(format!("fr_rand * 2 + g2_affine_generator + g2_affine_generator_mul_to_proj * 2 + g2_proj_to_affine * 2 + g2_affine_add").as_str(), |b| {
            b.iter(|| {
                let x1 = ark_bls12_381::Fr::rand(&mut test_rng());
                let x2 = ark_bls12_381::Fr::rand(&mut test_rng());
                let g = ark_bls12_381::G2Affine::prime_subgroup_generator();
                let p1_proj = g.mul(x1);
                let p2_proj = g.mul(x2);
                let p1_affine = p1_proj.into_affine();
                let p2_affine = p2_proj.into_affine();
                let _p3 = p1_affine + p2_affine;
            });
        });

        group.bench_function(format!("fr_rand * 2 + g2_affine_generator + g2_affine_generator_mul_to_proj * 2 + g2_proj_to_affine * 2 + g2_affine_addassign").as_str(), |b| {
            b.iter(|| {
                let x1 = ark_bls12_381::Fr::rand(&mut test_rng());
                let x2 = ark_bls12_381::Fr::rand(&mut test_rng());
                let g = ark_bls12_381::G2Affine::prime_subgroup_generator();
                let p1_proj = g.mul(x1);
                let p2_proj = g.mul(x2);
                let mut p1_affine = p1_proj.into_affine();
                let p2_affine = p2_proj.into_affine();
                p1_affine.add_assign(&p2_affine);
            });
        });

        group.bench_function(
            format!(
                "fr_rand * 2 + g2_affine_generator + g2_affine_generator_mul_to_proj * 2 + g2_proj_add"
            )
                .as_str(),
            |b| {
                b.iter(|| {
                    let x1 = ark_bls12_381::Fr::rand(&mut test_rng());
                    let x2 = ark_bls12_381::Fr::rand(&mut test_rng());
                    let g = ark_bls12_381::G2Affine::prime_subgroup_generator();
                    let p1_proj = g.mul(x1);
                    let p2_proj = g.mul(x2);
                    let _p3 = p1_proj + p2_proj;
                });
            },
        );

        group.bench_function(format!("fr_rand * 2 + g2_affine_generator + g2_affine_generator_mul_to_proj * 2 + g2_proj_addassign").as_str(), |b| {
            b.iter(|| {
                let x1 = ark_bls12_381::Fr::rand(&mut test_rng());
                let x2 = ark_bls12_381::Fr::rand(&mut test_rng());
                let g = ark_bls12_381::G2Affine::prime_subgroup_generator();
                let mut p1_proj = g.mul(x1);
                let p2_proj = g.mul(x2);
                p1_proj.add_assign(&p2_proj);
            });
        });

        group.bench_function(
            format!(
                "fr_rand * 2 + g2_affine_generator + g2_affine_generator_mul_to_proj * 2 + g2_proj_sub"
            )
                .as_str(),
            |b| {
                b.iter(|| {
                    let x1 = ark_bls12_381::Fr::rand(&mut test_rng());
                    let x2 = ark_bls12_381::Fr::rand(&mut test_rng());
                    let g = ark_bls12_381::G2Affine::prime_subgroup_generator();
                    let p1_proj = g.mul(x1);
                    let p2_proj = g.mul(x2);
                    let _p3 = p1_proj - p2_proj;
                });
            },
        );

        group.bench_function(format!("fr_rand * 2 + g2_affine_generator + g2_affine_generator_mul_to_proj * 2 + g2_proj_subassign").as_str(), |b| {
            b.iter(|| {
                let x1 = ark_bls12_381::Fr::rand(&mut test_rng());
                let x2 = ark_bls12_381::Fr::rand(&mut test_rng());
                let g = ark_bls12_381::G2Affine::prime_subgroup_generator();
                let mut p1_proj = g.mul(x1);
                let p2_proj = g.mul(x2);
                p1_proj.sub_assign(p2_proj);
            });
        });

        group.bench_function(
            format!(
                "fr_rand * 2 + g2_affine_generator + g2_affine_generator_mul_to_proj + g2_proj_mul"
            )
                .as_str(),
            |b| {
                b.iter(|| {
                    let x1 = ark_bls12_381::Fr::rand(&mut test_rng());
                    let x2 = ark_bls12_381::Fr::rand(&mut test_rng());
                    let g = ark_bls12_381::G2Affine::prime_subgroup_generator();
                    let p1_proj = g.mul(x1);
                    let _p3 = p1_proj.mul(x2.into_repr());
                });
            },
        );

        group.bench_function(format!("fr_rand * 2 + g2_affine_generator + g2_affine_generator_mul_to_proj + g2_proj_mulassign").as_str(), |b| {
            b.iter(|| {
                let x1 = ark_bls12_381::Fr::rand(&mut test_rng());
                let x2 = ark_bls12_381::Fr::rand(&mut test_rng());
                let g = ark_bls12_381::G2Affine::prime_subgroup_generator();
                let mut p1_proj = g.mul(x1);
                p1_proj.mul_assign(x2);
            });
        });

        group.bench_function(format!("fr_rand * 2 + g2_affine_generator + g2_affine_generator_mul_to_proj + g2_proj_to_affine + g2_affine_mul_to_proj").as_str(), |b| {
            b.iter(|| {
                let x1 = ark_bls12_381::Fr::rand(&mut test_rng());
                let x2 = ark_bls12_381::Fr::rand(&mut test_rng());
                let g = ark_bls12_381::G2Affine::prime_subgroup_generator();
                let p1_proj = g.mul(x1);
                let p1_affine = p1_proj.into_affine();
                let _p3 = p1_affine.mul(x2);
            });
        });

        group.bench_function(format!("fr_rand + g2_affine_generator + g2_affine_generator_mul_to_proj + g2_proj_to_affine + g2_affine_serialize_uncompressed + g2_affine_deserialize_uncompressed + g2_affine_eq_proj_true").as_str(), |b| {
            b.iter(|| {
                let x1 = ark_bls12_381::Fr::rand(&mut test_rng());
                let g = ark_bls12_381::G2Affine::prime_subgroup_generator();
                let p1_proj = g.mul(x1);
                let p1_affine = p1_proj.into_affine();
                let mut buf = vec![];
                p1_affine.serialize_uncompressed(&mut buf);
                let p1_affine_another = ark_bls12_381::G2Affine::deserialize_uncompressed(buf.as_slice()).unwrap();
                let _res = p1_affine_another == p1_proj;
            });
        });

        group.bench_function(format!("fr_rand * 2 + g2_affine_generator + g2_affine_generator_mul_to_proj * 2 + g2_proj_eq_proj_false").as_str(), |b| {
            b.iter(|| {
                let x1 = ark_bls12_381::Fr::rand(&mut test_rng());
                let x2 = ark_bls12_381::Fr::rand(&mut test_rng());
                let g = ark_bls12_381::G2Affine::prime_subgroup_generator();
                let p1_proj = g.mul(x1);
                let p2_proj = g.mul(x2);
                let _res = p1_proj == p2_proj;
            });
        });

        group.bench_function(format!("fr_rand + fr_serialize + fr_deserialize + g2_affine_generator + g2_affine_generator_mul_to_proj * 2 + g2_proj_eq_proj_true").as_str(), |b| {
            b.iter(|| {
                let x1 = ark_bls12_381::Fr::rand(&mut test_rng());
                let mut buf = vec![];
                x1.serialize_uncompressed(&mut buf);
                let x1_another = ark_bls12_381::Fr::deserialize_uncompressed(buf.as_slice()).unwrap();
                let g = ark_bls12_381::G2Affine::prime_subgroup_generator();
                let p1_proj = g.mul(x1);
                let p1_proj_another = g.mul(x1_another);
                let _res = p1_proj == p1_proj_another;
            });
        });

        group.bench_function(format!("fr_rand + g2_affine_generator + g2_affine_generator_mul_to_proj + g2_proj_to_affine + g2_affine_to_prepared").as_str(), |b| {
            b.iter(|| {
                let x = ark_bls12_381::Fr::rand(&mut test_rng());
                let g = ark_bls12_381::G2Affine::prime_subgroup_generator();
                let p1_proj = g.mul(x);
                let p1_affine = p1_proj.into_affine();
                let _p1_prep: G2Prepared<Parameters> = ark_ec::prepare_g2::<ark_bls12_381::Bls12_381>(p1_affine);
            });
        });

        group.bench_function(
            format!(
                "fr_rand + g2_affine_generator + g2_affine_generator_mul_to_proj + g2_proj_to_prepared"
            )
                .as_str(),
            |b| {
                b.iter(|| {
                    let x = ark_bls12_381::Fr::rand(&mut test_rng());
                    let g = ark_bls12_381::G2Affine::prime_subgroup_generator();
                    let p1_proj = g.mul(x);
                    let _p1_prep: G2Prepared<Parameters> =
                        ark_ec::prepare_g2::<ark_bls12_381::Bls12_381>(p1_proj);
                });
            },
        );
    }

    // Gt
    {
        let generator_buf = hex::decode("b68917caaa0543a808c53908f694d1b6e7b38de90ce9d83d505ca1ef1b442d2727d7d06831d8b2a7920afc71d8eb50120f17a0ea982a88591d9f43503e94a8f1abaf2e4589f65aafb7923c484540a868883432a5c60e75860b11e5465b1c9a08873ec29e844c1c888cb396933057ffdd541b03a5220eda16b2b3a6728ea678034ce39c6839f20397202d7c5c44bb68134f93193cec215031b17399577a1de5ff1f5b0666bdd8907c61a7651e4e79e0372951505a07fa73c25788db6eb8023519a5aa97b51f1cad1d43d8aabbff4dc319c79a58cafc035218747c2f75daf8f2fb7c00c44da85b129113173d4722f5b201b6b4454062e9ea8ba78c5ca3cadaf7238b47bace5ce561804ae16b8f4b63da4645b8457a93793cbd64a7254f150781019de87ee42682940f3e70a88683d512bb2c3fb7b2434da5dedbb2d0b3fb8487c84da0d5c315bdd69c46fb05d23763f2191aabd5d5c2e12a10b8f002ff681bfd1b2ee0bf619d80d2a795eb22f2aa7b85d5ffb671a70c94809f0dafc5b73ea2fb0657bae23373b4931bc9fa321e8848ef78894e987bff150d7d671aee30b3931ac8c50e0b3b0868effc38bf48cd24b4b811a2995ac2a09122bed9fd9fa0c510a87b10290836ad06c8203397b56a78e9a0c61c77e56ccb4f1bc3d3fcaea7550f3503efe30f2d24f00891cb45620605fcfaa4292687b3a7db7c1c0554a93579e889a121fd8f72649b2402996a084d2381c5043166673b3849e4fd1e7ee4af24aa8ed443f56dfd6b68ffde4435a92cd7a4ac3bc77e1ad0cb728606cf08bf6386e5410f").unwrap();
        let r_buf = hex::decode("01000000fffffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed73").unwrap();
        let r = ark_ff::BigInteger256::deserialize_uncompressed(r_buf.as_slice()).unwrap();

        group.bench_function(format!("fq12_rand").as_str(), |b| {
            b.iter(|| {
                let e = ark_bls12_381::Fq12::rand(&mut test_rng());

            });
        });

        group.bench_function(format!("fq12_rand + fq12_serialize").as_str(), |b| {
            b.iter(|| {
                let e = ark_bls12_381::Fq12::rand(&mut test_rng());
                let mut buf = vec![]; e.serialize_uncompressed(&mut buf);
            });
        });

        group.bench_function(format!("fq12_rand + fq12_serialize + fq12_deserialize").as_str(), |b| {
            b.iter(|| {
                let e = ark_bls12_381::Fq12::rand(&mut test_rng());
                let mut buf = vec![]; e.serialize_uncompressed(&mut buf);
                let _e_another = ark_bls12_381::Fq12::deserialize_uncompressed(buf.as_slice()).unwrap();
            });
        });

        group.bench_function(format!("fq12_deserialize_gtgen").as_str(), |b| {
            b.iter(|| {
                let _e_another = ark_bls12_381::Fq12::deserialize_uncompressed(generator_buf.as_slice()).unwrap();
            });
        });

        group.bench_function(format!("fq12_rand + fr_rand + fq12_raised_by_fr").as_str(), |b| {
            b.iter(|| {
                let b = ark_bls12_381::Fq12::rand(&mut test_rng());
                let e = ark_bls12_381::Fr::rand(&mut test_rng());
                b.pow(e.into_repr());
            });
        });

        group.bench_function(format!("fq12_rand + fq12_inverse").as_str(), |b| {
            b.iter(|| {
                let b = ark_bls12_381::Fq12::rand(&mut test_rng());
                let b_inv = b.inverse().unwrap();
            });
        });

        group.bench_function(format!("fq12_rand * 2 + fq12_mul").as_str(), |b| {
            b.iter(|| {
                let b1 = ark_bls12_381::Fq12::rand(&mut test_rng());
                let b2 = ark_bls12_381::Fq12::rand(&mut test_rng());
                let _product = b1 * b2;
            });
        });

        group.bench_function(format!("fq12_rand + fq12_serialize + fq12_deserialize + fq12_eq").as_str(), |b| {
            b.iter(|| {
                let b1 = ark_bls12_381::Fq12::rand(&mut test_rng());
                let mut buf = vec![]; b1.serialize_uncompressed(&mut buf);
                let b1_another = ark_bls12_381::Fq12::deserialize_uncompressed(buf.as_slice()).unwrap();
                let _cmp = b1 == b1_another;
            });
        });
    }

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
