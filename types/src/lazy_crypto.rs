// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Deferred decoding for fixed-size crypto material using the SerializeKey wire format.

use aptos_crypto::{traits::ValidCryptoMaterial, CryptoMaterialError};
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::{fmt, marker::PhantomData, sync::OnceLock};

/// Opt-in adapter for material encoded as a named byte string by SerializeKey.
/// Implementations must match the original serde name and fixed-size encoding.
pub trait LazyCryptoMaterialType<const N: usize>: ValidCryptoMaterial + Clone {
    const SERDE_NAME: &'static str;

    fn fixed_bytes(&self) -> [u8; N];
}

/// Stores wire bytes without validating them until explicitly materialized.
/// Callers must authenticate the enclosing payload before trusting this material.
#[derive(Clone)]
pub struct LazyCryptoMaterial<T, const N: usize> {
    bytes: [u8; N],
    // Box the decoded value to keep untrusted, uncached instances small.
    decoded: OnceLock<Box<T>>,
}

impl<T: LazyCryptoMaterialType<N>, const N: usize> LazyCryptoMaterial<T, N> {
    pub fn from_material(material: &T) -> Self {
        Self {
            bytes: material.fixed_bytes(),
            decoded: OnceLock::from(Box::new(material.clone())),
        }
    }

    /// Decode and validate once on successful use. This does not replace the
    /// material's subsequent signature verification or subgroup checks.
    pub fn materialize(&self) -> Result<&T, CryptoMaterialError> {
        if self.decoded.get().is_none() {
            let value = T::try_from(self.bytes.as_slice())?;
            // Concurrent callers decode the same deterministic value.
            let _ = self.decoded.set(Box::new(value));
        }
        Ok(self.decoded.get().expect("valid material was cached above"))
    }

    pub fn to_bytes(&self) -> [u8; N] {
        self.bytes
    }

    #[cfg(test)]
    pub(crate) fn decoded_for_test(&self) -> Option<&T> {
        self.decoded.get().map(|value| value.as_ref())
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn from_raw_bytes_for_test(bytes: [u8; N]) -> Self {
        Self {
            bytes,
            decoded: OnceLock::new(),
        }
    }
}

// Identity depends only on the wire bytes, never the transient cache or T's Eq/Hash.
impl<T, const N: usize> PartialEq for LazyCryptoMaterial<T, N> {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}

impl<T, const N: usize> Eq for LazyCryptoMaterial<T, N> {}

impl<T, const N: usize> std::hash::Hash for LazyCryptoMaterial<T, N> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.bytes.hash(state);
    }
}

impl<T, const N: usize> fmt::Debug for LazyCryptoMaterial<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.bytes))
    }
}

impl<T: LazyCryptoMaterialType<N>, const N: usize> Serialize for LazyCryptoMaterial<T, N> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            serializer.serialize_str(&format!("0x{}", hex::encode(self.bytes)))
        } else {
            serializer.serialize_newtype_struct(
                T::SERDE_NAME,
                serde_bytes::Bytes::new(self.bytes.as_slice()),
            )
        }
    }
}

impl<'de, T: LazyCryptoMaterialType<N>, const N: usize> Deserialize<'de>
    for LazyCryptoMaterial<T, N>
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        fn from_slice<T, E: de::Error, const N: usize>(
            bytes: &[u8],
        ) -> Result<LazyCryptoMaterial<T, N>, E> {
            let bytes = <[u8; N]>::try_from(bytes).map_err(|_| {
                E::custom(format_args!(
                    "invalid crypto material length: {} (expected {N})",
                    bytes.len()
                ))
            })?;
            Ok(LazyCryptoMaterial {
                bytes,
                decoded: OnceLock::new(),
            })
        }

        if deserializer.is_human_readable() {
            let encoded = String::deserialize(deserializer)?;
            let stripped = encoded.strip_prefix(T::AIP_80_PREFIX).unwrap_or(&encoded);
            let stripped = stripped.strip_prefix("0x").unwrap_or(stripped);
            let bytes = hex::decode(stripped).map_err(de::Error::custom)?;
            from_slice(&bytes)
        } else {
            struct MaterialVisitor<T, const N: usize>(PhantomData<T>);

            impl<'de, T, const N: usize> Visitor<'de> for MaterialVisitor<T, N> {
                type Value = LazyCryptoMaterial<T, N>;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    write!(formatter, "a crypto material byte string of length {N}")
                }

                fn visit_newtype_struct<D: Deserializer<'de>>(
                    self,
                    deserializer: D,
                ) -> Result<Self::Value, D::Error> {
                    // Borrow and length-check before copying, so oversized fields
                    // cannot force an allocation proportional to their size.
                    let bytes = <&[u8]>::deserialize(deserializer)?;
                    from_slice(bytes)
                }
            }

            deserializer.deserialize_newtype_struct(T::SERDE_NAME, MaterialVisitor(PhantomData))
        }
    }
}
