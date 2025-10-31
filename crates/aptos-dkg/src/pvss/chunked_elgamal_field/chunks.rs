// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use ark_ff::{BigInteger, PrimeField};

/// Converts a field element into little-endian chunks of `num_bits` bits.
#[allow(dead_code)]
pub(crate) fn chunk_field_elt<F: PrimeField>(num_bits: usize, scalar: &F) -> Vec<F> {
    assert!(
        num_bits % 8 == 0 && num_bits > 0 && num_bits <= 64,
        "Invalid chunk size"
    );

    let bytes = scalar.into_bigint().to_bytes_le();
    let num_bytes = num_bits / 8;
    let num_chunks = (bytes.len() + num_bytes - 1) / num_bytes;

    let mut chunks = Vec::with_capacity(num_chunks);

    for bytes_chunk in bytes.chunks(num_bytes) {
        // Copy into a fixed 8-byte array (up to 64 bits)
        let mut padded = [0u8; 8];
        padded[..bytes_chunk.len()].copy_from_slice(bytes_chunk);

        let chunk_val = u64::from_le_bytes(padded);
        let chunk = F::from(chunk_val);
        chunks.push(chunk);
    }

    chunks
}

/// Reconstructs a field element from `num_bits`-bit chunks (little-endian order).
#[allow(dead_code)]
pub(crate) fn unchunk_field_elt<F: PrimeField>(num_bits: usize, chunks: &[F]) -> F {
    assert!(
        num_bits % 8 == 0 && num_bits > 0 && num_bits <= 64,
        "Invalid chunk size"
    );

    let base = F::from((1u128 << num_bits) as u128);
    let mut acc = F::zero();
    let mut multiplier = F::one();

    for chunk in chunks {
        acc += *chunk * multiplier;
        multiplier *= base;
    }

    acc
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bls12_381::Fr;
    use ark_ff::UniformRand;
    use ark_std::test_rng;

    #[test]
    fn test_chunk_unchunk_roundtrip() {
        let mut rng = test_rng();
        let num_bits_list = [8, 16, 32, 64]; // include 64 bits now

        for &num_bits in &num_bits_list {
            for _ in 0..100 {
                let original: Fr = Fr::rand(&mut rng);
                let chunks = chunk_field_elt(num_bits, &original);
                let reconstructed = unchunk_field_elt(num_bits, &chunks);

                assert_eq!(
                    original, reconstructed,
                    "Roundtrip failed for num_bits={}",
                    num_bits
                );
            }
        }
    }
}
