// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements the Validate trait for BLS12-381 public keys, which enables library users
//! to make sure public keys used for verifying normal (non-aggregated) signatures lie in the prime-order
//! subgroup of the BLS12-381 group.
//!
//! NOTE: For public keys used to verify multisignatures, aggregate signatures and signature shares,
//! library users need NOT rely on this `Validatable<PublicKey>` wrapper and should instead verify
//! the proof-of-possession (PoP) of a public key, which implicitly guarantees the PK lies in the
//! prime-order subgroup. (See `bls12381_pop.rs` and `mod.rs` for details.)

use crate::{bls12381::PublicKey, validatable::Validate, CryptoMaterialError, ValidCryptoMaterial};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, hash::Hash};

/// An unvalidated `PublicKey`
#[derive(Debug, Clone, Eq)]
pub struct UnvalidatedPublicKey(pub(crate) [u8; PublicKey::LENGTH]);

impl UnvalidatedPublicKey {
    /// Return key as bytes
    pub fn to_bytes(&self) -> [u8; PublicKey::LENGTH] {
        self.0
    }
}

impl TryFrom<&[u8]> for UnvalidatedPublicKey {
    type Error = CryptoMaterialError;

    /// Deserializes an UnvalidatedPublicKey from a sequence of bytes.
    ///
    /// WARNING: Does NOT do any checks whatsoever on these bytes beyond checking the length.
    /// The returned `UnvalidatedPublicKey` can only be used to create a `Validatable::<PublicKey>`
    /// via `Validatable::<PublicKey>::from_unvalidated`.
    fn try_from(bytes: &[u8]) -> std::result::Result<Self, CryptoMaterialError> {
        if bytes.len() != PublicKey::LENGTH {
            Err(CryptoMaterialError::DeserializationError)
        } else {
            Ok(Self(<[u8; PublicKey::LENGTH]>::try_from(bytes).unwrap()))
        }
    }
}

impl ValidCryptoMaterial for UnvalidatedPublicKey {
    const AIP_80_PREFIX: &'static str = "bls12381-pub-";

    fn to_bytes(&self) -> Vec<u8> {
        self.0.to_vec()
    }
}

impl Serialize for UnvalidatedPublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            let encoded = ::hex::encode(self.0);
            serializer.serialize_str(&format!("0x{}", encoded))
        } else {
            // See comment in deserialize_key.
            serializer
                .serialize_newtype_struct("PublicKey", serde_bytes::Bytes::new(self.0.as_ref()))
        }
    }
}

impl<'de> Deserialize<'de> for UnvalidatedPublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        if deserializer.is_human_readable() {
            let encoded_key = <String>::deserialize(deserializer)?;
            let bytes_out = ::hex::decode(&encoded_key[2..]).map_err(D::Error::custom)?;
            <[u8; PublicKey::LENGTH]>::try_from(bytes_out.as_ref())
                .map(UnvalidatedPublicKey)
                .map_err(D::Error::custom)
        } else {
            // In order to preserve the Serde data model and help analysis tools,
            // make sure to wrap our value in a container with the same name
            // as the original type.
            #[derive(Deserialize)]
            #[serde(rename = "PublicKey")]
            struct Value<'a>(&'a [u8]);

            let value = Value::deserialize(deserializer)?;
            <[u8; PublicKey::LENGTH]>::try_from(value.0)
                .map(UnvalidatedPublicKey)
                .map_err(D::Error::custom)
        }
    }
}

impl Hash for UnvalidatedPublicKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write(&self.0)
    }
}

impl PartialEq for UnvalidatedPublicKey {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Validate for PublicKey {
    type Unvalidated = UnvalidatedPublicKey;

    fn validate(unvalidated: &Self::Unvalidated) -> Result<Self> {
        let pk = Self::try_from(unvalidated.0.as_ref())?;

        if pk.subgroup_check().is_err() {
            return Err(anyhow!("{:?}", CryptoMaterialError::SmallSubgroupError));
        }

        Ok(pk)
    }

    fn to_unvalidated(&self) -> Self::Unvalidated {
        UnvalidatedPublicKey(self.to_bytes())
    }
}

#[cfg(test)]
mod test {
    use crate::{
        bls12381::{PrivateKey, PublicKey, UnvalidatedPublicKey},
        test_utils::uniform_keypair_strategy,
        validatable::Validate,
    };
    use proptest::{prop_assert_eq, proptest};
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher},
    };

    proptest! {
        #[test]
        fn bls12381_validatable_pk(
            keypair in uniform_keypair_strategy::<PrivateKey, PublicKey>()
        ) {
            let valid = keypair.public_key;
            let unvalidated = valid.to_unvalidated();

            prop_assert_eq!(&unvalidated, &UnvalidatedPublicKey(valid.to_bytes()));
            prop_assert_eq!(&valid, &PublicKey::validate(&unvalidated).unwrap());

            // Ensure Serialize and Deserialize are implemented the same

            // BCS - A non-human-readable format
            {
                let serialized_valid = bcs::to_bytes(&valid).unwrap();
                let serialized_unvalidated = bcs::to_bytes(&unvalidated).unwrap();
                prop_assert_eq!(&serialized_valid, &serialized_unvalidated);

                let deserialized_valid_from_unvalidated: PublicKey = bcs::from_bytes(&serialized_unvalidated).unwrap();
                let deserialized_unvalidated_from_valid: UnvalidatedPublicKey = bcs::from_bytes(&serialized_valid).unwrap();

                prop_assert_eq!(&valid, &deserialized_valid_from_unvalidated);
                prop_assert_eq!(&unvalidated, &deserialized_unvalidated_from_valid);
            }

            // JSON A human-readable format
            {
                let serialized_valid = serde_json::to_string(&valid).unwrap();
                let serialized_unvalidated = serde_json::to_string(&unvalidated).unwrap();
                prop_assert_eq!(&serialized_valid, &serialized_unvalidated);

                let deserialized_valid_from_unvalidated: PublicKey = serde_json::from_str(&serialized_unvalidated).unwrap();
                let deserialized_unvalidated_from_valid: UnvalidatedPublicKey = serde_json::from_str(&serialized_valid).unwrap();

                prop_assert_eq!(&valid, &deserialized_valid_from_unvalidated);
                prop_assert_eq!(&unvalidated, &deserialized_unvalidated_from_valid);
            }


            // Ensure Hash is implemented the same
            let valid_hash = {
                let mut hasher = DefaultHasher::new();
                valid.hash(&mut hasher);
                hasher.finish()
            };

            let unvalidated_hash = {
                let mut hasher = DefaultHasher::new();
                unvalidated.hash(&mut hasher);
                hasher.finish()
            };

            prop_assert_eq!(valid_hash, unvalidated_hash);
        }
    }
}
