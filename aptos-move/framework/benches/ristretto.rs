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

fn g1_proj_rand() -> ark_bls12_381::G1Projective {
    let x = ark_bls12_381::Fr::rand(&mut test_rng());
    let g = ark_bls12_381::G1Projective::prime_subgroup_generator();
    let p = g.mul(x.into_repr());
    p
}

pub fn bench_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("ristretto");

    group.bench_function("scalar_rand + point_rand", |b| {
        b.iter(|| {
            let _p = curve25519_dalek::ristretto::RistrettoPoint::random(&mut thread_rng());
            let _s = curve25519_dalek::scalar::Scalar::random(&mut thread_rng());
        });
    });

    group.bench_function("scalar_rand + point_rand + point_mul", |b| {
        b.iter(|| {
            let p = curve25519_dalek::ristretto::RistrettoPoint::random(&mut thread_rng());
            let s = curve25519_dalek::scalar::Scalar::random(&mut thread_rng());
            let _r = p.mul(s);
        });
    });

    group.bench_function("scalar_rand", |b| {
        b.iter(|| {
            let _s = curve25519_dalek::scalar::Scalar::random(&mut thread_rng());
        });
    });

    group.bench_function("scalar_rand + scalar_inverse", |b| {
        b.iter(|| {
            let s = curve25519_dalek::scalar::Scalar::random(&mut thread_rng());
            let _s_inv = s.invert();
        });
    });

    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default();
    targets = bench_group);

criterion_main!(benches);
