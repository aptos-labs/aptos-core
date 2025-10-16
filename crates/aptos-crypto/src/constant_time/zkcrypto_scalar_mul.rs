// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use bls12_381::{G1Projective, Scalar};
use dudect_bencher::{
    rand::{seq::SliceRandom, CryptoRng, Rng, RngCore},
    BenchRng, Class, CtRunner,
};
use num_bigint::BigUint;
use std::{hint::black_box, ops::Mul};

const BIT_SIZE: usize = 255;

/// Runs a statistical test to check that zkcrypto's scalar multiplication on G1 is constant time.
pub fn run_bench(runner: &mut CtRunner, rng: &mut BenchRng) {
    build_and_run_bench(runner, rng, |sk, g1| g1.mul(sk));
}

fn random_scalar_with_k_bits_set<R: CryptoRng + RngCore>(rng: &mut R, k: usize) -> Scalar {
    const NUM_BYTES: usize = BIT_SIZE.div_ceil(8);
    // Note: if k == 255 => all bits will be set to 1 => infinite loop
    // (i.e., the sorted version of `selected` will always be [0, 1, ..., 254])
    assert!(
        k < BIT_SIZE,
        "k must be < the field's bit size {}",
        BIT_SIZE
    );

    loop {
        // uniformly pick k distinct bit positions
        let mut positions: Vec<u64> = (0..(BIT_SIZE as u64)).collect();
        positions.shuffle(rng);
        let selected = &positions[..k];

        // build the integer with those bits set
        let mut bigint = BigUint::default();
        for &bit in selected {
            bigint.set_bit(bit, true);
        }

        // accept only if < modulus (i.e., a valid canonical representative)
        let mut bytes = bigint.to_bytes_le();
        while bytes.len() < NUM_BYTES {
            bytes.push(0u8);
        }
        let opt = Scalar::from_bytes(<&[u8; NUM_BYTES]>::try_from(bytes.as_slice()).unwrap());
        if opt.is_some().unwrap_u8() == 1 {
            return opt.unwrap();
        }
        // else: resample; this keeps the result uniform over valid k-bit elements
    }
}

/// WARNING: See comment in `build_and_run_bench` in blstrs_scalar_mul.rs
fn build_and_run_bench<F>(runner: &mut CtRunner, rng: &mut BenchRng, scalar_mul_fn: F)
where
    F: Fn(&Scalar, &G1Projective) -> G1Projective,
{
    let g1 = G1Projective::generator();

    const N: usize = 10_000;

    let mut inputs: Vec<(Class, usize, Scalar, G1Projective)> = Vec::with_capacity(N);

    let min_num_bits_left = 0;
    let max_num_bits_left = 4;
    let num_bits_right = BIT_SIZE.div_ceil(2) + 1;
    eprintln!();
    eprintln!(
        "# of 1 bits in scalars for \"left\" class is in [{}, {})",
        min_num_bits_left, max_num_bits_left
    );
    eprintln!(
        "# of 1 bits in scalars for \"right\" class is always {}",
        num_bits_right
    );
    for _ in 0..N {
        let choice = rng.r#gen::<bool>();

        if choice {
            let num_bits_left = rng.gen_range(min_num_bits_left..max_num_bits_left);
            inputs.push((
                Class::Left,
                num_bits_left,
                random_scalar_with_k_bits_set(rng, num_bits_left),
                g1,
            ));
        } else {
            inputs.push((
                Class::Right,
                num_bits_right,
                random_scalar_with_k_bits_set(rng, num_bits_right),
                g1,
            ));
        }
    }

    for (class, _k, sk, base) in inputs {
        runner.run_one(class, || {
            black_box(scalar_mul_fn(&sk, &base));
        })
    }
}
