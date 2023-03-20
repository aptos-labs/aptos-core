// Copyright © Aptos Foundation

// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::natives::cryptography::algebra::{HashToStructureSuite, SerializationFormat, Structure};
use move_core_types::gas_algebra::{InternalGas, InternalGasPerArg, NumArgs};

fn log2_floor(n: usize) -> usize {
    (0_usize.leading_zeros() - n.leading_zeros()) as usize
}

fn log2_ceil(n: usize) -> usize {
    log2_floor(n-1)+1
}

fn ark_msm_window_size(num_entries: usize) -> usize {
    if num_entries < 32 {
        3
    } else {
        (log2_ceil(num_entries) * 69 / 100) + 2
    }
}

fn ark_msm_window_size_2(num_entries: usize) -> usize {
    if num_entries < 32 {
        3
    } else {
        (ark_std::log2(num_entries) * 69 / 100) as usize + 2
    }
}
//
#[test]
fn window_size() {
    for x in 0..100 {
        let w1 = ark_msm_window_size(x);
        let w2 = ark_msm_window_size_2(x);
        println!("x={x}, w1={w1}, w2={w2}");
    }
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
    pub ark_bls12_381_g1_affine_msm_base: InternalGasPerArg,
    pub ark_bls12_381_g1_affine_msm_per_entry: InternalGasPerArg,
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
    pub ark_bls12_381_g2_affine_msm_base: InternalGasPerArg,
    pub ark_bls12_381_g2_affine_msm_per_entry: InternalGasPerArg,
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
    pub fn deserialize(&self, structure: Structure, format: SerializationFormat) -> InternalGas {
        match (structure, format) {
            (Structure::BLS12381Fr, SerializationFormat::BLS12381FrLsb)
            | (Structure::BLS12381Fr, SerializationFormat::BLS12381FrMsb) => {
                self.ark_bls12_381_fr_deser * NumArgs::one()
            },
            (Structure::BLS12381Fq12, SerializationFormat::BLS12381Fq12LscLsb) => {
                self.ark_bls12_381_fq12_deser * NumArgs::one()
            },
            (Structure::BLS12381G1Affine, SerializationFormat::BLS12381G1AffineUncompressed) => {
                self.ark_bls12_381_g1_affine_deser_uncomp * NumArgs::one()
            },
            (Structure::BLS12381G1Affine, SerializationFormat::BLS12381G1AffineCompressed) => {
                self.ark_bls12_381_g1_affine_deser_comp * NumArgs::one()
            },
            (Structure::BLS12381G2Affine, SerializationFormat::BLS12381G2AffineUncompressed) => {
                self.ark_bls12_381_g2_affine_deser_uncomp * NumArgs::one()
            },
            (Structure::BLS12381G2Affine, SerializationFormat::BLS12381G2AffineCompressed) => {
                self.ark_bls12_381_g2_affine_deser_comp * NumArgs::one()
            },
            (Structure::BLS12381Gt, SerializationFormat::BLS12381Gt) => {
                self.ark_bls12_381_fq12_deser * NumArgs::one()
            },
            _ => unreachable!(),
        }
    }

    pub fn eq(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12381Fr => self.ark_bls12_381_fr_eq * NumArgs::one(),
            Structure::BLS12381Fq12 => self.ark_bls12_381_fq12_eq * NumArgs::one(),
            Structure::BLS12381G1Affine => self.ark_bls12_381_g1_proj_eq * NumArgs::one(),
            Structure::BLS12381G2Affine => self.ark_bls12_381_g2_proj_eq * NumArgs::one(),
            Structure::BLS12381Gt => self.ark_bls12_381_fq12_eq * NumArgs::one(),
        }
    }

    pub fn from_u64(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12381Fr => self.ark_bls12_381_fr_from_u64 * NumArgs::one(),
            Structure::BLS12381Fq12 => self.ark_bls12_381_fq12_from_u64 * NumArgs::one(),
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
            Structure::BLS12381G1Affine => self.ark_bls12_381_g1_proj_add * NumArgs::one(),
            Structure::BLS12381G2Affine => self.ark_bls12_381_g2_proj_add * NumArgs::one(),
            Structure::BLS12381Gt => self.ark_bls12_381_fq12_mul * NumArgs::one(),
            _ => unreachable!(),
        }
    }

    pub fn group_double(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12381G1Affine => self.ark_bls12_381_g1_proj_double * NumArgs::one(),
            Structure::BLS12381G2Affine => self.ark_bls12_381_g2_proj_double * NumArgs::one(),
            Structure::BLS12381Gt => self.ark_bls12_381_fq12_square * NumArgs::one(),
            _ => unreachable!(),
        }
    }

    pub fn group_identity(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12381G1Affine => self.ark_bls12_381_g1_proj_infinity * NumArgs::one(),
            Structure::BLS12381G2Affine => self.ark_bls12_381_g2_proj_infinity * NumArgs::one(),
            Structure::BLS12381Gt => self.ark_bls12_381_fq12_one * NumArgs::one(),
            _ => unreachable!(),
        }
    }

    pub fn group_generator(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12381G1Affine => self.ark_bls12_381_g1_proj_generator * NumArgs::one(),
            Structure::BLS12381G2Affine => self.ark_bls12_381_g2_proj_generator * NumArgs::one(),
            Structure::BLS12381Gt => self.ark_bls12_381_fq12_clone * NumArgs::one(),
            _ => unreachable!(),
        }
    }

    fn ark_msm_window_size(&self, num_entries: usize) -> usize {
        if num_entries < 32 {
            3
        } else {
            (log2_ceil(num_entries) * 69 / 100) + 2
        }
    }

    pub fn group_multi_scalar_mul(&self, structure: Structure, num_entries: usize) -> InternalGas {
        match structure {
            Structure::BLS12381G1Affine => {
                let window_size = self.ark_msm_window_size(num_entries);
                let num_windows = (255 + window_size - 1) / window_size;
                let num_buckets = 1_usize << window_size;
                self.ark_bls12_381_g1_proj_add * NumArgs::from(((num_entries + num_buckets + 1) * num_windows) as u64)
                    + self.ark_bls12_381_g1_proj_double * NumArgs::from((num_buckets * num_windows) as u64)
            },
            Structure::BLS12381G2Affine => {
                let window_size = self.ark_msm_window_size(num_entries);
                let num_windows = (255 + window_size - 1) / window_size;
                let num_buckets = 1_usize << window_size;
                self.ark_bls12_381_g2_proj_add * NumArgs::from(((num_entries + num_buckets + 1) * num_windows) as u64)
                    + self.ark_bls12_381_g2_proj_double * NumArgs::from((num_buckets * num_windows) as u64)
            },
            _ => unreachable!(),
        }
    }

    pub fn group_neg(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12381G1Affine => self.ark_bls12_381_g1_proj_neg * NumArgs::one(),
            Structure::BLS12381G2Affine => self.ark_bls12_381_g2_proj_neg * NumArgs::one(),
            Structure::BLS12381Gt => self.ark_bls12_381_fq12_inv * NumArgs::one(),
            _ => unreachable!(),
        }
    }

    pub fn group_scalar_mul(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12381G1Affine => self.ark_bls12_381_g1_proj_scalar_mul * NumArgs::one(),
            Structure::BLS12381G2Affine => self.ark_bls12_381_g2_proj_scalar_mul * NumArgs::one(),
            Structure::BLS12381Gt => self.ark_bls12_381_fq12_pow_u256 * NumArgs::one(),
            _ => unreachable!(),
        }
    }

    pub fn group_sub(&self, structure: Structure) -> InternalGas {
        match structure {
            Structure::BLS12381G1Affine => self.ark_bls12_381_g1_proj_sub * NumArgs::one(),
            Structure::BLS12381G2Affine => self.ark_bls12_381_g2_proj_sub * NumArgs::one(),
            Structure::BLS12381Gt => self.ark_bls12_381_fq12_div * NumArgs::one(),
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

    pub fn serialize(&self, structure: Structure, format: SerializationFormat) -> InternalGas {
        match (structure, format) {
            (Structure::BLS12381Fq12, SerializationFormat::BLS12381Fq12LscLsb) => {
                self.ark_bls12_381_fq12_serialize * NumArgs::one()
            },
            (Structure::BLS12381G1Affine, SerializationFormat::BLS12381G1AffineUncompressed) => {
                self.ark_bls12_381_g1_affine_serialize_uncomp * NumArgs::one()
            },
            (Structure::BLS12381G1Affine, SerializationFormat::BLS12381G1AffineCompressed) => {
                self.ark_bls12_381_g1_affine_serialize_comp * NumArgs::one()
            },
            (Structure::BLS12381G2Affine, SerializationFormat::BLS12381G2AffineUncompressed) => {
                self.ark_bls12_381_g2_affine_serialize_uncomp * NumArgs::one()
            },
            (Structure::BLS12381G2Affine, SerializationFormat::BLS12381G2AffineCompressed) => {
                self.ark_bls12_381_g2_affine_serialize_comp * NumArgs::one()
            },
            (Structure::BLS12381Gt, SerializationFormat::BLS12381Gt) => {
                self.ark_bls12_381_fq12_serialize * NumArgs::one()
            },
            (Structure::BLS12381Fr, SerializationFormat::BLS12381FrLsb)
            | (Structure::BLS12381Fr, SerializationFormat::BLS12381FrMsb) => {
                self.ark_bls12_381_fr_serialize * NumArgs::one()
            },
            _ => unreachable!(),
        }
    }
}
