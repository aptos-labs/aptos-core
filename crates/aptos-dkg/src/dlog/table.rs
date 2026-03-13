// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use ark_ec::CurveGroup;
use std::collections::HashMap;

/// Build a baby-step table of size `table_size` using batch normalization.
///
/// Computes all points [0, G, 2*G, ...] in projective form, calls `normalize_batch` once,
/// then inserts each affine point into the table. Normalization cost is amortized (one batch
/// inversion instead of many single inversions). Lookups use the affine point directly (no
/// serialization in the BSGS loop).
///
/// Returns a HashMap: `C::Affine |---> exponent`
#[allow(non_snake_case)]
pub fn build<C: CurveGroup>(G: C, table_size: u64) -> HashMap<C::Affine, u64> {
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
        table.insert(aff, j as u64);
    }

    table
}

#[allow(non_snake_case)]
pub fn build_default<C: CurveGroup>(table_size: u64) -> HashMap<C::Affine, u64> {
    let G = C::generator();
    build(G, table_size)
}
