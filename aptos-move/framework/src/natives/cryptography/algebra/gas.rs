// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use move_core_types::gas_algebra::{InternalGas, InternalGasPerArg, NumArgs};
use crate::natives::cryptography::algebra::Structure;

#[derive(Debug, Clone)]
pub struct GasParameters {
    pub blst_g1_msm_base: InternalGasPerArg,
    pub blst_g1_msm_per_pair: InternalGasPerArg,
    pub blst_g2_msm_base: InternalGasPerArg,
    pub blst_g2_msm_per_pair: InternalGasPerArg,
    pub blst_g1_proj_to_affine: InternalGasPerArg,
    pub blst_g1_affine_ser: InternalGasPerArg,
    pub blst_g2_proj_to_affine: InternalGasPerArg,
    pub blst_g2_affine_ser: InternalGasPerArg,
    pub ark_bls12_381_fr_add: InternalGasPerArg,
    pub ark_bls12_381_fr_deser_base: InternalGasPerArg,
    pub ark_bls12_381_fr_deser_per_byte: InternalGasPerArg,
    pub ark_bls12_381_fr_div: InternalGasPerArg,
    pub ark_bls12_381_fr_eq: InternalGasPerArg,
    pub ark_bls12_381_fr_from_u128: InternalGasPerArg,
    pub ark_bls12_381_fr_inv: InternalGasPerArg,
    pub ark_bls12_381_fr_mul: InternalGasPerArg,
    pub ark_bls12_381_fr_neg: InternalGasPerArg,
    pub ark_bls12_381_fr_pow_base: InternalGasPerArg,
    pub ark_bls12_381_fr_pow_per_exponent_u64: InternalGasPerArg,
    pub ark_bls12_381_fr_ser: InternalGasPerArg,
    pub ark_bls12_381_fr_sub: InternalGasPerArg,
    pub ark_bls12_381_fr_to_repr: InternalGasPerArg,
    pub ark_bls12_381_fq12_add: InternalGasPerArg,
    pub ark_bls12_381_fq12_clone: InternalGasPerArg,
    pub ark_bls12_381_fq12_deserialize: InternalGasPerArg,
    pub ark_bls12_381_fq12_div: InternalGasPerArg,
    pub ark_bls12_381_fq12_eq: InternalGasPerArg,
    pub ark_bls12_381_fq12_inv: InternalGasPerArg,
    pub ark_bls12_381_fq12_mul: InternalGasPerArg,
    pub ark_bls12_381_fq12_one: InternalGasPerArg,
    pub ark_bls12_381_fq12_pow_base: InternalGasPerArg,
    pub ark_bls12_381_fq12_pow_per_exponent_u64: InternalGasPerArg,
    pub ark_bls12_381_fq12_serialize: InternalGasPerArg,
    pub ark_bls12_381_fq12_square: InternalGasPerArg,
    pub ark_bls12_381_fq12_sub: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_add: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_deser_comp_base: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_deser_comp_per_byte: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_deser_uncomp_base: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_deser_uncomp_per_byte: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_eq_proj: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_generator: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_infinity: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_scalar_mul_to_proj: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_neg: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_ser_comp: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_ser_uncomp: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_to_prepared: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_to_proj: InternalGasPerArg,
    pub ark_bls12_381_g1_proj_add: InternalGasPerArg,
    pub ark_bls12_381_g1_proj_double: InternalGasPerArg,
    pub ark_bls12_381_g1_proj_eq: InternalGasPerArg,
    pub ark_bls12_381_g1_proj_generator: InternalGasPerArg,
    pub ark_bls12_381_g1_proj_infinity: InternalGasPerArg,
    pub ark_bls12_381_g1_proj_neg: InternalGasPerArg,
    pub ark_bls12_381_g1_proj_scalar_mul: InternalGasPerArg,
    pub ark_bls12_381_g1_proj_sub: InternalGasPerArg,
    pub ark_bls12_381_g1_proj_to_affine: InternalGasPerArg,
    pub ark_bls12_381_g1_proj_to_prepared: InternalGasPerArg,
    pub ark_bls12_381_g2_affine_add: InternalGasPerArg,
    pub ark_bls12_381_g2_affine_deser_comp: InternalGasPerArg,
    pub ark_bls12_381_g2_affine_deser_uncomp: InternalGasPerArg,
    pub ark_bls12_381_g2_affine_eq: InternalGasPerArg,
    pub ark_bls12_381_g2_affine_generator: InternalGasPerArg,
    pub ark_bls12_381_g2_affine_infinity: InternalGasPerArg,
    pub ark_bls12_381_g2_affine_scalar_mul_to_proj: InternalGasPerArg,
    pub ark_bls12_381_g2_affine_neg: InternalGasPerArg,
    pub ark_bls12_381_g2_affine_ser_comp: InternalGasPerArg,
    pub ark_bls12_381_g2_affine_ser_uncomp: InternalGasPerArg,
    pub ark_bls12_381_g2_affine_to_prepared: InternalGasPerArg,
    pub ark_bls12_381_g2_affine_to_proj: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_add: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_double: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_eq: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_generator: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_infinity: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_neg: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_scalar_mul: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_sub: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_to_affine: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_to_prepared: InternalGasPerArg,
    pub ark_bls12_381_pairing_product_base: InternalGasPerArg,
    pub ark_bls12_381_pairing_product_per_pair: InternalGasPerArg,
}

fn to_size_in_u64(size_in_u8: usize) -> usize { (size_in_u8 + 7) / 8 }

impl GasParameters {
    pub fn field_add(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12_381_Fr => self.ark_bls12_381_fr_add * NumArgs::one(),
            Structure::BLS12_381_Fq12 => self.ark_bls12_381_fq12_add * NumArgs::one(),
            _ => unreachable!()
        }
    }

    pub fn field_inv(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12_381_Fr => self.ark_bls12_381_fr_inv * NumArgs::one(),
            Structure::BLS12_381_Fq12 => self.ark_bls12_381_fq12_inv * NumArgs::one(),
            _ => unreachable!()
        }
    }

    pub fn field_mul(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12_381_Fr => self.ark_bls12_381_fr_mul * NumArgs::one(),
            Structure::BLS12_381_Fq12 => self.ark_bls12_381_fq12_mul * NumArgs::one(),
            _ => unreachable!()
        }
    }

    pub fn field_sub(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12_381_Fr => self.ark_bls12_381_fr_sub * NumArgs::one(),
            Structure::BLS12_381_Fq12 => self.ark_bls12_381_fq12_sub * NumArgs::one(),
            _ => unreachable!()
        }
    }

    pub fn field_pow(&self, structure: Structure, exponent_size_in_bytes: usize) -> InternalGas {
        match structure {
            Structure::BLS12_381_Fr => {
                self.ark_bls12_381_fr_pow_base * NumArgs::one()
                    + self.ark_bls12_381_fr_pow_per_exponent_u64 * NumArgs::from(to_size_in_u64(exponent_size_in_bytes) as u64)
            },
            Structure::BLS12_381_Fq12 => {
                self.ark_bls12_381_fq12_pow_base * NumArgs::one()
                    + self.ark_bls12_381_fq12_pow_per_exponent_u64 * NumArgs::from(to_size_in_u64(exponent_size_in_bytes) as u64)
            },
            _ => unreachable!()
        }
    }

    pub fn field_div(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12_381_Fr => {
                self.ark_bls12_381_fr_div * NumArgs::one()
            },
            Structure::BLS12_381_Fq12 => {
                self.ark_bls12_381_fq12_div * NumArgs::one()
            },
            _ => unreachable!()
        }
    }
}
