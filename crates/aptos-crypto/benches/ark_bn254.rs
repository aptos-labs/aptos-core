// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate criterion;

use crate::bench_utils::{
    bench_function_add, bench_function_clone, bench_function_deser_comp,
    bench_function_deser_uncomp, bench_function_div, bench_function_double, bench_function_eq,
    bench_function_from_u64, bench_function_inv, bench_function_mul, bench_function_neg,
    bench_function_pow_u256, bench_function_serialize_uncomp, bench_function_square,
    bench_function_sub,
};
use ark_bn254::{Bn254, Fq, Fq12, Fr, G1Affine, G1Projective, G2Affine, G2Projective};
use ark_ec::{pairing::Pairing, short_weierstrass::Projective, AffineRepr, CurveGroup};
use ark_ff::{UniformRand, Zero};
use ark_groth16::Groth16;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::test_rng;
use criterion::{Bencher, BenchmarkId, Criterion};
use std::ops::{Mul, Neg};
use ark_ff::AdditiveGroup;
use ark_ec::PrimeGroup;

mod bench_utils;

fn msm_all_bench_cases() -> Vec<usize> {
    let series_until_65 = (1..65).step_by(2);
    let series_until_129 = (64..129).step_by(4);
    let series_until_257 = (129..257).step_by(8);
    series_until_65
        .chain(series_until_129)
        .chain(series_until_257)
        .collect::<Vec<_>>()
}

macro_rules! rand {
    ($typ:ty) => {{
        <$typ>::rand(&mut test_rng())
    }};
}

macro_rules! serialize {
    ($obj:expr, $method:ident) => {{
        let mut buf = vec![];
        $obj.$method(&mut buf).unwrap();
        buf
    }};
}

fn bench_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("ark_bn254");

    group.bench_function("groth16/verify", bench_groth16_verify);

    group.bench_function("fr_add", bench_function_add::<Fr>);
    group.bench_function("fr_clone", bench_function_clone::<Fr>);
    group.bench_function("fr_deser", bench_function_deser_uncomp::<Fr>);
    group.bench_function("fr_div", bench_function_div::<Fr>);
    group.bench_function("fr_double", bench_function_double::<Fr>);
    group.bench_function("fr_eq", bench_function_eq::<Fr>);
    group.bench_function("fr_from_u64", bench_function_from_u64::<Fr>);
    group.bench_function("fr_inv", bench_function_inv::<Fr>);
    group.bench_function("fr_mul", bench_function_mul::<Fr>);
    group.bench_function("fr_neg", bench_function_neg::<Fr>);
    group.bench_function("fr_pow_u256", bench_function_pow_u256::<Fr>);
    group.bench_function("fr_serialize", bench_function_serialize_uncomp::<Fr>);
    group.bench_function("fr_square", bench_function_square::<Fr>);
    group.bench_function("fr_sub", bench_function_sub::<Fr>);

    group.bench_function("fq_add", bench_function_add::<Fq>);
    group.bench_function("fq_clone", bench_function_clone::<Fq>);
    group.bench_function("fq_deser", bench_function_deser_uncomp::<Fq>);
    group.bench_function("fq_div", bench_function_div::<Fq>);
    group.bench_function("fq_double", bench_function_double::<Fq>);
    group.bench_function("fq_eq", bench_function_eq::<Fq>);
    group.bench_function("fq_from_u64", bench_function_from_u64::<Fq>);
    group.bench_function("fq_inv", bench_function_inv::<Fq>);
    group.bench_function("fq_mul", bench_function_mul::<Fq>);
    group.bench_function("fq_neg", bench_function_neg::<Fq>);
    group.bench_function("fq_pow_u256", bench_function_pow_u256::<Fq>);
    group.bench_function("fq_serialize", bench_function_serialize_uncomp::<Fq>);
    group.bench_function("fq_square", bench_function_square::<Fq>);
    group.bench_function("fq_sub", bench_function_sub::<Fq>);

    group.bench_function("fq12_add", bench_function_add::<Fq12>);
    group.bench_function("fq12_clone", bench_function_clone::<Fq12>);
    group.bench_function("fq12_deser", bench_function_deser_uncomp::<Fq12>);
    group.bench_function("fq12_div", bench_function_div::<Fq12>);
    group.bench_function("fq12_double", bench_function_double::<Fq12>);
    group.bench_function("fq12_eq", bench_function_eq::<Fq12>);
    group.bench_function("fq12_from_u64", bench_function_from_u64::<Fq12>);
    group.bench_function("fq12_inv", bench_function_inv::<Fq12>);
    group.bench_function("fq12_mul", bench_function_mul::<Fq12>);
    group.bench_function("fq12_neg", bench_function_neg::<Fq12>);
    group.bench_function("fq12_pow_u256", bench_function_pow_u256::<Fq12>);
    group.bench_function("fq12_serialize", bench_function_serialize_uncomp::<Fq12>);
    group.bench_function("fq12_square", bench_function_square::<Fq12>);
    group.bench_function("fq12_sub", bench_function_sub::<Fq12>);

    group.bench_function("g1_affine_add", bench_function_add::<G1Affine>);
    group.bench_function(
        "g1_affine_deser_comp",
        bench_function_deser_comp::<G1Affine>,
    );
    group.bench_function(
        "g1_affine_deser_uncomp",
        bench_function_deser_uncomp::<G1Affine>,
    );
    group.bench_function("g1_affine_eq", bench_function_eq::<G1Affine>);
    group.bench_function("g1_affine_generator", move |b| {
        b.iter(|| {
            let _res = G1Affine::generator();
        })
    });

    group.bench_function("g1_affine_infinity", move |b| {
        b.iter(|| {
            let _res = G1Affine::zero();
        })
    });

    group.bench_function("g1_affine_scalar_mul_to_proj", move |b| {
        b.iter_with_setup(
            || (rand!(G1Affine), rand!(Fr)),
            |(p, k)| {
                let _res = p.mul(k);
            },
        )
    });

    group.bench_function("g1_affine_neg", move |b| {
        b.iter_with_setup(
            || rand!(G1Affine),
            |p| {
                let _res = p.neg();
            },
        )
    });

    group.bench_function("g1_affine_serialize_comp", move |b| {
        b.iter_with_setup(
            || rand!(G1Affine),
            |p_affine| {
                let _buf = serialize!(p_affine, serialize_compressed);
            },
        )
    });

    group.bench_function("g1_affine_serialize_uncomp", move |b| {
        b.iter_with_setup(
            || rand!(G1Affine),
            |p_affine| {
                let _buf = serialize!(p_affine, serialize_uncompressed);
            },
        )
    });

    group.bench_function("g1_affine_to_proj", move |b| {
        b.iter_with_setup(
            || rand!(G1Affine),
            |p_affine| {
                let _res = G1Projective::from(p_affine);
            },
        )
    });

    group.bench_function("g1_proj_add", move |b| {
        b.iter_with_setup(
            || {
                let p = rand!(G1Projective);
                let q = rand!(G1Projective);
                (p, q)
            },
            |(p, q)| {
                let _res = p + q;
            },
        )
    });

    group.bench_function("g1_proj_double", move |b| {
        b.iter_with_setup(
            || rand!(G1Projective),
            |p| {
                let _q = p.double();
            },
        )
    });

    group.bench_function("g1_proj_eq", move |b| {
        b.iter_with_setup(
            || {
                let p = rand!(G1Projective);
                let q = p;
                (p, q)
            },
            |(p, q)| {
                let _res = p == q;
            },
        )
    });

    group.bench_function("g1_proj_generator", move |b| {
        b.iter(|| {
            let _res = G1Projective::generator();
        })
    });

    group.bench_function("g1_proj_infinity", move |b| {
        b.iter(|| {
            let _res = G1Projective::zero();
        })
    });

    group.bench_function("g1_proj_neg", move |b| {
        b.iter_with_setup(
            || rand!(G1Projective),
            |p| {
                let _q = p.neg();
            },
        )
    });

    group.bench_function("g1_proj_scalar_mul", move |b| {
        b.iter_with_setup(
            || {
                let p = rand!(G1Projective);
                let k = rand!(Fr);
                (p, k)
            },
            |(p, k)| {
                let _q = p.mul(k);
            },
        )
    });

    group.bench_function("g1_proj_sub", move |b| {
        b.iter_with_setup(
            || {
                let p = rand!(G1Projective);
                let q = rand!(G1Projective);
                (p, q)
            },
            |(p, q)| {
                let _r = p - q;
            },
        )
    });

    group.bench_function("g1_proj_to_affine", move |b| {
        b.iter_with_setup(
            || rand!(G1Projective),
            |p_proj| {
                let _ = p_proj.into_affine();
            },
        )
    });

    group.bench_function("g2_affine_add", move |b| {
        b.iter_with_setup(
            || (rand!(G2Affine), rand!(G2Affine)),
            |(p1, p2)| {
                let _p3 = p1 + p2;
            },
        )
    });

    group.bench_function("g2_affine_deser_comp", move |b| {
        b.iter_with_setup(
            || {
                let p = rand!(G2Affine);
                serialize!(p, serialize_compressed)
            },
            |buf| {
                let _p = G2Affine::deserialize_compressed(buf.as_slice());
            },
        )
    });

    group.bench_function("g2_affine_deser_uncomp", move |b| {
        b.iter_with_setup(
            || {
                let p = rand!(G2Affine);
                serialize!(p, serialize_uncompressed)
            },
            |buf| {
                let _p = G2Affine::deserialize_uncompressed(buf.as_slice());
            },
        )
    });

    group.bench_function("g2_affine_eq", move |b| {
        b.iter_with_setup(
            || {
                let p1 = rand!(G2Affine);
                let p2 = p1;
                (p1, p2)
            },
            |(p1, p2)| {
                let _res = p1 == Projective::from(p2);
            },
        )
    });

    group.bench_function("g2_affine_generator", move |b| {
        b.iter(|| {
            let _res = G2Affine::generator();
        })
    });

    group.bench_function("g2_affine_infinity", move |b| {
        b.iter(|| {
            let _res = G2Affine::zero();
        })
    });

    group.bench_function("g2_affine_scalar_mul_to_proj", move |b| {
        b.iter_with_setup(
            || (rand!(G2Affine), rand!(Fr)),
            |(p, k)| {
                let _res = p.mul(k);
            },
        )
    });

    group.bench_function("g2_affine_neg", move |b| {
        b.iter_with_setup(
            || rand!(G2Affine),
            |p| {
                let _res = p.neg();
            },
        )
    });

    group.bench_function("g2_affine_serialize_comp", move |b| {
        b.iter_with_setup(
            || rand!(G2Affine),
            |p_affine| {
                let _buf = serialize!(p_affine, serialize_compressed);
            },
        )
    });

    group.bench_function("g2_affine_serialize_uncomp", move |b| {
        b.iter_with_setup(
            || rand!(G2Affine),
            |p_affine| {
                let _buf = serialize!(p_affine, serialize_uncompressed);
            },
        )
    });

    group.bench_function("g2_affine_to_proj", move |b| {
        b.iter_with_setup(
            || rand!(G2Affine),
            |p_affine| {
                let _res = G2Projective::from(p_affine);
            },
        )
    });

    group.bench_function("g2_proj_add", move |b| {
        b.iter_with_setup(
            || {
                let p = rand!(G2Projective);
                let q = rand!(G2Projective);
                (p, q)
            },
            |(p, q)| {
                let _res = p + q;
            },
        )
    });

    group.bench_function("g2_proj_double", move |b| {
        b.iter_with_setup(
            || rand!(G2Projective),
            |p| {
                let _q = p.double();
            },
        )
    });

    group.bench_function("g2_proj_eq", move |b| {
        b.iter_with_setup(
            || {
                let p = rand!(G2Projective);
                let q = p;
                (p, q)
            },
            |(p, q)| {
                let _res = p == q;
            },
        )
    });

    group.bench_function("g2_proj_generator", move |b| {
        b.iter(|| {
            let _res = G2Projective::generator();
        })
    });

    group.bench_function("g2_proj_infinity", move |b| {
        b.iter(|| {
            let _res = G2Projective::zero();
        })
    });

    group.bench_function("g2_proj_neg", move |b| {
        b.iter_with_setup(
            || rand!(G2Projective),
            |p| {
                let _q = p.neg();
            },
        )
    });

    group.bench_function("g2_proj_scalar_mul", move |b| {
        b.iter_with_setup(
            || {
                let p = rand!(G2Projective);
                let k = rand!(Fr);
                (p, k)
            },
            |(p, k)| {
                let _q = p.mul(k);
            },
        )
    });

    group.bench_function("g2_proj_sub", move |b| {
        b.iter_with_setup(
            || {
                let p = rand!(G2Projective);
                let q = rand!(G2Projective);
                (p, q)
            },
            |(p, q)| {
                let _r = p - q;
            },
        )
    });

    group.bench_function("g2_proj_to_affine", move |b| {
        b.iter_with_setup(
            || rand!(G2Projective),
            |p_proj| {
                let _ = p_proj.into_affine();
            },
        )
    });

    group.bench_function("pairing", move |b| {
        b.iter_with_setup(
            || (rand!(G1Affine), rand!(G2Affine)),
            |(g1e, g2e)| {
                let _res = ark_bn254::Bn254::pairing(g1e, g2e).0;
            },
        )
    });

    let linear_regression_max_num_datapoints = 20;

    let pairing_product_max_num_pairs = 100;
    for num_pairs in (0..pairing_product_max_num_pairs)
        .step_by(pairing_product_max_num_pairs / linear_regression_max_num_datapoints)
    {
        group.bench_function(BenchmarkId::new("pairing_product", num_pairs), |b| {
            b.iter_with_setup(
                || {
                    let g1_elements = (0..num_pairs).map(|_i| rand!(G1Affine)).collect::<Vec<_>>();
                    let g2_elements = (0..num_pairs).map(|_i| rand!(G2Affine)).collect::<Vec<_>>();
                    (g1_elements, g2_elements)
                },
                |(g1_elements, g2_elements)| {
                    let _product = ark_bn254::Bn254::multi_pairing(g1_elements, g2_elements).0;
                },
            );
        });
    }

    for num_entries in msm_all_bench_cases() {
        group.bench_function(BenchmarkId::new("g1_affine_msm", num_entries), |b| {
            b.iter_with_setup(
                || {
                    let elements = (0..num_entries)
                        .map(|_i| rand!(G1Affine))
                        .collect::<Vec<_>>();
                    let scalars = (0..num_entries).map(|_i| rand!(Fr)).collect::<Vec<_>>();
                    (elements, scalars)
                },
                |(elements, scalars)| {
                    let _res: G1Projective =
                        ark_ec::VariableBaseMSM::msm(elements.as_slice(), scalars.as_slice())
                            .unwrap();
                },
            );
        });
    }

    for num_entries in msm_all_bench_cases() {
        group.bench_function(BenchmarkId::new("g2_affine_msm", num_entries), |b| {
            b.iter_with_setup(
                || {
                    let elements = (0..num_entries)
                        .map(|_i| rand!(G2Affine))
                        .collect::<Vec<_>>();
                    let scalars = (0..num_entries).map(|_i| rand!(Fr)).collect::<Vec<_>>();
                    (elements, scalars)
                },
                |(elements, scalars)| {
                    let _res: G2Projective =
                        ark_ec::VariableBaseMSM::msm(elements.as_slice(), scalars.as_slice())
                            .unwrap();
                },
            );
        });
    }

    group.finish();
}

fn bench_groth16_verify(b: &mut Bencher) {
    let pvk = ark_groth16::PreparedVerifyingKey {
        vk: ark_groth16::VerifyingKey {
            alpha_g1: rand!(G1Affine),
            beta_g2: rand!(G2Affine),
            gamma_g2: rand!(G2Affine),
            delta_g2: rand!(G2Affine),
            gamma_abc_g1: vec![rand!(G1Affine), rand!(G1Affine)],
        },
        alpha_g1_beta_g2: rand!(ark_bn254::Fq12),
        gamma_g2_neg_pc: rand!(G2Affine).into(),
        delta_g2_neg_pc: rand!(G2Affine).into(),
    };

    b.iter_with_setup(
        || ark_groth16::Proof {
            a: rand!(G1Affine),
            b: rand!(G2Affine),
            c: rand!(G1Affine),
        },
        |proof| {
            let result = Groth16::<Bn254>::verify_proof(&pvk, &proof, &[rand!(Fr)]);
            assert!(matches!(result, Ok(false)))
        },
    )
}

criterion_group!(
    name = ark_bn254_benches;
    config = Criterion::default(); //.measurement_time(Duration::from_secs(100));
    targets = bench_group);
criterion_main!(ark_bn254_benches);
