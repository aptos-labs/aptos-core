// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::pcs::univariate_hiding_kzg;
use ark_ec::pairing::Pairing;
use ark_serialize::CanonicalSerialize;

pub mod dekart_multivariate;
pub mod dekart_univariate;
pub mod dekart_univariate_v2;
pub mod scalars_to_bits;
pub mod traits;

// Both DeKART multivariate and univariate use the same "public statement" struct.
#[derive(CanonicalSerialize)]
pub struct PublicStatement<E: Pairing> {
    pub n: usize,
    pub ell: u8,
    pub comm: univariate_hiding_kzg::Commitment<E>,
}
