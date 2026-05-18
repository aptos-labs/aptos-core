// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
//
// Vendored from aptos-core/types/src/keyless/mod.rs @ rev 8ec3fb76, lines 215-360
// (Pepper, IdCommitment, KeylessPublicKey). Trait derives that pull in
// aptos-internal traits (CryptoHash, BCSCryptoHash, Move types) have been
// removed; BCS wire layout is preserved.

use crate::errors::VerifyError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// 31-byte pepper used to derive a hiding identity commitment (IDC) when computing
/// a keyless account address. Length is fixed to
/// `poseidon_bn254::BYTES_PACKED_PER_SCALAR`.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Pepper(pub(crate) [u8; Pepper::NUM_BYTES]);

impl Pepper {
    /// 31 bytes — one BN254 scalar packs at most 31 bytes.
    pub const NUM_BYTES: usize = 31;

    pub fn new(bytes: [u8; Self::NUM_BYTES]) -> Self {
        Self(bytes)
    }

    pub fn to_bytes(&self) -> &[u8; Self::NUM_BYTES] {
        &self.0
    }

    pub fn from_number(num: u128) -> Self {
        let big_int = num_bigint::BigUint::from(num);
        let bytes: Vec<u8> = big_int.to_bytes_le();
        let mut extended = [0u8; Self::NUM_BYTES];
        extended[..bytes.len()].copy_from_slice(&bytes);
        Self(extended)
    }
}

// Matches aptos-types' custom serde (hex string in human-readable, raw bytes
// in BCS) so the wire format is identical.
impl<'de> Deserialize<'de> for Pepper {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        if d.is_human_readable() {
            let s = <String>::deserialize(d)?;
            let bytes = hex::decode(s)
                .map_err(serde::de::Error::custom)?
                .try_into()
                .map_err(|e| serde::de::Error::custom(format!("{:?}", e)))?;
            Ok(Pepper::new(bytes))
        } else {
            #[derive(Deserialize)]
            #[serde(rename = "Pepper")]
            struct Value([u8; Pepper::NUM_BYTES]);
            let v = Value::deserialize(d)?;
            Ok(Pepper::new(v.0))
        }
    }
}

impl Serialize for Pepper {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if s.is_human_readable() {
            hex::encode(self.0).serialize(s)
        } else {
            s.serialize_newtype_struct("Pepper", &self.0)
        }
    }
}

/// 32-byte hiding commitment to (aud, uid_key, uid_val, pepper).
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct IdCommitment(#[serde(with = "serde_bytes")] pub Vec<u8>);

impl IdCommitment {
    pub const NUM_BYTES: usize = 32;

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// The on-chain public key identifying a keyless account (the `iss` URL plus a
/// 32-byte hiding commitment to the user's OIDC identity).
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct KeylessPublicKey {
    /// The `iss` claim value of the JWT used to authenticate this account.
    pub iss_val: String,

    /// A 32-byte commitment hiding (aud, uid_key, uid_val, pepper).
    pub idc: IdCommitment,
}

impl KeylessPublicKey {
    /// Parse from BCS bytes (the wire encoding used by `aptos-types`).
    pub fn from_bcs_bytes(bytes: &[u8]) -> Result<Self, VerifyError> {
        bcs::from_bytes::<KeylessPublicKey>(bytes)
            .map_err(|e| VerifyError::Decode(format!("KeylessPublicKey: {}", e)))
    }

    pub fn iss(&self) -> &str {
        &self.iss_val
    }

    /// SHA3-256(uleb128(AnyPublicKey::Keyless_variant_tag) || BCS(self) ||
    /// `SingleKey` scheme byte) — the 32-byte account authentication key when
    /// this `KeylessPublicKey` is used as an account public key (i.e. wrapped
    /// as `AnyPublicKey::Keyless` in a `SingleKey` authenticator).
    ///
    /// TODO(impl): implement once `AnyPublicKey` variant tag constants land.
    pub fn account_authentication_key(&self) -> [u8; 32] {
        unimplemented!("see follow-up commit on this branch")
    }
}
