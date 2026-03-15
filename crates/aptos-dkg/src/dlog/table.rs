// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use ark_ec::{AffineRepr, CurveGroup};
use ark_ff::Zero;
use std::collections::HashMap;

/// Baby-step table plus precomputed giant-step term: holds points j*G for j in [0, m),
/// and stores G, table_size, and the precomputed -table_size*G for the giant-step loop.
#[allow(non_snake_case)]
#[derive(Clone)]
pub struct BabyStepTable<A: AffineRepr> {
    /// Base point for baby steps (point = j*G).
    pub G: A,
    /// Baby steps: affine point -> exponent j (so that point = affinisation of j*G).
    pub table: HashMap<A, u32>,
    /// Number of baby steps (table length).
    pub table_size: u32,
    /// Precomputed -table_size*G for giant steps.
    pub G_neg_table_size: A,
}

impl<A: AffineRepr> BabyStepTable<A> {
    /// Builds the baby-step table and precomputes `table_size` and `G_neg_table_size`.
    ///
    /// Computes all points [0, G, 2*G, ...] in projective form, batch-normalizes once,
    /// then inserts each affine point into the table.
    #[allow(non_snake_case)]
    pub fn new(G: A, table_size: u32) -> Self {
        let table_size_as_usize = table_size as usize;
        let mut points: Vec<A::Group> = Vec::with_capacity(table_size_as_usize);
        let mut current = A::Group::zero();
        for _ in 0..table_size {
            points.push(current);
            current += G;
        }
        let normalized = A::Group::normalize_batch(&points);
        let mut table = HashMap::with_capacity(table_size_as_usize);
        for (j, aff) in normalized.into_iter().enumerate() {
            table.insert(aff, j as u32);
        }
        let G_neg_table_size = (G * -A::ScalarField::from(table_size)).into_affine();
        Self {
            G,
            table,
            table_size,
            G_neg_table_size,
        }
    }
}
