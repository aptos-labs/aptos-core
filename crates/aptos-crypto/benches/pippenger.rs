// Copyright © Aptos Foundation

// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate criterion;

use std::cmp::min;
use ark_bls12_381::{Fr, G1Projective, G2Projective};
use ark_ff::Zero;
use ark_std::test_rng;
use criterion::{BenchmarkId, Criterion};
use std::ops::{Add, AddAssign, Neg};
use std::time::Duration;
use ark_ec::Group;
use aptos_crypto::{msm_all_bench_cases, rand, serialize};
use aptos_crypto::pippenger::{PippengerFriendlyStructure, generic_pippenger, probably_pippenger_signed_digits, find_best_window_size};
use ark_std::UniformRand;
use ark_serialize::CanonicalSerialize;

fn bits_from_lsb(buf: &[u8], num_bits_needed: usize) -> Vec<bool> {
    let num_bits_offered = 8 * buf.len();
    let p = min(num_bits_needed, num_bits_offered);
    let mut bits = Vec::with_capacity(num_bits_needed);
    for i in 0..p {
        let byte_id = i >> 3;
        let bit_id = i & 7;
        let bit = (buf[byte_id] & (1<<bit_id)) != 0;
        bits.push(bit);
    }
    for _i in p..num_bits_needed {
        bits.push(false);
    }
    bits
}

#[derive(Eq, PartialEq, Debug, Clone)]
struct Bls12381G1ProjWrapper {
    inner: G1Projective,
}

#[derive(Eq, PartialEq, Debug, Clone)]
struct Bls12381G2ProjWrapper {
    inner: G2Projective,
}

impl PippengerFriendlyStructure for Bls12381G1ProjWrapper {
    fn add(&self, other: &Self) -> Self {
        Self {
            inner: self.inner.add(&other.inner)
        }
    }

    fn add_assign(&mut self, other: &Self) {
        self.inner.add_assign(&other.inner);
    }

    fn double(&self) -> Self {
        Self {
            inner: self.inner.double()
        }
    }

    fn double_assign(&mut self) {
        self.inner.double_in_place();
    }

    fn neg(&self) -> Self {
        Self {
            inner: self.inner.neg()
        }
    }

    fn zero() -> Self {
        Self {
            inner: G1Projective::zero()
        }
    }
}

impl PippengerFriendlyStructure for Bls12381G2ProjWrapper {
    fn add(&self, other: &Self) -> Self {
        Self {
            inner: self.inner.add(&other.inner)
        }
    }

    fn add_assign(&mut self, other: &Self) {
        self.inner.add_assign(&other.inner);
    }

    fn double(&self) -> Self {
        Self {
            inner: self.inner.double()
        }
    }

    fn double_assign(&mut self) {
        self.inner.double_in_place();
    }

    fn neg(&self) -> Self {
        Self {
            inner: self.inner.neg()
        }
    }

    fn zero() -> Self {
        Self {
            inner: G2Projective::zero()
        }
    }
}

fn bench_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("pippenger");

    // Debugging configurations begin.
    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_millis(500));
    // Debugging configurations end.

    for num_entries in msm_all_bench_cases() {
        let est_window_size = find_best_window_size(num_entries);
        for window_bitlen in (est_window_size-1)..(est_window_size+3) {
            group.bench_function(BenchmarkId::new(format!("basic_ws{window_bitlen}_arkg1"), num_entries), |b| {
                b.iter_with_setup(
                    || {
                        let elements = (0..num_entries).map(|_i| {
                            Bls12381G1ProjWrapper {
                                inner: rand!(G1Projective)
                            }
                        }).collect::<Vec<_>>();
                        let scalars = (0..num_entries).map(|_i| {
                            let s = rand!(Fr);
                            let buf = serialize!(s, serialize_uncompressed);
                            bits_from_lsb(buf.as_slice(), 255)
                        }).collect::<Vec<_>>();
                        (elements, scalars)
                    },
                    |(elements, scalars)| {
                        let _res = generic_pippenger(elements.as_slice(), scalars.as_slice(), window_bitlen);
                    },
                );
            });
            group.bench_function(BenchmarkId::new(format!("signed_ws{window_bitlen}_arkg1"), num_entries), |b| {
                b.iter_with_setup(
                    || {
                        let elements = (0..num_entries).map(|_i| {
                            Bls12381G1ProjWrapper {
                                inner: rand!(G1Projective)
                            }
                        }).collect::<Vec<_>>();
                        let scalars = (0..num_entries).map(|_i| {
                            let s = rand!(Fr);
                            let buf = serialize!(s, serialize_uncompressed);
                            bits_from_lsb(buf.as_slice(), 255)
                        }).collect::<Vec<_>>();
                        (elements, scalars)
                    },
                    |(elements, scalars)| {
                        let _res = probably_pippenger_signed_digits(elements.as_slice(), scalars.as_slice(), window_bitlen);
                    },
                );
            });
            group.bench_function(BenchmarkId::new(format!("basic_ws{window_bitlen}_arkg2"), num_entries), |b| {
                b.iter_with_setup(
                    || {
                        let elements = (0..num_entries).map(|_i| {
                            Bls12381G2ProjWrapper {
                                inner: rand!(G2Projective)
                            }
                        }).collect::<Vec<_>>();
                        let scalars = (0..num_entries).map(|_i| {
                            let s = rand!(Fr);
                            let buf = serialize!(s, serialize_uncompressed);
                            bits_from_lsb(buf.as_slice(), 255)
                        }).collect::<Vec<_>>();
                        (elements, scalars)
                    },
                    |(elements, scalars)| {
                        let _res = generic_pippenger(elements.as_slice(), scalars.as_slice(), window_bitlen);
                    },
                );
            });
            group.bench_function(BenchmarkId::new(format!("signed_ws{window_bitlen}_arkg2"), num_entries), |b| {
                b.iter_with_setup(
                    || {
                        let elements = (0..num_entries).map(|_i| {
                            Bls12381G2ProjWrapper {
                                inner: rand!(G2Projective)
                            }
                        }).collect::<Vec<_>>();
                        let scalars = (0..num_entries).map(|_i| {
                            let s = rand!(Fr);
                            let buf = serialize!(s, serialize_uncompressed);
                            bits_from_lsb(buf.as_slice(), 255)
                        }).collect::<Vec<_>>();
                        (elements, scalars)
                    },
                    |(elements, scalars)| {
                        let _res = probably_pippenger_signed_digits(elements.as_slice(), scalars.as_slice(), window_bitlen);
                    },
                );
            });
        }
    }

    group.finish();
}

criterion_group!(
    name = pippenger_benches;
    config = Criterion::default();
    targets = bench_group);
criterion_main!(pippenger_benches);
