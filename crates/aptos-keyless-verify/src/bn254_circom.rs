// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
//
// Vendored from aptos-core/types/src/keyless/bn254_circom.rs @ rev 8ec3fb76.
// Manual serde impls are reproduced verbatim so BCS wire format matches.

use crate::errors::VerifyError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub const G1_PROJECTIVE_COMPRESSED_NUM_BYTES: usize = 32;
pub const G2_PROJECTIVE_COMPRESSED_NUM_BYTES: usize = 64;

/// Compressed G1 point on BN254 in the encoding used by Circom-generated proofs.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct G1Bytes(pub(crate) [u8; G1_PROJECTIVE_COMPRESSED_NUM_BYTES]);

impl G1Bytes {
    pub fn new(bytes: [u8; G1_PROJECTIVE_COMPRESSED_NUM_BYTES]) -> Self {
        Self(bytes)
    }

    pub fn new_from_vec(v: Vec<u8>) -> Result<Self, VerifyError> {
        v.try_into()
            .map(Self::new)
            .map_err(|_| VerifyError::Decode("G1Bytes: expected 32 bytes".into()))
    }

    pub fn as_bytes(&self) -> &[u8; G1_PROJECTIVE_COMPRESSED_NUM_BYTES] {
        &self.0
    }
}

/// Compressed G2 point on BN254 in the encoding used by Circom-generated proofs.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct G2Bytes(pub(crate) [u8; G2_PROJECTIVE_COMPRESSED_NUM_BYTES]);

impl G2Bytes {
    pub fn new(bytes: [u8; G2_PROJECTIVE_COMPRESSED_NUM_BYTES]) -> Self {
        Self(bytes)
    }

    pub fn new_from_vec(v: Vec<u8>) -> Result<Self, VerifyError> {
        v.try_into()
            .map(Self::new)
            .map_err(|_| VerifyError::Decode("G2Bytes: expected 64 bytes".into()))
    }

    pub fn as_bytes(&self) -> &[u8; G2_PROJECTIVE_COMPRESSED_NUM_BYTES] {
        &self.0
    }
}

// ── serde impls (verbatim from upstream so BCS wire format matches) ──────────

impl<'de> Deserialize<'de> for G1Bytes {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        if d.is_human_readable() {
            let s = <String>::deserialize(d)?;
            let bytes = hex::decode(s).map_err(serde::de::Error::custom)?;
            G1Bytes::new_from_vec(bytes).map_err(serde::de::Error::custom)
        } else {
            #[derive(Deserialize)]
            #[serde(rename = "G1Bytes")]
            struct Value([u8; G1_PROJECTIVE_COMPRESSED_NUM_BYTES]);
            let v = Value::deserialize(d)?;
            Ok(G1Bytes(v.0))
        }
    }
}

impl Serialize for G1Bytes {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if s.is_human_readable() {
            hex::encode(self.0).serialize(s)
        } else {
            s.serialize_newtype_struct("G1Bytes", &self.0)
        }
    }
}

impl<'de> Deserialize<'de> for G2Bytes {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        if d.is_human_readable() {
            let s = <String>::deserialize(d)?;
            let bytes = hex::decode(s).map_err(serde::de::Error::custom)?;
            G2Bytes::new_from_vec(bytes).map_err(serde::de::Error::custom)
        } else {
            // serde 1.0.143+ derives Deserialize for [u8; N] via const generics.
            #[derive(Deserialize)]
            #[serde(rename = "G2Bytes")]
            struct Value(serde_big_array::Array<u8, G2_PROJECTIVE_COMPRESSED_NUM_BYTES>);
            let v = Value::deserialize(d)?;
            Ok(G2Bytes(v.0.0))
        }
    }
}

impl Serialize for G2Bytes {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if s.is_human_readable() {
            hex::encode(self.0).serialize(s)
        } else {
            s.serialize_newtype_struct("G2Bytes", &serde_big_array::Array(self.0))
        }
    }
}

// TODO(impl): port `g1_projective_str_to_affine`, `g2_projective_str_to_affine`,
// `to_projective_point`, and `get_public_inputs_hash` from the upstream file
// in a follow-up commit on this branch.
