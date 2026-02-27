// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Shared helpers for converting scalar values to bit matrices used by range proof provers.

use ark_ff::PrimeField;

/// Transposes a bit matrix: `bits[i]` is row i.
/// Returns a matrix where row `j` is column `j` of the input, i.e. `out[j][i] == bits[i][j]`.
/// Column count is taken from the first row; returns empty if `bits` is empty.
pub fn transpose_bit_matrix(bit_matrix: &[Vec<bool>]) -> Vec<Vec<bool>> {
    let num_cols = match bit_matrix.first() {
        Some(row) => row.len(),
        None => return vec![],
    };
    (0..num_cols)
        .map(|j| bit_matrix.iter().map(|row| row[j]).collect())
        .collect()
}

/// Converts each field scalar to its first `number_of_bits` bits in little-endian order.
pub fn scalars_to_bits_le<F: PrimeField>(scalars: &[F], number_of_bits: u8) -> Vec<Vec<bool>> {
    scalars
        .iter()
        .map(|scalar| {
            crate::utils::scalar_to_bits_le(scalar)
                .into_iter()
                .take(number_of_bits as usize)
                .collect::<Vec<_>>()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::Fr;

    #[test]
    fn test_matches_inline_computation() {
        let scalars: Vec<Fr> = vec![
            Fr::from(0u64),
            Fr::from(42u64),
            Fr::from(1u64),
            Fr::from(100u64),
        ];
        let number_of_bits: u8 = 8;

        let bits = scalars_to_bits_le::<Fr>(&scalars, number_of_bits);
        let transposed = transpose_bit_matrix(&bits);
        assert_eq!(transposed.len(), number_of_bits as usize);
        assert_eq!(transposed[0].len(), scalars.len());
        // Roundtrip: transpose(transpose(bits)) should match bits
        let bits_rt = transpose_bit_matrix(&transposed);
        assert_eq!(bits, bits_rt);
    }
}
