// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use ark_ff::{BigInteger, PrimeField};

/// Converts a field element into little-endian chunks of `num_bits` bits.
/// Supports any chunk size from 1 to 64 bits (e.g. 43 or 52 bits). Made `pub` for tests.
pub fn scalar_to_le_chunks<F: PrimeField>(num_bits: u8, scalar: &F) -> Vec<F> {
    assert!(num_bits > 0 && num_bits <= 64, "Invalid chunk size");
    if num_bits.is_multiple_of(8) {
        return scalar_to_le_chunks_byte_aligned(num_bits, scalar);
    }

    let bytes = scalar.into_bigint().to_bytes_le();
    let total_bits = bytes.len() * 8;
    let num_chunks = total_bits.div_ceil(num_bits as usize);

    let mut chunks = Vec::with_capacity(num_chunks);

    for chunk_idx in 0..num_chunks {
        let start = chunk_idx * (num_bits as usize);
        let mut value: u64 = 0;
        for i in 0..num_bits {
            let bit_idx = start + (i as usize);
            if bit_idx < total_bits {
                let byte_idx = bit_idx / 8;
                let bit_in_byte = bit_idx % 8;
                let bit = (bytes[byte_idx] >> bit_in_byte) & 1;
                value |= (bit as u64) << i;
            }
        }
        chunks.push(F::from(value));
    }

    chunks
}

/// Byte-aligned chunking: same as `scalar_to_le_chunks` but requires `num_bits` to be a multiple of 8 (8, 16, 32, 64).
/// Faster for those sizes since it works on whole bytes. Made `pub` for tests and benchmarks.
///
/// Benchmarks suggest it's 1.5x faster than `scalar_to_le_chunks` for 32 bits, and 2x faster than `scalar_to_le_chunks` for 64 bits.
pub fn scalar_to_le_chunks_byte_aligned<F: PrimeField>(num_bits: u8, scalar: &F) -> Vec<F> {
    assert!(
        num_bits.is_multiple_of(8) && num_bits > 0 && num_bits <= 64,
        "Invalid chunk size (must be multiple of 8)"
    );

    let bytes = scalar.into_bigint().to_bytes_le();
    let num_bytes = num_bits / 8;
    let num_chunks = bytes.len().div_ceil(num_bytes as usize);

    let mut chunks = Vec::with_capacity(num_chunks);

    for bytes_chunk in bytes.chunks(num_bytes as usize) {
        let mut padded = [0u8; 8]; // The last chunk might be shorter, so this guarantees a fixed 8-byte buffer
        padded[..bytes_chunk.len()].copy_from_slice(bytes_chunk);

        let chunk_val = u64::from_le_bytes(padded);
        let chunk = F::from(chunk_val);
        chunks.push(chunk);
    }

    chunks
}

/// Reconstructs a field element from `num_bits`-bit chunks (little-endian order). Made `pub` for tests
/// This is the inverse of `scalar_to_le_chunks`.
/// Chunks must be in the range [0, 2^num_bits).
pub fn le_chunks_to_scalar<F: PrimeField>(num_bits: u8, chunks: &[F]) -> F {
    assert!(num_bits > 0 && num_bits <= 64, "Invalid chunk size");

    let base = F::from(1u128 << num_bits); // need u128 in the case where `num_bits` is 64, because of `chunk * multiplier`
    let mut acc = F::zero();
    let mut multiplier = F::one(); // TODO: could precompute these, if it gives a speedup

    for &chunk in chunks {
        acc += chunk * multiplier;
        multiplier *= base;
    }

    acc
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bls12_381::Fr;
    use ark_ff::{UniformRand, Zero};
    use ark_std::test_rng;

    #[test]
    fn test_chunk_unchunk_roundtrip() {
        let mut rng = test_rng();
        let num_bits_list = [1, 7, 8, 16, 32, 43, 52, 64];

        for &num_bits in &num_bits_list {
            for _ in 0..100 {
                let original: Fr = Fr::rand(&mut rng);
                let chunks = scalar_to_le_chunks(num_bits, &original);
                let reconstructed = le_chunks_to_scalar(num_bits, &chunks);

                assert_eq!(
                    original, reconstructed,
                    "Roundtrip failed for num_bits={}",
                    num_bits
                );
            }
        }
    }

    /// Zero roundtrips: chunk then reconstruct yields zero. (Zero is represented as one chunk [0], not empty.)
    #[test]
    fn test_zero_roundtrips() {
        let zero = Fr::zero();
        for &num_bits in &[1u8, 7, 8, 16, 32, 52, 64] {
            let chunks = scalar_to_le_chunks(num_bits, &zero);
            let reconstructed = le_chunks_to_scalar(num_bits, &chunks);
            assert_eq!(
                reconstructed, zero,
                "zero must roundtrip for num_bits={}",
                num_bits
            );
        }
    }

    /// Byte-aligned and arbitrary-bit chunking agree for 8, 16, 32, 64.
    #[test]
    fn test_byte_aligned_matches_arbitrary() {
        let mut rng = test_rng();
        for &num_bits in &[8u8, 16, 32, 64] {
            for _ in 0..50 {
                let scalar: Fr = Fr::rand(&mut rng);
                let chunks_arb = scalar_to_le_chunks(num_bits, &scalar);
                let chunks_byte = scalar_to_le_chunks_byte_aligned(num_bits, &scalar);
                assert_eq!(
                    chunks_arb, chunks_byte,
                    "byte_aligned and arbitrary must match for num_bits={}",
                    num_bits
                );
            }
        }
    }
}
