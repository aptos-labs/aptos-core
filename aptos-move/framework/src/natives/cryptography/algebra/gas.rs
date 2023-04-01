// Copyright © Aptos Foundation

// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::natives::cryptography::algebra::{HashToStructureSuite, Structure};
use move_core_types::gas_algebra::{InternalGas, InternalGasPerArg, NumArgs};

fn log2_floor(n: usize) -> usize {
    (0_usize.leading_zeros() - n.leading_zeros()) as usize
}

fn log2_ceil(n: usize) -> usize {
    log2_floor(n - 1) + 1
}

fn ark_msm_window_size(num_entries: usize) -> usize {
    if num_entries < 32 {
        3
    } else {
        (log2_ceil(num_entries) * 69 / 100) + 2
    }
}

/// The approximate cost model of https://github.com/arkworks-rs/algebra/blob/v0.4.0/ec/src/scalar_mul/variable_base/mod.rs#L89.
fn ark_msm_bigint_wnaf_cost(
    cost_add: InternalGasPerArg,
    cost_double: InternalGasPerArg,
    num_entries: usize,
) -> InternalGas {
    let window_size = ark_msm_window_size(num_entries);
    let num_windows = (255 + window_size - 1) / window_size;
    let num_buckets = 1_usize << window_size;
    cost_add * NumArgs::from(((num_entries + num_buckets + 1) * num_windows) as u64)
        + cost_double * NumArgs::from((num_buckets * num_windows) as u64)
}

#[derive(Debug, Clone)]
pub struct GasParameters {
    pub ark_bls12_381_fr_add: InternalGasPerArg,
    pub ark_bls12_381_fr_deser: InternalGasPerArg,
    pub ark_bls12_381_fr_div: InternalGasPerArg,
    pub ark_bls12_381_fr_eq: InternalGasPerArg,
    pub ark_bls12_381_fr_from_u64: InternalGasPerArg,
    pub ark_bls12_381_fr_inv: InternalGasPerArg,
    pub ark_bls12_381_fr_mul: InternalGasPerArg,
    pub ark_bls12_381_fr_neg: InternalGasPerArg,
    pub ark_bls12_381_fr_one: InternalGasPerArg,
    pub ark_bls12_381_fr_serialize: InternalGasPerArg,
    pub ark_bls12_381_fr_square: InternalGasPerArg,
    pub ark_bls12_381_fr_sub: InternalGasPerArg,
    pub ark_bls12_381_fr_zero: InternalGasPerArg,
    pub ark_bls12_381_fq12_add: InternalGasPerArg,
    pub ark_bls12_381_fq12_clone: InternalGasPerArg,
    pub ark_bls12_381_fq12_deser: InternalGasPerArg,
    pub ark_bls12_381_fq12_div: InternalGasPerArg,
    pub ark_bls12_381_fq12_eq: InternalGasPerArg,
    pub ark_bls12_381_fq12_from_u64: InternalGasPerArg,
    pub ark_bls12_381_fq12_inv: InternalGasPerArg,
    pub ark_bls12_381_fq12_mul: InternalGasPerArg,
    pub ark_bls12_381_fq12_neg: InternalGasPerArg,
    pub ark_bls12_381_fq12_one: InternalGasPerArg,
    pub ark_bls12_381_fq12_pow_u256: InternalGasPerArg,
    pub ark_bls12_381_fq12_serialize: InternalGasPerArg,
    pub ark_bls12_381_fq12_square: InternalGasPerArg,
    pub ark_bls12_381_fq12_sub: InternalGasPerArg,
    pub ark_bls12_381_fq12_zero: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_deser_comp: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_deser_uncomp: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_serialize_comp: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_serialize_uncomp: InternalGasPerArg,
    pub ark_bls12_381_g1_proj_add: InternalGasPerArg,
    pub ark_bls12_381_g1_proj_double: InternalGasPerArg,
    pub ark_bls12_381_g1_proj_eq: InternalGasPerArg,
    pub ark_bls12_381_g1_proj_generator: InternalGasPerArg,
    pub ark_bls12_381_g1_proj_infinity: InternalGasPerArg,
    pub ark_bls12_381_g1_proj_neg: InternalGasPerArg,
    pub ark_bls12_381_g1_proj_scalar_mul: InternalGasPerArg,
    pub ark_bls12_381_g1_proj_sub: InternalGasPerArg,
    pub ark_bls12_381_g1_proj_to_affine: InternalGasPerArg,
    pub ark_bls12_381_g2_affine_deser_comp: InternalGasPerArg,
    pub ark_bls12_381_g2_affine_deser_uncomp: InternalGasPerArg,
    pub ark_bls12_381_g2_affine_serialize_comp: InternalGasPerArg,
    pub ark_bls12_381_g2_affine_serialize_uncomp: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_add: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_double: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_eq: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_generator: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_infinity: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_neg: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_scalar_mul: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_sub: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_to_affine: InternalGasPerArg,
    pub ark_bls12_381_pairing: InternalGasPerArg,
    pub ark_bls12_381_multi_pairing_base: InternalGasPerArg,
    pub ark_bls12_381_multi_pairing_per_pair: InternalGasPerArg,
    pub ark_h2c_bls12381g1_xmd_sha256_sswu_base: InternalGasPerArg,
    pub ark_h2c_bls12381g1_xmd_sha256_sswu_per_msg_byte: InternalGasPerArg,
    pub ark_h2c_bls12381g2_xmd_sha256_sswu_base: InternalGasPerArg,
    pub ark_h2c_bls12381g2_xmd_sha256_sswu_per_msg_byte: InternalGasPerArg,
}

impl GasParameters {
    pub fn group_multi_scalar_mul(&self, structure: Structure, num_entries: usize) -> InternalGas {
        match structure {
            Structure::BLS12381G1Affine => ark_msm_bigint_wnaf_cost(
                self.ark_bls12_381_g1_proj_add,
                self.ark_bls12_381_g1_proj_double,
                num_entries,
            ),
            Structure::BLS12381G2Affine => ark_msm_bigint_wnaf_cost(
                self.ark_bls12_381_g2_proj_add,
                self.ark_bls12_381_g2_proj_double,
                num_entries,
            ),
            _ => unreachable!(),
        }
    }

    pub fn hash_to(
        &self,
        suite: HashToStructureSuite,
        _dst_len: usize,
        msg_len: usize,
    ) -> InternalGas {
        match suite {
            HashToStructureSuite::Bls12381g1XmdSha256SswuRo => {
                // Simplified formula, by fixing `dst_len` to be its maximum value (255).
                self.ark_h2c_bls12381g1_xmd_sha256_sswu_base * NumArgs::one()
                    + self.ark_h2c_bls12381g1_xmd_sha256_sswu_per_msg_byte
                        * NumArgs::from(msg_len as u64)
            },
            HashToStructureSuite::Bls12381g2XmdSha256SswuRo => {
                // Simplified formula, by fixing `dst_len` to be its maximum value (255).
                self.ark_h2c_bls12381g2_xmd_sha256_sswu_base * NumArgs::one()
                    + self.ark_h2c_bls12381g2_xmd_sha256_sswu_per_msg_byte
                        * NumArgs::from(msg_len as u64)
            },
        }
    }

    pub fn multi_pairing(
        &self,
        g1: Structure,
        g2: Structure,
        g3: Structure,
        num_pairs: usize,
    ) -> InternalGas {
        match (g1, g2, g3) {
            (Structure::BLS12381G1Affine, Structure::BLS12381G2Affine, Structure::BLS12381Gt) => {
                self.ark_bls12_381_multi_pairing_base * NumArgs::one()
                    + self.ark_bls12_381_multi_pairing_per_pair * NumArgs::from(num_pairs as u64)
            },
            _ => unreachable!(),
        }
    }

    pub fn pairing(&self, g1: Structure, g2: Structure, g3: Structure) -> InternalGas {
        match (g1, g2, g3) {
            (Structure::BLS12381G1Affine, Structure::BLS12381G2Affine, Structure::BLS12381Gt) => {
                (self.ark_bls12_381_pairing) * NumArgs::one()
                    + self.ark_bls12_381_g1_proj_to_affine * NumArgs::one()
                    + self.ark_bls12_381_g2_proj_to_affine * NumArgs::one()
            },
            _ => unreachable!(),
        }
    }
}
