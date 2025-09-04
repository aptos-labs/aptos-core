// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    keyless::KEYLESS_ACCOUNT_MODULE_NAME, move_utils::as_move_value::AsMoveValue, serialize,
};
use velor_crypto::CryptoMaterialError;
use velor_crypto_derive::{BCSCryptoHash, CryptoHasher};
use ark_bn254::{Bn254, G1Affine, G2Affine};
use ark_groth16::{PreparedVerifyingKey, VerifyingKey};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use move_core_types::{
    ident_str,
    identifier::IdentStr,
    move_resource::MoveStructType,
    value::{MoveStruct, MoveValue},
};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// Reflection of velor_framework::keyless_account::Groth16VerificationKey
#[derive(Clone, Serialize, Deserialize, Eq, PartialEq, Debug, BCSCryptoHash, CryptoHasher)]
pub struct Groth16VerificationKey {
    pub alpha_g1: Vec<u8>,
    pub beta_g2: Vec<u8>,
    pub gamma_g2: Vec<u8>,
    pub delta_g2: Vec<u8>,
    pub gamma_abc_g1: Vec<Vec<u8>>,
}

impl AsMoveValue for Groth16VerificationKey {
    fn as_move_value(&self) -> MoveValue {
        MoveValue::Struct(MoveStruct::Runtime(vec![
            self.alpha_g1.as_move_value(),
            self.beta_g2.as_move_value(),
            self.gamma_g2.as_move_value(),
            self.delta_g2.as_move_value(),
            self.gamma_abc_g1.as_move_value(),
        ]))
    }
}

/// WARNING: This struct uses resource groups on the Move side. Do NOT implement OnChainConfig
/// for it, since `OnChainConfig::fetch_config` does not work with resource groups (yet).
impl MoveStructType for Groth16VerificationKey {
    const MODULE_NAME: &'static IdentStr = ident_str!(KEYLESS_ACCOUNT_MODULE_NAME);
    const STRUCT_NAME: &'static IdentStr = ident_str!("Groth16VerificationKey");
}

impl TryFrom<Groth16VerificationKey> for PreparedVerifyingKey<Bn254> {
    type Error = CryptoMaterialError;

    fn try_from(vk: Groth16VerificationKey) -> Result<Self, Self::Error> {
        (&vk).try_into()
    }
}

impl TryFrom<&Groth16VerificationKey> for PreparedVerifyingKey<Bn254> {
    type Error = CryptoMaterialError;

    fn try_from(vk: &Groth16VerificationKey) -> Result<Self, Self::Error> {
        if vk.gamma_abc_g1.len() != 2 {
            return Err(CryptoMaterialError::DeserializationError);
        }

        // NOTE: Technically, we already validate the points when we set the VK in Move, so we could
        // make this 2x faster by avoiding the point validation checks  via
        // `deserialize_with_mode(..., Compress::Yes, Validate::No)`. Due to paranoia, will not
        // optimize this for now.
        Ok(Self::from(VerifyingKey {
            alpha_g1: G1Affine::deserialize_compressed(vk.alpha_g1.as_slice())
                .map_err(|_| CryptoMaterialError::DeserializationError)?,
            beta_g2: G2Affine::deserialize_compressed(vk.beta_g2.as_slice())
                .map_err(|_| CryptoMaterialError::DeserializationError)?,
            gamma_g2: G2Affine::deserialize_compressed(vk.gamma_g2.as_slice())
                .map_err(|_| CryptoMaterialError::DeserializationError)?,
            delta_g2: G2Affine::deserialize_compressed(vk.delta_g2.as_slice())
                .map_err(|_| CryptoMaterialError::DeserializationError)?,
            gamma_abc_g1: vec![
                G1Affine::deserialize_compressed(vk.gamma_abc_g1[0].as_slice())
                    .map_err(|_| CryptoMaterialError::DeserializationError)?,
                G1Affine::deserialize_compressed(vk.gamma_abc_g1[1].as_slice())
                    .map_err(|_| CryptoMaterialError::DeserializationError)?,
            ],
        }))
    }
}

impl From<PreparedVerifyingKey<Bn254>> for Groth16VerificationKey {
    fn from(pvk: PreparedVerifyingKey<Bn254>) -> Self {
        (&pvk).into()
    }
}

impl From<&PreparedVerifyingKey<Bn254>> for Groth16VerificationKey {
    fn from(pvk: &PreparedVerifyingKey<Bn254>) -> Self {
        let PreparedVerifyingKey {
            vk:
                VerifyingKey {
                    alpha_g1,
                    beta_g2,
                    gamma_g2,
                    delta_g2,
                    gamma_abc_g1,
                },
            alpha_g1_beta_g2: _alpha_g1_beta_g2, // unnecessary for Move
            gamma_g2_neg_pc: _gamma_g2_neg_pc,   // unnecessary for Move
            delta_g2_neg_pc: _delta_g2_neg_pc,   // unnecessary for Move
        } = pvk;

        let mut gamma_abc_g1_bytes = Vec::with_capacity(gamma_abc_g1.len());
        for e in gamma_abc_g1.iter() {
            gamma_abc_g1_bytes.push(serialize!(e));
        }

        Groth16VerificationKey {
            alpha_g1: serialize!(alpha_g1),
            beta_g2: serialize!(beta_g2),
            gamma_g2: serialize!(gamma_g2),
            delta_g2: serialize!(delta_g2),
            gamma_abc_g1: gamma_abc_g1_bytes,
        }
    }
}

impl PartialEq<PreparedVerifyingKey<Bn254>> for Groth16VerificationKey {
    fn eq(&self, other: &PreparedVerifyingKey<Bn254>) -> bool {
        let other_vk: Groth16VerificationKey = other.into();
        self == &other_vk
    }
}

impl Display for Groth16VerificationKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "alpha_g1: {}", hex::encode(&self.alpha_g1))?;
        write!(f, "beta_g2: {}", hex::encode(&self.beta_g2))?;
        write!(f, "gamma_g2: {}", hex::encode(&self.gamma_g2))?;
        write!(f, "delta_g2: {}", hex::encode(&self.delta_g2))?;
        for (i, e) in self.gamma_abc_g1.iter().enumerate() {
            write!(f, "gamma_abc_g1[{i}]: {}", hex::encode(serialize!(e)))?;
        }
        Ok(())
    }
}
