// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use ark_ec::{AffineRepr, CurveGroup};
use ark_ff::Zero;
use ark_serialize::{CanonicalSerialize, Compress};
use std::collections::HashMap;

/// Maximum compressed point size we support as an inline HashMap key.
/// BLS12-381 G1 compressed = 48 bytes, BN254 G1 compressed = 32 bytes.
const MAX_COMPRESSED_POINT_SIZE: usize = 48;

/// Fixed-size inline key for the baby-step table, stored directly in the HashMap
/// without per-entry heap allocations.
type PointKey = [u8; MAX_COMPRESSED_POINT_SIZE];

/// Baby-step table plus precomputed giant-step term: holds points j*G for j in [0, m),
/// and stores G, table_size, and the precomputed -table_size*G for the giant-step loop.
///
/// Compressed points are stored as fixed-size inline byte arrays in the HashMap,
/// avoiding millions of individual heap allocations.
#[allow(non_snake_case)]
#[derive(Clone)]
pub struct BabyStepTable<A: AffineRepr> {
    /// Base point for baby steps (point = j*G).
    pub G: A,
    /// Baby steps: compressed(point) -> exponent j (so that point = affinisation of j*G).
    table: HashMap<PointKey, u32>,
    /// Actual number of bytes used in each key (may be less than MAX_COMPRESSED_POINT_SIZE).
    key_size: usize,
    /// Number of baby steps (table length).
    pub table_size: u32,
    /// Precomputed -table_size*G for giant steps.
    pub G_neg_table_size: A,
}

impl<A: AffineRepr> BabyStepTable<A> {
    /// Builds the baby-step table and precomputes `table_size` and `G_neg_table_size`.
    ///
    /// Computes all points [0, G, 2*G, ...] in projective form, batch-normalizes once,
    /// then inserts each compressed point into the table using a fixed-size inline key.
    #[allow(non_snake_case)]
    pub fn new(G: A, table_size: u32) -> Self {
        let table_size_as_usize = table_size as usize;
        let key_size = G.compressed_size();
        assert!(
            key_size <= MAX_COMPRESSED_POINT_SIZE,
            "compressed point size ({key_size}) exceeds MAX_COMPRESSED_POINT_SIZE ({MAX_COMPRESSED_POINT_SIZE})"
        );

        // 1. Compute all multiples in projective form.
        let mut points: Vec<A::Group> = Vec::with_capacity(table_size_as_usize);
        let mut current = A::Group::zero();
        for _ in 0..table_size {
            points.push(current);
            current += G;
        }

        // 2. Batch normalize to affine.
        let normalized = A::Group::normalize_batch(&points);
        // Free the projective points before building the table.
        drop(points);

        // 3. Insert compressed points into the HashMap with inline keys.
        let mut table = HashMap::with_capacity(table_size_as_usize);
        for (j, aff) in normalized.into_iter().enumerate() {
            let key = compress_to_key(&aff, key_size);
            table.insert(key, j as u32);
        }

        let G_neg_table_size = (G * -A::ScalarField::from(table_size)).into_affine();
        Self {
            G,
            table,
            key_size,
            table_size,
            G_neg_table_size,
        }
    }

    /// Look up an affine point in the table; returns the exponent j if point = j*G.
    pub fn get(&self, point: &A) -> Option<u32> {
        let key = compress_to_key(point, self.key_size);
        self.table.get(&key).copied()
    }

    /// Approximate memory size of the table in gigabytes (inline key + value bytes; HashMap overhead not included).
    pub fn size_gb(&self) -> f64 {
        let bytes_approx =
            self.table.len() * (MAX_COMPRESSED_POINT_SIZE + std::mem::size_of::<u32>());
        bytes_approx as f64 / 1e9
    }
}

/// Serialize an affine point into a fixed-size inline key, zero-padded if the
/// compressed representation is shorter than `MAX_COMPRESSED_POINT_SIZE`.
fn compress_to_key<A: CanonicalSerialize>(point: &A, key_size: usize) -> PointKey {
    let mut key = [0u8; MAX_COMPRESSED_POINT_SIZE];
    point
        .serialize_with_mode(&mut key[..key_size], Compress::Yes)
        .expect("baby-step table: serialization failed");
    key
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;
    use ark_bn254::{Fr, G1Affine};
    use ark_ec::{AffineRepr, CurveGroup};

    /// Table has exactly `table_size` entries and each j in [0, table_size) maps to j*G.
    #[test]
    fn table_entries_correct() {
        let G = G1Affine::generator();
        let table_size = 32u32;
        let tbl = BabyStepTable::<G1Affine>::new(G, table_size);

        assert_eq!(tbl.table_size, table_size);

        for j in 0..table_size {
            let point = (G * Fr::from(j)).into_affine();
            let stored = tbl.get(&point);
            assert_eq!(stored, Some(j), "table should map j*G -> j for j = {}", j);
        }
    }
}
