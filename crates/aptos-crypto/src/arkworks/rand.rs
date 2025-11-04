// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module provides functions to sample random elements from cryptographic
//! structures such as prime fields and elliptic curve groups; `arkworks`` can
//! do this by itself but the point here is to do it with our version of the
//! `rand`` crate.

use ark_ff::PrimeField;
use rand::Rng;

/// Samples a uniformly random element from the prime field `F`.
pub fn sample_field_element<F: PrimeField, R: Rng>(rng: &mut R) -> F {
    loop {
        // Number of bytes needed for F
        let num_bits = F::MODULUS_BIT_SIZE as usize;
        let num_bytes = num_bits.div_ceil(8);

        // Draw enough random bytes to cover the field size
        let mut bytes = vec![0u8; num_bytes];
        rng.fill_bytes(&mut bytes);

        // Mask away unused bits (so we don't exceed modulus size)
        let excess_bits = num_bytes * 8 - num_bits;
        if excess_bits > 0 {
            let mask = 0xFFu8 >> excess_bits;
            bytes[0] &= mask;
        }

        // Interpret as little-endian integer mod p (rejection sampling)
        if let Some(f) = F::from_random_bytes(&bytes) {
            return f;
        }
    }
}
