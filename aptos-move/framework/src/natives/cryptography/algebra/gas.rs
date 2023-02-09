// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use move_core_types::gas_algebra::{InternalGas, InternalGasPerArg, NumArgs};
use crate::natives::cryptography::algebra::{BLS12_381_FQ12_FORMAT, BLS12_381_FR_BENDIAN_FORMAT, BLS12_381_FR_FORMAT, BLS12_381_G1_COMPRESSED_FORMAT, BLS12_381_G1_UNCOMPRESSED_FORMAT, BLS12_381_G2_COMPRESSED_FORMAT, BLS12_381_G2_UNCOMPRESSED_FORMAT, BLS12_381_GT_FORMAT, Structure};

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
    pub ark_bls12_381_fr_deser: InternalGasPerArg,
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
    pub ark_bls12_381_fq12_deser: InternalGasPerArg,
    pub ark_bls12_381_fq12_div: InternalGasPerArg,
    pub ark_bls12_381_fq12_eq: InternalGasPerArg,
    pub ark_bls12_381_fq12_from_u128: InternalGasPerArg,
    pub ark_bls12_381_fq12_inv: InternalGasPerArg,
    pub ark_bls12_381_fq12_mul: InternalGasPerArg,
    pub ark_bls12_381_fq12_neg: InternalGasPerArg,
    pub ark_bls12_381_fq12_one: InternalGasPerArg,
    pub ark_bls12_381_fq12_pow_base: InternalGasPerArg,
    pub ark_bls12_381_fq12_pow_per_exponent_u64: InternalGasPerArg,
    pub ark_bls12_381_fq12_ser: InternalGasPerArg,
    pub ark_bls12_381_fq12_square: InternalGasPerArg,
    pub ark_bls12_381_fq12_sub: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_add: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_deser_comp: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_deser_uncomp: InternalGasPerArg,
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
    pub fn deserialize(&self, structure: Structure, scheme: &[u8]) -> InternalGas {
        match (structure, scheme) {
            (Structure::BLS12_381_Fr, sch) if sch == BLS12_381_FR_FORMAT.as_slice() =>  self.ark_bls12_381_fr_deser * NumArgs::one(),
            (Structure::BLS12_381_Fr, sch) if sch == BLS12_381_FR_BENDIAN_FORMAT.as_slice() =>  self.ark_bls12_381_fr_deser * NumArgs::one(),
            (Structure::BLS12_381_Fq12, sch) if sch == BLS12_381_FQ12_FORMAT.as_slice() => self.ark_bls12_381_fq12_deser * NumArgs::one(),
            (Structure::BLS12_381_G1, sch) if sch == BLS12_381_G1_UNCOMPRESSED_FORMAT.as_slice() => self.ark_bls12_381_g1_affine_deser_uncomp * NumArgs::one(),
            (Structure::BLS12_381_G1, sch) if sch == BLS12_381_G1_COMPRESSED_FORMAT.as_slice() => self.ark_bls12_381_g1_affine_deser_comp * NumArgs::one(),
            (Structure::BLS12_381_G2, sch) if sch == BLS12_381_G2_UNCOMPRESSED_FORMAT.as_slice() => self.ark_bls12_381_g2_affine_deser_uncomp * NumArgs::one(),
            (Structure::BLS12_381_G2, sch) if sch == BLS12_381_G2_COMPRESSED_FORMAT.as_slice() => self.ark_bls12_381_g2_affine_deser_comp * NumArgs::one(),
            (Structure::BLS12_381_Gt, sch) if sch == BLS12_381_GT_FORMAT.as_slice() => self.ark_bls12_381_fq12_deser * NumArgs::one(),
            (Structure::BLS12_381_Gt, sch) if sch == BLS12_381_GT_FORMAT.as_slice() => self.ark_bls12_381_fq12_deser * NumArgs::one(),
            _ => unreachable!()
        }
    }

    pub fn eq(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12_381_Fr =>  self.ark_bls12_381_fr_eq * NumArgs::one(),
            Structure::BLS12_381_Fq12 =>  self.ark_bls12_381_fq12_eq * NumArgs::one(),
            Structure::BLS12_381_G1 =>  self.ark_bls12_381_g1_proj_eq * NumArgs::one(),
            Structure::BLS12_381_G2 =>  self.ark_bls12_381_g2_proj_eq * NumArgs::one(),
            Structure::BLS12_381_Gt =>  self.ark_bls12_381_fq12_eq * NumArgs::one(),
            _ => unreachable!()
        }
    }

    pub fn from_u128(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12_381_Fr => self.ark_bls12_381_fr_from_u128 * NumArgs::one(),
            Structure::BLS12_381_Fq12 => self.ark_bls12_381_fq12_from_u128 * NumArgs::one(),
            _ => unreachable!()
        }
    }

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
            Structure::BLS12_381_Fr => self.ark_bls12_381_fr_div * NumArgs::one(),
            Structure::BLS12_381_Fq12 => self.ark_bls12_381_fq12_div * NumArgs::one(),
            _ => unreachable!()
        }
    }

    pub fn neg(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12_381_Fr => self.ark_bls12_381_fr_neg * NumArgs::one(),
            Structure::BLS12_381_Fq12 => self.ark_bls12_381_fq12_neg * NumArgs::one(),
            _ => unreachable!()
        }
    }

    pub fn group_add(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12_381_G1 => self.ark_bls12_381_g1_proj_add * NumArgs::one(),
            Structure::BLS12_381_G2 => self.ark_bls12_381_g2_proj_add * NumArgs::one(),
            _ => unreachable!()
        }
    }

    pub fn group_double(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12_381_G1 => self.ark_bls12_381_g1_proj_double * NumArgs::one(),
            Structure::BLS12_381_G2 => self.ark_bls12_381_g2_proj_double * NumArgs::one(),
            _ => unreachable!()
        }
    }

    pub fn group_identity(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12_381_G1 => self.ark_bls12_381_g1_proj_infinity * NumArgs::one(),
            Structure::BLS12_381_G2 => self.ark_bls12_381_g2_proj_infinity * NumArgs::one(),
            _ => unreachable!()
        }
    }

    pub fn group_generator(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12_381_G1 => self.ark_bls12_381_g1_proj_generator * NumArgs::one(),
            Structure::BLS12_381_G2 => self.ark_bls12_381_g2_proj_generator * NumArgs::one(),
            _ => unreachable!()
        }
    }

    pub fn group_neg(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12_381_G1 => self.ark_bls12_381_g1_proj_neg * NumArgs::one(),
            Structure::BLS12_381_G2 => self.ark_bls12_381_g2_proj_neg * NumArgs::one(),
            _ => unreachable!()
        }
    }

    pub fn group_scalar_mul(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12_381_G1 => self.ark_bls12_381_g1_proj_scalar_mul * NumArgs::one(),
            Structure::BLS12_381_G2 => self.ark_bls12_381_g2_proj_scalar_mul * NumArgs::one(),
            _ => unreachable!()
        }
    }

    pub fn serialize(&self, structure: Structure, scheme: &[u8]) -> InternalGas {
        match (structure, scheme) {
            (Structure::BLS12_381_Fq12, _) => self.ark_bls12_381_fq12_ser * NumArgs::one(),
            (Structure::BLS12_381_G1, s) if s == BLS12_381_G1_UNCOMPRESSED_FORMAT.as_slice() => self.ark_bls12_381_g1_affine_ser_uncomp * NumArgs::one(),
            (Structure::BLS12_381_G1, s) if s == BLS12_381_G1_COMPRESSED_FORMAT.as_slice() => self.ark_bls12_381_g1_affine_ser_comp * NumArgs::one(),
            (Structure::BLS12_381_G2, s) if s == BLS12_381_G2_UNCOMPRESSED_FORMAT.as_slice() => self.ark_bls12_381_g2_affine_ser_uncomp * NumArgs::one(),
            (Structure::BLS12_381_G2, s) if s == BLS12_381_G2_COMPRESSED_FORMAT.as_slice() => self.ark_bls12_381_g2_affine_ser_comp * NumArgs::one(),
            (Structure::BLS12_381_Gt, s) if s == BLS12_381_GT_FORMAT.as_slice() => self.ark_bls12_381_fq12_ser * NumArgs::one(),
            (Structure::BLS12_381_Fr, _) => self.ark_bls12_381_fr_ser * NumArgs::one(),
            _ => unreachable!()
        }
    }
}
