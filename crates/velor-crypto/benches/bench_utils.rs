// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use ark_ff::{BigInteger256, Field};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::{test_rng, UniformRand};
use criterion::Bencher;
use std::ops::{Add, Div, Mul, Neg, Sub};

fn rand<T: UniformRand>() -> T {
    T::rand(&mut test_rng())
}

pub fn bench_function_add<T: Add + UniformRand>(b: &mut Bencher) {
    b.iter_with_setup(
        || (rand::<T>(), rand::<T>()),
        |(e_1, e_2)| {
            let _e_3 = e_1 + e_2;
        },
    )
}

pub fn bench_function_clone<T: Clone + UniformRand>(b: &mut Bencher) {
    b.iter_with_setup(
        || rand::<T>(),
        |e| {
            let _e_2 = e.clone();
        },
    )
}

pub fn bench_function_deser_comp<T: CanonicalSerialize + CanonicalDeserialize + UniformRand>(
    b: &mut Bencher,
) {
    b.iter_with_setup(
        || {
            let e = rand::<T>();
            let mut buf = vec![];
            e.serialize_compressed(&mut buf).unwrap();
            buf
        },
        |buf| {
            let _e = T::deserialize_compressed(buf.as_slice()).unwrap();
        },
    )
}

pub fn bench_function_deser_uncomp<T: CanonicalSerialize + CanonicalDeserialize + UniformRand>(
    b: &mut Bencher,
) {
    b.iter_with_setup(
        || {
            let e = rand::<T>();
            let mut buf = vec![];
            e.serialize_uncompressed(&mut buf).unwrap();
            buf
        },
        |buf| {
            let _e = T::deserialize_uncompressed(buf.as_slice()).unwrap();
        },
    )
}

pub fn bench_function_div<T: Div + UniformRand>(b: &mut Bencher) {
    b.iter_with_setup(
        || (rand::<T>(), rand::<T>()),
        |(e, f)| {
            let _g = e.div(f);
        },
    )
}

pub fn bench_function_double<T: Field + UniformRand>(b: &mut Bencher) {
    b.iter_with_setup(
        || rand::<T>(),
        |e| {
            let _e_2 = e.double();
        },
    )
}

pub fn bench_function_eq<T: Clone + Eq + UniformRand>(b: &mut Bencher) {
    b.iter_with_setup(
        || {
            let e_1 = rand::<T>();
            let e_2 = e_1.clone();
            (e_1, e_2)
        },
        |(e_1, e_2)| {
            let _res = e_1 == e_2;
        },
    )
}

pub fn bench_function_from_u64<T: From<u64> + UniformRand>(b: &mut Bencher) {
    b.iter_with_setup(rand::<u64>, |i| {
        let _res = T::from(i);
    })
}

pub fn bench_function_inv<T: Field + UniformRand>(b: &mut Bencher) {
    b.iter_with_setup(
        || rand::<T>(),
        |e| {
            let _e_inv = e.inverse();
        },
    )
}

pub fn bench_function_mul<T: Mul + UniformRand>(b: &mut Bencher) {
    b.iter_with_setup(
        || (rand::<T>(), rand::<T>()),
        |(e_1, e_2)| {
            let _e_3 = e_1 * e_2;
        },
    )
}

pub fn bench_function_neg<T: Neg + UniformRand>(b: &mut Bencher) {
    b.iter_with_setup(
        || rand::<T>(),
        |e| {
            let _e_2 = e.neg();
        },
    )
}

pub fn bench_function_pow_u256<T: Field + UniformRand>(b: &mut Bencher) {
    b.iter_with_setup(
        || {
            let base = rand::<T>();
            let exp = rand::<BigInteger256>();
            (base, exp)
        },
        |(base, exp)| {
            let _res = base.pow(exp);
        },
    )
}

pub fn bench_function_serialize_uncomp<T: CanonicalSerialize + UniformRand>(b: &mut Bencher) {
    b.iter_with_setup(
        || rand::<T>(),
        |e| {
            let mut buf = vec![];
            e.serialize_uncompressed(&mut buf).unwrap();
        },
    )
}

pub fn bench_function_square<T: Field + UniformRand>(b: &mut Bencher) {
    b.iter_with_setup(
        || rand::<T>(),
        |e| {
            let _res = e.square();
        },
    )
}

pub fn bench_function_sub<T: Sub + UniformRand>(b: &mut Bencher) {
    b.iter_with_setup(
        || (rand::<T>(), rand::<T>()),
        |(e, f)| {
            let _res = e - f;
        },
    )
}
