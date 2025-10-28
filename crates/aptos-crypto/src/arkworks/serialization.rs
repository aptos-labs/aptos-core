// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Copied from https://github.com/arkworks-rs/algebra/issues/178#issuecomment-1413219278

use ark_serialize::{CanonicalDeserialize, CanonicalSerialize, Compress, Validate};

/// Serializes a type implementing `CanonicalSerialize` into bytes using the
/// [`ark_serialize`](https://docs.rs/ark-serialize) format and writes it to a Serde serializer.
///
/// This is useful for integrating Arkworks types (e.g., elliptic curve elements, field elements)
/// with Serde-compatible formats such as JSON, CBOR, or MessagePack.
pub fn ark_se<S, A: CanonicalSerialize>(a: &A, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let mut bytes = vec![];
    a.serialize_with_mode(&mut bytes, Compress::No)
        .map_err(serde::ser::Error::custom)?;
    s.serialize_bytes(&bytes)
}

/// Deserializes a type implementing `CanonicalDeserialize` from bytes produced by [`ark_se`].
///
/// This function allows Arkworks types to be deserialized from Serde-compatible data sources.
/// It assumes the data was serialized without compression and without validation. // !!!!!!!!!!!!!!!!!!!
pub fn ark_de<'de, D, A: CanonicalDeserialize>(data: D) -> Result<A, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let s: Vec<u8> = serde::de::Deserialize::deserialize(data)?;
    let a = A::deserialize_with_mode(s.as_slice(), Compress::No, Validate::No);
    a.map_err(serde::de::Error::custom)
}
