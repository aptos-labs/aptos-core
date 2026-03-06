// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use ark_serialize::CanonicalSerialize;

pub mod shplonked;
pub(crate) mod shplonked_sigma;
pub mod traits;
pub mod univariate_hiding_kzg;
pub mod univariate_kzg;
pub mod zeromorph;
//pub mod zk_samaritan;

// ---------------------------------------------------------------------------
// Generalized evaluation sets and zero polynomials
// ---------------------------------------------------------------------------

/// Per-polynomial evaluation set: S_i = S_i^rev ⊔ S_i^hid.
/// Order of points in `rev` and `hid` determines the flat index in y^rev and y^hid.
#[derive(CanonicalSerialize, Clone, Debug)]
pub struct EvaluationSet<F: CanonicalSerialize> {
    /// Points at which the prover reveals the evaluation (y^rev).
    pub rev: Vec<F>,
    /// Points at which the evaluation is hidden (y^hid); commitment C_{y^hid} is sent.
    pub hid: Vec<F>,
}

impl<F: CanonicalSerialize> EvaluationSet<F> {
    /// All points in this set (rev first, then hid).
    pub fn all_points(&self) -> impl Iterator<Item = &F> {
        self.rev.iter().chain(self.hid.iter())
    }

    pub fn len(&self) -> usize {
        self.rev.len() + self.hid.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rev.is_empty() && self.hid.is_empty()
    }
}
