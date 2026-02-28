// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Serde serialization for `bytes::Bytes` that is wire-compatible with `serde_bytes`.
//!
//! This module provides serialize/deserialize functions that produce the same
//! wire format as `serde_bytes` but work with `bytes::Bytes` instead of `Vec<u8>`.
//! This enables zero-copy message handling while maintaining backward compatibility.

use bytes::Bytes;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Serialize `Bytes` using the same format as `serde_bytes`.
pub fn serialize<S>(bytes: &Bytes, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serde_bytes::serialize(bytes.as_ref(), serializer)
}

/// Deserialize `Bytes` using the same format as `serde_bytes`.
pub fn deserialize<'de, D>(deserializer: D) -> Result<Bytes, D::Error>
where
    D: Deserializer<'de>,
{
    let vec: Vec<u8> = serde_bytes::deserialize(deserializer)?;
    Ok(Bytes::from(vec))
}

/// A wrapper type for `Bytes` that implements `Serialize` and `Deserialize`
/// using `serde_bytes` format. Useful for optional fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SerdeBytes(pub Bytes);

impl From<Bytes> for SerdeBytes {
    fn from(bytes: Bytes) -> Self {
        SerdeBytes(bytes)
    }
}

impl From<SerdeBytes> for Bytes {
    fn from(serde_bytes: SerdeBytes) -> Self {
        serde_bytes.0
    }
}

impl From<Vec<u8>> for SerdeBytes {
    fn from(vec: Vec<u8>) -> Self {
        SerdeBytes(Bytes::from(vec))
    }
}

impl AsRef<[u8]> for SerdeBytes {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl Serialize for SerdeBytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize(&self.0, serializer)
    }
}

impl<'de> Deserialize<'de> for SerdeBytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize(deserializer).map(SerdeBytes)
    }
}

#[cfg(any(test, feature = "fuzzing"))]
impl proptest::arbitrary::Arbitrary for SerdeBytes {
    type Parameters = proptest::arbitrary::ParamsFor<Vec<u8>>;
    type Strategy = proptest::strategy::MapInto<proptest::arbitrary::StrategyFor<Vec<u8>>, Self>;

    fn arbitrary_with(params: Self::Parameters) -> Self::Strategy {
        use proptest::strategy::Strategy;
        proptest::arbitrary::any_with::<Vec<u8>>(params).prop_map_into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wire_compatibility() {
        // Test that SerdeBytes produces the same wire format as Vec<u8> with serde_bytes
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct VecMessage {
            #[serde(with = "serde_bytes")]
            data: Vec<u8>,
        }

        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct BytesMessage {
            #[serde(with = "super")]
            data: Bytes,
        }

        let test_data = vec![1u8, 2, 3, 4, 5, 100, 200, 255];

        let vec_msg = VecMessage {
            data: test_data.clone(),
        };
        let bytes_msg = BytesMessage {
            data: Bytes::from(test_data.clone()),
        };

        // Serialize both and compare
        let vec_serialized = bcs::to_bytes(&vec_msg).unwrap();
        let bytes_serialized = bcs::to_bytes(&bytes_msg).unwrap();

        assert_eq!(
            vec_serialized, bytes_serialized,
            "Wire format must be identical"
        );

        // Cross-deserialize to verify compatibility
        let deserialized_from_vec: BytesMessage = bcs::from_bytes(&vec_serialized).unwrap();
        let deserialized_from_bytes: VecMessage = bcs::from_bytes(&bytes_serialized).unwrap();

        assert_eq!(deserialized_from_vec.data.as_ref(), test_data.as_slice());
        assert_eq!(deserialized_from_bytes.data, test_data);
    }

    #[test]
    fn test_empty_bytes() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct BytesMessage {
            #[serde(with = "super")]
            data: Bytes,
        }

        let msg = BytesMessage {
            data: Bytes::new(),
        };

        let serialized = bcs::to_bytes(&msg).unwrap();
        let deserialized: BytesMessage = bcs::from_bytes(&serialized).unwrap();

        assert_eq!(msg, deserialized);
        assert!(deserialized.data.is_empty());
    }

    #[test]
    fn test_large_bytes() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct BytesMessage {
            #[serde(with = "super")]
            data: Bytes,
        }

        let large_data: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();
        let msg = BytesMessage {
            data: Bytes::from(large_data.clone()),
        };

        let serialized = bcs::to_bytes(&msg).unwrap();
        let deserialized: BytesMessage = bcs::from_bytes(&serialized).unwrap();

        assert_eq!(msg, deserialized);
        assert_eq!(deserialized.data.as_ref(), large_data.as_slice());
    }
}
