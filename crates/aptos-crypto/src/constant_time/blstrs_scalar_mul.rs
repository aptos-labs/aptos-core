// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use blstrs::{G1Projective, Scalar};
use dudect_bencher::{
    rand::{seq::SliceRandom, CryptoRng, Rng, RngCore},
    BenchRng, Class, CtRunner,
};
use group::Group;
use num_bigint::BigUint;
use std::{hint::black_box, ops::Mul};

const BIT_SIZE: usize = 255;
const N: usize = 5_000;

/// Runs a statistical test to check that blst's scalar multiplication on G1 is constant time
/// This function pick random bases for all scalar multiplications.
pub fn run_bench_with_random_bases(runner: &mut CtRunner, rng: &mut BenchRng) {
    build_and_run_bench(runner, rng, true, N);
}

/// Runs a statistical test to check that blst's scalar multiplication on G1 is constant time
/// This function keeps the multiplied base the same: the generator of G1.
pub fn run_bench_with_fixed_bases(runner: &mut CtRunner, rng: &mut BenchRng) {
    build_and_run_bench(runner, rng, false, N);
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
        let opt = Scalar::from_bytes_le(<&[u8; NUM_BYTES]>::try_from(bytes.as_slice()).unwrap());
        if opt.is_some().unwrap_u8() == 1 {
            return opt.unwrap();
        }
        // else: resample; this keeps the result uniform over valid k-bit elements
    }
}

/// WARNING: Blindly following the same "pattern" as in the dudect examples for how to "build" the
/// testcases. This coin flipping to decided whether to pick "left" or "right" feels awkward to me,
/// but I'd need to read their paper to understand better. It could've also been done by the
/// framework itself. The queing up of the inputs is also odd: why not run the benchmark immediately
/// after generating the input?
///
/// Note: We could technically implement this more abstractly via traits (may be painful) or macros,
/// since this is duplicated across this file and the `zkcrypto` file.
pub fn build_and_run_bench(
    runner: &mut CtRunner,
    rng: &mut BenchRng,
    random_bases: bool,
    num_iters: usize,
) {
    let mut inputs: Vec<(Class, usize, Scalar, G1Projective)> = Vec::with_capacity(N);

    let min_num_bits_left = 1;
    let max_num_bits_left = 4;
    let num_bits_right = 200; //BIT_SIZE.div_ceil(2) + 1;
    eprintln!();
    eprintln!(
        "# of 1 bits in scalars for \"left\" class is in [{}, {})",
        min_num_bits_left, max_num_bits_left
    );
    eprintln!(
        "# of 1 bits in scalars for \"right\" class is always {}",
        num_bits_right
    );

    for _ in 0..num_iters {
        let base = if random_bases {
            G1Projective::random(&mut *rng)
        } else {
            G1Projective::generator()
        };
        let choice = rng.r#gen::<bool>();

        if choice {
            // WARNING: `blstrs` is faster when the scalar is exactly 0!
            let num_bits_left = rng.gen_range(min_num_bits_left..max_num_bits_left);
            inputs.push((
                Class::Left,
                num_bits_left,
                random_scalar_with_k_bits_set(rng, num_bits_left),
                base,
            ));
        } else {
            inputs.push((
                Class::Right,
                num_bits_right,
                random_scalar_with_k_bits_set(rng, num_bits_right),
                base,
            ));
        }
    }

    for (class, _k, sk, base) in inputs {
        runner.run_one(class, || {
            let _ = black_box(base.mul(&sk));
        })
    }
}
