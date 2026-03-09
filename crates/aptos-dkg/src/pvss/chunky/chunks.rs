// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use ark_ff::{BigInteger, PrimeField};

/// Converts a field element into little-endian chunks of `num_bits` bits. Made `pub` for tests
pub fn scalar_to_le_chunks<F: PrimeField>(num_bits: u8, scalar: &F) -> Vec<F> {
    assert!(
        num_bits.is_multiple_of(8) && num_bits > 0 && num_bits <= 64,
        "Invalid chunk size"
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
    assert!(
        num_bits.is_multiple_of(8) && num_bits > 0 && num_bits <= 64,
        "Invalid chunk size"
    );

    let base = F::from(1u128 << num_bits); // need u128 in the case where `num_bits` is 64, because of `chunk * multiplier`
    let mut acc = F::zero();
    let mut multiplier = F::one();

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
        let num_bits_list = [8, 16, 32, 64];

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
        for &num_bits in &[8u8, 16, 32, 64] {
            let chunks = scalar_to_le_chunks(num_bits, &zero);
            let reconstructed = le_chunks_to_scalar(num_bits, &chunks);
            assert_eq!(
                reconstructed, zero,
                "zero must roundtrip for num_bits={}",
                num_bits
            );
        }
    }
}
