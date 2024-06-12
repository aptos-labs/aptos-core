// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::TestCircuitHandle;
use aptos_keyless_common::input_processing::{
    circuit_input_signals::CircuitInputSignals, config::CircuitPaddingConfig,
};
use aptos_logger::info;
use rand::{thread_rng, Rng};

/// Given a non-negative integer `x`, generate a non-negative integer `y` that satisfies `y < x`.
/// `x` and `y` are both encoded to byte array with the most significant byte first.
fn rand_big<R: Rng>(rng: &mut R, x_bytes_be: &[u8]) -> Vec<u8> {
    let n = x_bytes_be.len();
    let non_zero_idxs: Vec<usize> = x_bytes_be
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(idx, byte)| if byte == 0 { None } else { Some(idx) })
        .collect();
    let diff_idx = non_zero_idxs[rng.gen_range(0, non_zero_idxs.len())]; // will be the first byte in y that's smaller than the same position in x.

    let y_diff_byte = rng.gen_range(0, x_bytes_be[diff_idx]);

    let mut random_bytes = vec![0; n - (diff_idx + 1)];
    rng.fill_bytes(random_bytes.as_mut_slice());

    [
        x_bytes_be[0..diff_idx].to_vec(),
        vec![y_diff_byte],
        random_bytes,
    ]
    .concat()
}

fn bytes_le_into_limbs_le(mut bytes: Vec<u8>) -> Vec<u64> {
    let num_bytes = bytes.len();
    let num_limbs = num_bytes.div_ceil(8);
    let num_pad_zeros = num_limbs * 8 - num_bytes;
    bytes.extend(vec![0; num_pad_zeros]);
    let mut ret = vec![0; num_limbs];
    for i in 0..num_limbs {
        let arr = <[u8; 8]>::try_from(bytes[i * 8..(i + 1) * 8].to_vec()).unwrap();
        ret[i] = u64::from_le_bytes(arr);
    }
    ret
}

/// Sample a random integer in range [0, 2^2048) and return its big-endian byte representation.
/// Some special ranges get higher weight.
fn sample_big_be<R: Rng>(rng: &mut R) -> Vec<u8> {
    let byte_len: usize = {
        let sample = rng.gen_range(0.0, 1.0);
        if sample < 0.2 {
            rng.gen_range(1, 9)
        } else if sample < 0.4 {
            rng.gen_range(248, 257)
        } else {
            rng.gen_range(1, 33) * 8
        }
    };
    let mut ret: Vec<u8> = vec![0; byte_len];
    rng.fill_bytes(ret.as_mut_slice());
    if let Some(byte) = ret.first_mut() {
        *byte = rng.gen_range(0, 255) + 1;
    }
    ret
}

fn common<F: FnMut() -> (Vec<u8>, Vec<u8>)>(mut a_b_provider: F, expected_output: bool) {
    let circuit = TestCircuitHandle::new("bigint/big_less_than_test.circom").unwrap();
    let num_iterations = std::env::var("NUM_ITERATIONS")
        .unwrap_or("100".to_string())
        .parse::<usize>()
        .unwrap_or(100);
    for i in 0..num_iterations {
        info!("Iteration {i} starts. Generate big numbers.");
        let (a_bytes_be, b_bytes_be) = a_b_provider();
        println!("a_hex={}", hex::encode(&a_bytes_be));
        println!("b_hex={}", hex::encode(&b_bytes_be));
        let a_bytes_le: Vec<u8> = a_bytes_be.into_iter().rev().collect();
        let a_limbs_le = bytes_le_into_limbs_le(a_bytes_le);
        let b_bytes_le: Vec<u8> = b_bytes_be.into_iter().rev().collect();
        let b_limbs_le = bytes_le_into_limbs_le(b_bytes_le);

        let config = CircuitPaddingConfig::new()
            .max_length("a", 32)
            .max_length("b", 32);
        let circuit_input_signals = CircuitInputSignals::new()
            .limbs_input("a", a_limbs_le.as_slice())
            .limbs_input("b", b_limbs_le.as_slice())
            .bool_input("expected_output", expected_output)
            .pad(&config)
            .unwrap();
        let result = circuit.gen_witness(circuit_input_signals);
        println!("result={:?}", result);
        assert!(result.is_ok());
    }
}

#[test]
fn big_less_than_should_return_1_if_a_lt_b() {
    let mut rng = thread_rng();
    common(
        || {
            let b_bytes_be = sample_big_be(&mut rng);
            let a_bytes_be = rand_big(&mut rng, b_bytes_be.as_slice());
            (a_bytes_be, b_bytes_be)
        },
        true,
    );
}

#[test]
fn big_less_than_should_return_0_if_a_gt_b() {
    let mut rng = thread_rng();
    common(
        || {
            let a_bytes_be = sample_big_be(&mut rng);
            let b_bytes_be = rand_big(&mut rng, a_bytes_be.as_slice());
            (a_bytes_be, b_bytes_be)
        },
        false,
    );
}

#[test]
fn big_less_than_should_return_0_if_a_eq_b() {
    let mut rng = thread_rng();
    common(
        || {
            let b_bytes_be = sample_big_be(&mut rng);
            let a_bytes_be = b_bytes_be.clone();
            (a_bytes_be, b_bytes_be)
        },
        false,
    );
}
