// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use ark_ff::Field;

/// Converts an Arkworks field element into chunks of `num_bits` bits (little-endian order).
pub fn chunk_field_elt<F: Field>(num_bits: usize, scalar: &F) -> Vec<F> {
    assert!(num_bits % 8 == 0, "Chunk size must be a multiple of 8");
    assert!(num_bits > 0 && num_bits <= 64, "Invalid chunk size");

    // Convert the field element to little-endian bytes
    let bytes = scalar.into_bigint().to_bytes_le();

    let num_bytes = num_bits / 8;
    let num_chunks = bytes.len().div_ceil(num_bytes);

    let mut chunks = Vec::with_capacity(num_chunks);

    for bytes_chunk in bytes.chunks(num_bytes) {
        // Pad to full byte length for conversion
        let mut padded = vec![0u8; F::MODULUS_BIT_SIZE.div_ceil(8) as usize];
        padded[..bytes_chunk.len()].copy_from_slice(bytes_chunk);

        // Convert the padded bytes back into a field element
        let bigint = F::BigInt::from_bytes_le(&padded);
        let chunk = F::from_bigint(bigint).expect("valid field element");
        debug_assert!(chunk < F::from(1u64 << num_bits));

        chunks.push(chunk);
    }

    chunks
}