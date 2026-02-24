// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#[macro_use]
extern crate criterion;

use aptos_crypto::test_utils::random_bytes;
use criterion::{measurement::Measurement, BenchmarkGroup, BenchmarkId, Criterion, Throughput};
use curve25519_dalek::{
    constants::RISTRETTO_BASEPOINT_TABLE,
    ristretto::{CompressedRistretto, RistrettoPoint},
    scalar::Scalar,
    traits::{Identity, VartimeMultiscalarMul},
};
use rand::{distributions::Uniform, prelude::ThreadRng, thread_rng, Rng, RngCore};
use std::ops::{Add, Mul, Neg, Sub};

fn random_scalar(rng: &mut ThreadRng) -> Scalar {
    let mut bytes = [0u8; 32];
    rng.fill_bytes(&mut bytes);
    Scalar::from_bytes_mod_order(bytes)
}

fn random_ristretto_point(rng: &mut ThreadRng) -> RistrettoPoint {
    // Generate a random point by multiplying a random scalar with the basepoint
    let scalar = random_scalar(rng);
    RistrettoPoint::mul_base(&scalar)
}

fn benchmark_groups(c: &mut Criterion) {
    let mut group = c.benchmark_group("ristretto255");

    group.sample_size(1000);

    point_mul(&mut group);
    basepoint_mul(&mut group);
    basepoint_double_mul(&mut group);
    point_add(&mut group);
    point_compress(&mut group);
    point_decompress(&mut group);
    point_equals(&mut group);
    point_from_64_uniform_bytes(&mut group);
    point_identity(&mut group);
    point_neg(&mut group);
    point_sub(&mut group);

    scalar_add(&mut group);
    scalar_reduced_from_32_bytes(&mut group);
    scalar_uniform_from_64_bytes(&mut group);
    scalar_from_u128(&mut group);
    scalar_from_u64(&mut group);
    scalar_invert(&mut group);
    scalar_mul(&mut group);
    scalar_neg(&mut group);
    scalar_sub(&mut group);

    //for n in 1..=128 {
    //for n in [256, 512, 1024, 2048, 4096] {
    for n in [2, 8192, 16384, 32768] {
        multi_scalar_mul(&mut group, n);
    }

    group.finish();
}

fn multi_scalar_mul<M: Measurement>(g: &mut BenchmarkGroup<M>, n: usize) {
    let mut rng = thread_rng();

    g.throughput(Throughput::Elements(1));
    g.bench_function(BenchmarkId::new("vartime_multiscalar_mul", n), move |b| {
        b.iter_with_setup(
            || {
                let points = (0..n)
                    .map(|_| random_ristretto_point(&mut rng))
                    .collect::<Vec<RistrettoPoint>>();
                let scalars = (0..n)
                    .map(|_| random_scalar(&mut rng))
                    .collect::<Vec<Scalar>>();

                (points, scalars)
            },
            |(points, scalars): (Vec<RistrettoPoint>, Vec<Scalar>)| {
                RistrettoPoint::vartime_multiscalar_mul(
                    scalars.iter(),
                    points.iter().collect::<Vec<&RistrettoPoint>>(),
                )
            },
        )
    });
}

/// Benchmarks the time for a single scalar multiplication on the Ristretto255 basepoint (with precomputation).
fn basepoint_mul<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut rng: ThreadRng = thread_rng();

    let basepoint = RISTRETTO_BASEPOINT_TABLE;

    g.throughput(Throughput::Elements(1));
    g.bench_function("basepoint_mul", move |b| {
        b.iter_with_setup(|| random_scalar(&mut rng), |a| basepoint.mul(&a))
    });
}

/// Benchmarks the time for a double scalar multiplication where one of the bases is the Ristretto255 basepoint.
fn basepoint_double_mul<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut rng: ThreadRng = thread_rng();

    g.throughput(Throughput::Elements(1));
    g.bench_function("basepoint_double_mul", move |b| {
        b.iter_with_setup(
            || {
                (
                    random_ristretto_point(&mut rng),
                    random_scalar(&mut rng),
                    random_scalar(&mut rng),
                )
            },
            |(a_point, a, b)| {
                RistrettoPoint::vartime_double_scalar_mul_basepoint(&a, &a_point, &b);
            },
        )
    });
}

fn point_add<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut rng: ThreadRng = thread_rng();

    g.throughput(Throughput::Elements(1));
    g.bench_function("point_add", move |b| {
        b.iter_with_setup(
            || {
                (
                    random_ristretto_point(&mut rng),
                    random_ristretto_point(&mut rng),
                )
            },
            |(a_point, b_point): (RistrettoPoint, RistrettoPoint)| a_point.add(&b_point),
        )
    });
}

fn point_compress<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut rng: ThreadRng = thread_rng();

    g.throughput(Throughput::Elements(1));
    g.bench_function("point_compress", move |b| {
        b.iter_with_setup(
            || random_ristretto_point(&mut rng),
            |point| point.compress(),
        )
    });
}

fn point_decompress<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut rng: ThreadRng = thread_rng();

    g.throughput(Throughput::Elements(1));
    g.bench_function("point_decompress", move |b| {
        b.iter_with_setup(
            || random_ristretto_point(&mut rng).compress().to_bytes(),
            |bytes| CompressedRistretto(bytes).decompress(),
        )
    });
}

fn point_equals<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut rng: ThreadRng = thread_rng();

    g.throughput(Throughput::Elements(1));
    g.bench_function("point_equals", move |b| {
        b.iter_with_setup(
            || {
                let a = random_ristretto_point(&mut rng);
                (a, a)
            },
            |(a_point, b_point): (RistrettoPoint, RistrettoPoint)| a_point.eq(&b_point),
        )
    });
}

fn point_from_64_uniform_bytes<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut rng: ThreadRng = thread_rng();

    g.throughput(Throughput::Elements(1));
    g.bench_function("point_from_64_uniform_bytes", move |b| {
        b.iter_with_setup(
            || <[u8; 64]>::try_from(random_bytes(&mut rng, 64)).unwrap(),
            |bytes| RistrettoPoint::from_uniform_bytes(&bytes),
        )
    });
}

fn point_identity<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    g.throughput(Throughput::Elements(1));
    g.bench_function("point_identity", move |b| b.iter(RistrettoPoint::identity));
}

/// Benchmarks the time for a single scalar multiplication on a Ristretto255 point (without precomputation).
fn point_mul<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut rng: ThreadRng = thread_rng();

    g.throughput(Throughput::Elements(1));
    g.bench_function("point_mul", move |b| {
        b.iter_with_setup(
            || (random_ristretto_point(&mut rng), random_scalar(&mut rng)),
            |(g, a): (RistrettoPoint, Scalar)| g.mul(&a),
        )
    });
}

fn point_neg<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut rng: ThreadRng = thread_rng();

    g.throughput(Throughput::Elements(1));
    g.bench_function("point_neg", move |b| {
        b.iter_with_setup(
            || random_ristretto_point(&mut rng),
            |point: RistrettoPoint| point.neg(),
        )
    });
}

fn point_sub<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut rng: ThreadRng = thread_rng();

    g.throughput(Throughput::Elements(1));
    g.bench_function("point_sub", move |b| {
        b.iter_with_setup(
            || {
                (
                    random_ristretto_point(&mut rng),
                    random_ristretto_point(&mut rng),
                )
            },
            |(a_point, b_point): (RistrettoPoint, RistrettoPoint)| a_point.sub(&b_point),
        )
    });
}

fn scalar_add<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut rng: ThreadRng = thread_rng();

    g.throughput(Throughput::Elements(1));
    g.bench_function("scalar_add", move |b| {
        b.iter_with_setup(
            || (random_scalar(&mut rng), random_scalar(&mut rng)),
            // NOTE: Specifically moving 'b' in, just like the native Rust function does in Move
            |(a, b)| a.add(b),
        )
    });
}

fn scalar_reduced_from_32_bytes<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut rng: ThreadRng = thread_rng();

    g.throughput(Throughput::Elements(1));
    g.bench_function("scalar_reduced_from_32_bytes", move |b| {
        b.iter_with_setup(
            || <[u8; 32]>::try_from(random_bytes(&mut rng, 32)).unwrap(),
            Scalar::from_bytes_mod_order,
        )
    });
}

fn scalar_uniform_from_64_bytes<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut rng: ThreadRng = thread_rng();

    g.throughput(Throughput::Elements(1));
    g.bench_function("scalar_uniform_from_64_bytes", move |b| {
        b.iter_with_setup(
            || <[u8; 64]>::try_from(random_bytes(&mut rng, 64)).unwrap(),
            |bytes| Scalar::from_bytes_mod_order_wide(&bytes),
        )
    });
}

fn scalar_from_u128<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut rng: ThreadRng = thread_rng();

    g.throughput(Throughput::Elements(1));
    g.bench_function("scalar_from_u128", move |b| {
        b.iter_with_setup(|| rng.sample(Uniform::new(0u128, u128::MAX)), Scalar::from)
    });
}

fn scalar_from_u64<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut rng: ThreadRng = thread_rng();

    g.throughput(Throughput::Elements(1));
    g.bench_function("scalar_from_u64", move |b| {
        b.iter_with_setup(|| rng.sample(Uniform::new(0u64, u64::MAX)), Scalar::from)
    });
}

fn scalar_invert<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut rng: ThreadRng = thread_rng();

    g.throughput(Throughput::Elements(1));
    g.bench_function("scalar_invert", move |b| {
        b.iter_with_setup(|| random_scalar(&mut rng), |a| a.invert())
    });
}

// scalar_is_canonical benchmark removed - is_canonical() is now private in curve25519-dalek v4

fn scalar_mul<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut rng: ThreadRng = thread_rng();

    g.throughput(Throughput::Elements(1));
    g.bench_function("scalar_mul", move |b| {
        b.iter_with_setup(
            || (random_scalar(&mut rng), random_scalar(&mut rng)),
            // NOTE: Specifically moving 'b' in, just like the native Rust function does in Move
            |(a, b)| a.mul(b),
        )
    });
}

fn scalar_neg<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut rng: ThreadRng = thread_rng();

    g.throughput(Throughput::Elements(1));
    g.bench_function("scalar_neg", move |b| {
        b.iter_with_setup(|| random_scalar(&mut rng), |a| a.neg())
    });
}

fn scalar_sub<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut rng: ThreadRng = thread_rng();

    g.throughput(Throughput::Elements(1));
    g.bench_function("scalar_sub", move |b| {
        b.iter_with_setup(
            || {
                let mut bytes1 = [0u8; 32];
                let mut bytes2 = [0u8; 32];
                rng.fill_bytes(&mut bytes1);
                rng.fill_bytes(&mut bytes2);
                (
                    Scalar::from_bytes_mod_order(bytes1),
                    Scalar::from_bytes_mod_order(bytes2),
                )
            },
            // NOTE: Specifically moving 'b' in, just like the native Rust function does in Move
            |(a, b): (Scalar, Scalar)| a.sub(b),
        )
    });
}

criterion_group!(ristretto255_benches, benchmark_groups);
criterion_main!(ristretto255_benches);
