// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::natives::cryptography::algebra::{
    Structure, BLS12_381_FQ12_FORMAT, BLS12_381_FR_BENDIAN_FORMAT, BLS12_381_FR_FORMAT,
    BLS12_381_G1_COMPRESSED_FORMAT, BLS12_381_G1_UNCOMPRESSED_FORMAT,
    BLS12_381_G2_COMPRESSED_FORMAT, BLS12_381_G2_UNCOMPRESSED_FORMAT, BLS12_381_GT_FORMAT,
};
use move_core_types::gas_algebra::{InternalGas, InternalGasPerArg, NumArgs};

#[derive(Debug, Clone)]
pub struct GasParameters {
    pub ark_bls12_381_fr_add: InternalGasPerArg,
    pub ark_bls12_381_fr_deser: InternalGasPerArg,
    pub ark_bls12_381_fr_div: InternalGasPerArg,
    pub ark_bls12_381_fr_eq: InternalGasPerArg,
    pub ark_bls12_381_fr_from_u128: InternalGasPerArg,
    pub ark_bls12_381_fr_inv: InternalGasPerArg,
    pub ark_bls12_381_fr_is_one: InternalGasPerArg,
    pub ark_bls12_381_fr_is_zero: InternalGasPerArg,
    pub ark_bls12_381_fr_mul: InternalGasPerArg,
    pub ark_bls12_381_fr_neg: InternalGasPerArg,
    pub ark_bls12_381_fr_one: InternalGasPerArg,
    pub ark_bls12_381_fr_serialize: InternalGasPerArg,
    pub ark_bls12_381_fr_square: InternalGasPerArg,
    pub ark_bls12_381_fr_sub: InternalGasPerArg,
    pub ark_bls12_381_fr_to_repr: InternalGasPerArg,
    pub ark_bls12_381_fr_zero: InternalGasPerArg,
    pub ark_bls12_381_fq12_add: InternalGasPerArg,
    pub ark_bls12_381_fq12_clone: InternalGasPerArg,
    pub ark_bls12_381_fq12_deser: InternalGasPerArg,
    pub ark_bls12_381_fq12_div: InternalGasPerArg,
    pub ark_bls12_381_fq12_eq: InternalGasPerArg,
    pub ark_bls12_381_fq12_from_u128: InternalGasPerArg,
    pub ark_bls12_381_fq12_inv: InternalGasPerArg,
    pub ark_bls12_381_fq12_is_one: InternalGasPerArg,
    pub ark_bls12_381_fq12_is_zero: InternalGasPerArg,
    pub ark_bls12_381_fq12_mul: InternalGasPerArg,
    pub ark_bls12_381_fq12_neg: InternalGasPerArg,
    pub ark_bls12_381_fq12_one: InternalGasPerArg,
    pub ark_bls12_381_fq12_pow_u256: InternalGasPerArg,
    pub ark_bls12_381_fq12_serialize: InternalGasPerArg,
    pub ark_bls12_381_fq12_square: InternalGasPerArg,
    pub ark_bls12_381_fq12_sub: InternalGasPerArg,
    pub ark_bls12_381_fq12_zero: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_add: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_deser_comp: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_deser_uncomp: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_eq_proj: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_generator: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_infinity: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_scalar_mul_to_proj: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_msm_base: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_msm_per_entry: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_neg: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_serialize_comp: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_serialize_uncomp: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_to_prepared: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_to_proj: InternalGasPerArg,
    pub ark_bls12_381_g1_proj_add: InternalGasPerArg,
    pub ark_bls12_381_g1_proj_double: InternalGasPerArg,
    pub ark_bls12_381_g1_proj_eq: InternalGasPerArg,
    pub ark_bls12_381_g1_proj_generator: InternalGasPerArg,
    pub ark_bls12_381_g1_proj_infinity: InternalGasPerArg,
    pub ark_bls12_381_g1_proj_is_zero: InternalGasPerArg,
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
    pub ark_bls12_381_g2_affine_msm_base: InternalGasPerArg,
    pub ark_bls12_381_g2_affine_msm_per_entry: InternalGasPerArg,
    pub ark_bls12_381_g2_affine_neg: InternalGasPerArg,
    pub ark_bls12_381_g2_affine_scalar_mul_to_proj: InternalGasPerArg,
    pub ark_bls12_381_g2_affine_serialize_comp: InternalGasPerArg,
    pub ark_bls12_381_g2_affine_serialize_uncomp: InternalGasPerArg,
    pub ark_bls12_381_g2_affine_to_prepared: InternalGasPerArg,
    pub ark_bls12_381_g2_affine_to_proj: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_add: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_double: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_eq: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_generator: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_infinity: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_is_zero: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_neg: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_scalar_mul: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_sub: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_to_affine: InternalGasPerArg,
    pub ark_bls12_381_g2_proj_to_prepared: InternalGasPerArg,
    pub ark_bls12_381_pairing: InternalGasPerArg,
}

impl GasParameters {
    pub fn deserialize(&self, structure: Structure, scheme: &[u8]) -> InternalGas {
        match (structure, scheme) {
            (Structure::BLS12381Fr, sch) if sch == BLS12_381_FR_FORMAT.as_slice() => {
                self.ark_bls12_381_fr_deser * NumArgs::one()
            },
            (Structure::BLS12381Fr, sch) if sch == BLS12_381_FR_BENDIAN_FORMAT.as_slice() => {
                self.ark_bls12_381_fr_deser * NumArgs::one()
            },
            (Structure::BLS12381Fq12, sch) if sch == BLS12_381_FQ12_FORMAT.as_slice() => {
                self.ark_bls12_381_fq12_deser * NumArgs::one()
            },
            (Structure::BLS12381G1, sch) if sch == BLS12_381_G1_UNCOMPRESSED_FORMAT.as_slice() => {
                self.ark_bls12_381_g1_affine_deser_uncomp * NumArgs::one()
            },
            (Structure::BLS12381G1, sch) if sch == BLS12_381_G1_COMPRESSED_FORMAT.as_slice() => {
                self.ark_bls12_381_g1_affine_deser_comp * NumArgs::one()
            },
            (Structure::BLS12381G2, sch) if sch == BLS12_381_G2_UNCOMPRESSED_FORMAT.as_slice() => {
                self.ark_bls12_381_g2_affine_deser_uncomp * NumArgs::one()
            },
            (Structure::BLS12381G2, sch) if sch == BLS12_381_G2_COMPRESSED_FORMAT.as_slice() => {
                self.ark_bls12_381_g2_affine_deser_comp * NumArgs::one()
            },
            (Structure::BLS12381Gt, sch) if sch == BLS12_381_GT_FORMAT.as_slice() => {
                self.ark_bls12_381_fq12_deser * NumArgs::one()
            },
            (Structure::BLS12381Gt, sch) if sch == BLS12_381_GT_FORMAT.as_slice() => {
                self.ark_bls12_381_fq12_deser * NumArgs::one()
            },
            _ => unreachable!(),
        }
    }

    pub fn eq(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12381Fr => self.ark_bls12_381_fr_eq * NumArgs::one(),
            Structure::BLS12381Fq12 => self.ark_bls12_381_fq12_eq * NumArgs::one(),
            Structure::BLS12381G1 => self.ark_bls12_381_g1_proj_eq * NumArgs::one(),
            Structure::BLS12381G2 => self.ark_bls12_381_g2_proj_eq * NumArgs::one(),
            Structure::BLS12381Gt => self.ark_bls12_381_fq12_eq * NumArgs::one(),
        }
    }

    pub fn from_u128(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12381Fr => self.ark_bls12_381_fr_from_u128 * NumArgs::one(),
            Structure::BLS12381Fq12 => self.ark_bls12_381_fq12_from_u128 * NumArgs::one(),
            _ => unreachable!(),
        }
    }

    pub fn field_add(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12381Fr => self.ark_bls12_381_fr_add * NumArgs::one(),
            Structure::BLS12381Fq12 => self.ark_bls12_381_fq12_add * NumArgs::one(),
            _ => unreachable!(),
        }
    }

    pub fn field_div(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12381Fr => self.ark_bls12_381_fr_div * NumArgs::one(),
            Structure::BLS12381Fq12 => self.ark_bls12_381_fq12_div * NumArgs::one(),
            _ => unreachable!(),
        }
    }

    pub fn field_inv(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12381Fr => self.ark_bls12_381_fr_inv * NumArgs::one(),
            Structure::BLS12381Fq12 => self.ark_bls12_381_fq12_inv * NumArgs::one(),
            _ => unreachable!(),
        }
    }

    pub fn field_is_one(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12381Fr => self.ark_bls12_381_fr_is_one * NumArgs::one(),
            Structure::BLS12381Fq12 => self.ark_bls12_381_fq12_is_one * NumArgs::one(),
            _ => unreachable!(),
        }
    }

    pub fn field_is_zero(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12381Fr => self.ark_bls12_381_fr_is_zero * NumArgs::one(),
            Structure::BLS12381Fq12 => self.ark_bls12_381_fq12_is_zero * NumArgs::one(),
            _ => unreachable!(),
        }
    }

    pub fn field_mul(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12381Fr => self.ark_bls12_381_fr_mul * NumArgs::one(),
            Structure::BLS12381Fq12 => self.ark_bls12_381_fq12_mul * NumArgs::one(),
            _ => unreachable!(),
        }
    }

    pub fn field_one(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12381Fr => self.ark_bls12_381_fr_one * NumArgs::one(),
            Structure::BLS12381Fq12 => self.ark_bls12_381_fq12_one * NumArgs::one(),
            _ => unreachable!(),
        }
    }

    pub fn field_sqr(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12381Fr => self.ark_bls12_381_fr_square * NumArgs::one(),
            Structure::BLS12381Fq12 => self.ark_bls12_381_fq12_square * NumArgs::one(),
            _ => unreachable!(),
        }
    }

    pub fn field_sub(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12381Fr => self.ark_bls12_381_fr_sub * NumArgs::one(),
            Structure::BLS12381Fq12 => self.ark_bls12_381_fq12_sub * NumArgs::one(),
            _ => unreachable!(),
        }
    }

    pub fn field_zero(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12381Fr => self.ark_bls12_381_fr_zero * NumArgs::one(),
            Structure::BLS12381Fq12 => self.ark_bls12_381_fq12_zero * NumArgs::one(),
            _ => unreachable!(),
        }
    }

    pub fn field_neg(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12381Fr => self.ark_bls12_381_fr_neg * NumArgs::one(),
            Structure::BLS12381Fq12 => self.ark_bls12_381_fq12_neg * NumArgs::one(),
            _ => unreachable!(),
        }
    }

    pub fn group_add(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12381G1 => self.ark_bls12_381_g1_proj_add * NumArgs::one(),
            Structure::BLS12381G2 => self.ark_bls12_381_g2_proj_add * NumArgs::one(),
            Structure::BLS12381Gt => self.ark_bls12_381_fq12_mul * NumArgs::one(),
            _ => unreachable!(),
        }
    }

    pub fn group_double(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12381G1 => self.ark_bls12_381_g1_proj_double * NumArgs::one(),
            Structure::BLS12381G2 => self.ark_bls12_381_g2_proj_double * NumArgs::one(),
            Structure::BLS12381Gt => self.ark_bls12_381_fq12_square * NumArgs::one(),
            _ => unreachable!(),
        }
    }

    pub fn group_identity(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12381G1 => self.ark_bls12_381_g1_proj_infinity * NumArgs::one(),
            Structure::BLS12381G2 => self.ark_bls12_381_g2_proj_infinity * NumArgs::one(),
            Structure::BLS12381Gt => self.ark_bls12_381_fq12_one * NumArgs::one(),
            _ => unreachable!(),
        }
    }

    pub fn group_is_identity(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12381G1 => self.ark_bls12_381_g1_proj_is_zero * NumArgs::one(),
            Structure::BLS12381G2 => self.ark_bls12_381_g2_proj_is_zero * NumArgs::one(),
            Structure::BLS12381Gt => self.ark_bls12_381_fq12_is_one * NumArgs::one(),
            _ => unreachable!(),
        }
    }

    pub fn group_generator(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12381G1 => self.ark_bls12_381_g1_proj_generator * NumArgs::one(),
            Structure::BLS12381G2 => self.ark_bls12_381_g2_proj_generator * NumArgs::one(),
            Structure::BLS12381Gt => self.ark_bls12_381_fq12_clone * NumArgs::one(),
            _ => unreachable!(),
        }
    }

    pub fn group_multi_scalar_mul(&self, structure: Structure, num_entries: usize, scalar_bit_len: usize) -> InternalGas {
        match structure {
            Structure::BLS12381G1 => self.ark_bls12_381_g1_affine_msm_base * NumArgs::one() + self.ark_bls12_381_g1_affine_msm_per_entry * NumArgs::from(num_entries as u64),
            Structure::BLS12381G2 => self.ark_bls12_381_g2_affine_msm_base * NumArgs::one() + self.ark_bls12_381_g2_affine_msm_per_entry * NumArgs::from(num_entries as u64),
            Structure::BLS12381Gt => pippenger_gas(self.ark_bls12_381_fq12_mul, self.ark_bls12_381_fq12_square, num_entries, scalar_bit_len, pippenger_window_size(structure, num_entries)),
            _ => unreachable!(),
        }
    }

    pub fn group_multi_scalar_mul_typed(&self, structure: Structure, num_entries: usize) -> InternalGas {
        match structure {
            Structure::BLS12381G1 => self.ark_bls12_381_g1_affine_msm_base * NumArgs::one() + self.ark_bls12_381_g1_affine_msm_per_entry * NumArgs::from(num_entries as u64),
            Structure::BLS12381G2 => self.ark_bls12_381_g2_affine_msm_base * NumArgs::one() + self.ark_bls12_381_g2_affine_msm_per_entry * NumArgs::from(num_entries as u64),
            Structure::BLS12381Gt => pippenger_gas(self.ark_bls12_381_fq12_mul, self.ark_bls12_381_fq12_square, num_entries, 255, pippenger_window_size(structure, num_entries)),
            _ => unreachable!(),
        }
    }

    pub fn group_neg(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12381G1 => self.ark_bls12_381_g1_proj_neg * NumArgs::one(),
            Structure::BLS12381G2 => self.ark_bls12_381_g2_proj_neg * NumArgs::one(),
            Structure::BLS12381Gt => self.ark_bls12_381_fq12_inv * NumArgs::one(),
            _ => unreachable!(),
        }
    }

    pub fn group_scalar_mul(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12381G1 => self.ark_bls12_381_g1_proj_scalar_mul * NumArgs::one(),
            Structure::BLS12381G2 => self.ark_bls12_381_g2_proj_scalar_mul * NumArgs::one(),
            Structure::BLS12381Gt => self.ark_bls12_381_fq12_pow_u256 * NumArgs::one(),
            _ => unreachable!(),
        }
    }

    pub fn group_sub(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12381G1 => self.ark_bls12_381_g1_proj_sub * NumArgs::one(),
            Structure::BLS12381G2 => self.ark_bls12_381_g2_proj_sub * NumArgs::one(),
            Structure::BLS12381Gt => self.ark_bls12_381_fq12_div * NumArgs::one(),
            _ => unreachable!(),
        }
    }

    pub fn pairing(&self, g1: Structure, g2: Structure, g3: Structure) -> InternalGas {
        match (g1, g2, g3) {
            (Structure::BLS12381G1, Structure::BLS12381G2, Structure::BLS12381Gt) => {
                (self.ark_bls12_381_pairing) * NumArgs::one()
                    + self.ark_bls12_381_g1_proj_to_affine * NumArgs::one()
                    + self.ark_bls12_381_g2_proj_to_affine * NumArgs::one()
            },
            _ => unreachable!(),
        }
    }

    pub fn serialize(&self, structure: Structure, scheme: &[u8]) -> InternalGas {
        match (structure, scheme) {
            (Structure::BLS12381Fq12, _) => self.ark_bls12_381_fq12_serialize * NumArgs::one(),
            (Structure::BLS12381G1, s) if s == BLS12_381_G1_UNCOMPRESSED_FORMAT.as_slice() => {
                self.ark_bls12_381_g1_affine_serialize_uncomp * NumArgs::one()
            },
            (Structure::BLS12381G1, s) if s == BLS12_381_G1_COMPRESSED_FORMAT.as_slice() => {
                self.ark_bls12_381_g1_affine_serialize_comp * NumArgs::one()
            },
            (Structure::BLS12381G2, s) if s == BLS12_381_G2_UNCOMPRESSED_FORMAT.as_slice() => {
                self.ark_bls12_381_g2_affine_serialize_uncomp * NumArgs::one()
            },
            (Structure::BLS12381G2, s) if s == BLS12_381_G2_COMPRESSED_FORMAT.as_slice() => {
                self.ark_bls12_381_g2_affine_serialize_comp * NumArgs::one()
            },
            (Structure::BLS12381Gt, s) if s == BLS12_381_GT_FORMAT.as_slice() => {
                self.ark_bls12_381_fq12_serialize * NumArgs::one()
            },
            (Structure::BLS12381Fr, _) => self.ark_bls12_381_fr_serialize * NumArgs::one(),
            _ => unreachable!(),
        }
    }
}

fn pippenger_window_size(structure: Structure, num_entries: usize) -> usize {
    match structure {
        Structure::BLS12381Gt => 8, //TODO: precompute and hardcode.
        _ => unimplemented!()
    }
}

fn pippenger_gas(addition_gas: InternalGasPerArg, doubling_gas: InternalGasPerArg, num_entries: usize, scalar_bit_length: usize, window_size_in_bits: usize) -> InternalGas {
    let num_windows = (scalar_bit_length + window_size_in_bits - 1) / window_size_in_bits;
    return addition_gas * NumArgs::from((num_windows * (num_entries + (1<<num_windows) + 1)) as u64) + doubling_gas * NumArgs::from(scalar_bit_length as u64);
    // A\cdot w(n + 2^w + 1) + |s|D
}
