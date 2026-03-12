// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use ark_ec::CurveGroup;
use ark_serialize::CanonicalSerialize;
use std::collections::HashMap;

/// Build a baby-step table of size `table_size` using batch normalization.
///
/// Computes all points [0, G, 2*G, ...] in projective form, calls `normalize_batch` once,
/// then serializes each affine point and inserts into the table. Normalization cost is
/// amortized (one batch inversion instead of many single inversions); benchmarks suggest a 13x speedup.
///
/// Returns a HashMap: `C.to_compressed() |---> exponent`
#[allow(non_snake_case)]
pub fn build<C: CurveGroup>(G: C, table_size: u64) -> HashMap<Vec<u8>, u64> {
    let byte_size = G.compressed_size();

    // Collect all projective points: 0, G, 2*G, ..., (table_size - 1)*G
    let mut points: Vec<C> = Vec::with_capacity(table_size as usize);
    let mut current = C::zero();
    for _ in 0..table_size {
        points.push(current);
        current += G;
    }

    let normalized = C::normalize_batch(&points);

    let mut table = HashMap::with_capacity(table_size as usize);
    for (j, aff) in normalized.into_iter().enumerate() {
        let mut buf = vec![0u8; byte_size];
        aff.serialize_compressed(&mut &mut buf[..]).unwrap();
        table.insert(buf, j as u64);
    }

    table
}

#[allow(non_snake_case)]
pub fn build_default<C: CurveGroup>(table_size: u64) -> HashMap<Vec<u8>, u64> {
    let G = C::generator();
    build(G, table_size)
}
