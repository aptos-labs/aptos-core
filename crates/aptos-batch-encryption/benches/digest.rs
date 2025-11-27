// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use aptos_batch_encryption::{
    group::*,
    shared::{
        digest::DigestKey,
        ids::{free_roots::UncomputedCoeffs, *},
    },
};
use ark_std::{rand::thread_rng, UniformRand};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::{
    fs::File,
    io::{Read, Write as _},
};

pub fn compute(c: &mut Criterion) {
    let mut group = c.benchmark_group("Digest::compute/FFTDomainId");

    // 8

    let mut rng = thread_rng();
    let setup = DigestKey::new(&mut rng, 8, 1).unwrap();
    let mut ids: FFTDomainIdSet<8, UncomputedCoeffs> = FFTDomainIdSet::with_capacity(8).unwrap();
    for x in 0..8 {
        ids.set(x, Fr::rand(&mut rng));
    }

    group.bench_with_input(BenchmarkId::from_parameter(8), &(setup, ids), |b, input| {
        b.iter(|| input.0.digest(&mut input.1.clone(), 0));
    });

    // 32

    let mut rng = thread_rng();
    let setup = DigestKey::new(&mut rng, 32, 1).unwrap();
    let mut ids: FFTDomainIdSet<32, UncomputedCoeffs> = FFTDomainIdSet::with_capacity(32).unwrap();
    for x in 0..32 {
        ids.set(x, Fr::rand(&mut rng));
    }

    group.bench_with_input(
        BenchmarkId::from_parameter(32),
        &(setup, ids),
        |b, input| {
            b.iter(|| input.0.digest(&mut input.1.clone(), 0));
        },
    );

    // 128

    let mut rng = thread_rng();
    let setup = DigestKey::new(&mut rng, 128, 1).unwrap();
    let mut ids: FFTDomainIdSet<128, UncomputedCoeffs> =
        FFTDomainIdSet::with_capacity(128).unwrap();
    for x in 0..128 {
        ids.set(x, Fr::rand(&mut rng));
    }

    group.bench_with_input(
        BenchmarkId::from_parameter(128),
        &(setup, ids),
        |b, input| {
            b.iter(|| input.0.digest(&mut input.1.clone(), 0));
        },
    );

    // 512

    let mut rng = thread_rng();
    let setup = DigestKey::new(&mut rng, 512, 1).unwrap();
    let mut ids: FFTDomainIdSet<512, UncomputedCoeffs> =
        FFTDomainIdSet::with_capacity(512).unwrap();
    for x in 0..512 {
        ids.set(x, Fr::rand(&mut rng));
    }

    group.bench_with_input(
        BenchmarkId::from_parameter(512),
        &(setup, ids),
        |b, input| {
            b.iter(|| input.0.digest(&mut input.1.clone(), 0));
        },
    );
}

pub fn compute_arbitrary_x(c: &mut Criterion) {
    let mut group = c.benchmark_group("Digest::compute/FreeRootId");

    for batch_size in [8, 32, 128, 512] {
        let mut rng = thread_rng();
        let setup = DigestKey::new(&mut rng, batch_size, 1).unwrap();
        let mut ids = FreeRootIdSet::with_capacity(batch_size).unwrap();

        for _x in 0..batch_size {
            ids.add(&FreeRootId::new(Fr::rand(&mut rng)));
        }

        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            &(setup, ids),
            |b, input| {
                b.iter(|| input.0.digest(&mut input.1.clone(), 0));
            },
        );
    }
}
//
//
//pub fn compute_all_eval_proofs(c: &mut Criterion) {
//    let mut group = c.benchmark_group("Digest::compute_all_eval_proofs");
//
//    for batch_size in [8, 32, 128, 512 ] {
//        let mut rng = thread_rng();
//        let setup = DigestKey::new(&mut rng, batch_size).unwrap();
//        let mut ids = FFTDomainIdSet::with_capacity(batch_size).unwrap();
//
//        // set all possible ids
//        for x in 0..batch_size {
//            ids.set(x, Fr::rand(&mut rng));
//        }
//
//        let d = Digest::compute(&setup, ids);
//        group.bench_with_input(BenchmarkId::from_parameter(batch_size), &d, |b, d| {
//            b.iter(|| d.compute_all_eval_proofs());
//        });
//    }
//}
//
pub fn compute_all_eval_proofs_arbitrary_x(c: &mut Criterion) {
    let mut group = c.benchmark_group("EvalProofs::compute_all/FreeRootId");

    for batch_size in [8, 32, 128, 512] {
        let mut rng = thread_rng();
        let setup = DigestKey::new(&mut rng, batch_size, 1).unwrap();
        let mut ids = FreeRootIdSet::with_capacity(batch_size).unwrap();

        for _x in 0..batch_size {
            ids.add(&FreeRootId::new(Fr::rand(&mut rng)));
        }
        let (_, pfs) = setup.digest(&mut ids, 0).unwrap();

        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            &(pfs, setup),
            |b, input| {
                b.iter(|| input.0.clone().compute_all(&input.1));
            },
        );
    }
}

pub fn setup(c: &mut Criterion) {
    let mut group = c.benchmark_group("DigestKey::new");
    group.significance_level(0.1).sample_size(10);

    let batch_size = 128;

    {
        let num_rounds = 10000_usize;
        let mut rng = thread_rng();

        group.bench_with_input(
            BenchmarkId::from_parameter(num_rounds),
            &num_rounds,
            |b, num_rounds| {
                b.iter(|| {
                    let setup = DigestKey::new(&mut rng, batch_size, *num_rounds).unwrap();
                    let mut file = File::create("setup.bcs").unwrap();
                    file.write_all(&bcs::to_bytes(&setup).unwrap()).unwrap();
                    //panic!();

                });
            },
        );
    }
}

pub fn deserialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("DigestKey::deserialize");
    group.significance_level(0.1).sample_size(10);


    {
        let num_rounds = 10000_usize;

        group.bench_with_input(
            BenchmarkId::from_parameter(num_rounds),
            &num_rounds,
            |b, _num_rounds| {
                b.iter(|| {
                    let mut file = File::open("setup.bcs").unwrap();
                    let mut contents = vec![];
                    file.read_to_end(&mut contents).unwrap();
                    let _setup: DigestKey = bcs::from_bytes(&contents).unwrap();


                });
            },
        );
    }
}

criterion_group!(
    benches,
    compute,
    compute_arbitrary_x,
    compute_all_eval_proofs_arbitrary_x
);
criterion_main!(benches);
