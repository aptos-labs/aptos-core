// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Lazy wire type for BLS aggregate/multi-signatures.
//!
//! `LazyBlsSignature` carries the same on-wire encoding as
//! `aptos_crypto::bls12381::Signature` but skips the expensive G2-point
//! decompression at deserialization time. `bls12381::Signature`'s
//! `Deserialize` runs `blst::min_pk::Signature::from_bytes`, which decompresses
//! the 96-byte compressed G2 point (a field square root) on every element —
//! before any cheap structural check on the surrounding message can run.
//!
//! By storing the raw compressed bytes and deferring decompression until
//! [`LazyBlsSignature::decompress`] is called, callers can run cheap structural
//! gates (vector length, bitmask, voting power) first and only pay the
//! per-signature decompression cost once a message has cleared them. This
//! bounds the CPU work a peer-supplied payload can force on the receiver.
//!
//! ## Wire compatibility
//!
//! `bls12381::Signature` derives serde via `SerializeKey`/`DeserializeKey`,
//! which encode:
//!   - non-human-readable (e.g. BCS): `serialize_newtype_struct("Signature",
//!     serde_bytes::Bytes)` — i.e. a length-prefixed byte string named
//!     "Signature".
//!   - human-readable (e.g. JSON): `serialize_str("0x" + hex(bytes))`, decoded
//!     via `from_encoded_string` (which also tolerates an AIP-80 prefix).
//!
//! `LazyBlsSignature` replicates both branches exactly, emitting the same serde
//! data-model name ("Signature") so the encoding is byte-identical in every
//! format and the serde-reflection format corpus is unchanged. The
//! `lazy_bls_wire_compat_*` tests assert bitwise equality with
//! `bls12381::Signature` for both BCS and JSON.

use aptos_crypto::{bls12381, traits::ValidCryptoMaterial, CryptoMaterialError};
use serde::{
    de::{self, Deserializer},
    ser::Serializer,
    Deserialize, Serialize,
};

/// The serde data-model name used by `bls12381::Signature`'s `SerializeKey`
/// derive. Must match so the on-wire encoding (and traced format) is identical.
const SIGNATURE_NAME: &str = "Signature";

/// Compressed-bytes form of a `bls12381::Signature`. Wire-identical to
/// `bls12381::Signature`, but decoding does not decompress the G2 point.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LazyBlsSignature([u8; bls12381::Signature::LENGTH]);

impl LazyBlsSignature {
    /// Capture the compressed wire bytes of a known-valid signature.
    pub fn from_signature(sig: &bls12381::Signature) -> Self {
        Self(sig.to_bytes())
    }

    /// Subgroup-unchecked G2 decompression — the expensive operation we defer
    /// until a payload has cleared structural validation. (The subgroup check
    /// itself still happens later, inside signature verification.)
    pub fn decompress(&self) -> Result<bls12381::Signature, CryptoMaterialError> {
        bls12381::Signature::try_from(self.0.as_slice())
    }

    /// The raw 96-byte compressed encoding. Lets callers that only need the
    /// bytes (e.g. API hex export) avoid decompression entirely.
    pub fn to_bytes(&self) -> [u8; bls12381::Signature::LENGTH] {
        self.0
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn from_raw_bytes_for_test(bytes: [u8; bls12381::Signature::LENGTH]) -> Self {
        Self(bytes)
    }
}

impl Serialize for LazyBlsSignature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            // Mirror `to_encoded_string`: "0x" + hex(bytes).
            serializer.serialize_str(&format!("0x{}", hex::encode(self.0)))
        } else {
            // Mirror `SerializeKey`: a newtype struct named "Signature" wrapping
            // a serde_bytes byte string (length-prefixed in BCS).
            serializer.serialize_newtype_struct(
                SIGNATURE_NAME,
                serde_bytes::Bytes::new(self.0.as_slice()),
            )
        }
    }
}

impl<'de> Deserialize<'de> for LazyBlsSignature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes: Vec<u8> = if deserializer.is_human_readable() {
            // Mirror `from_encoded_string`: tolerate an AIP-80 prefix and/or a
            // leading "0x", then hex-decode.
            let encoded = <String>::deserialize(deserializer)?;
            let stripped = encoded
                .strip_prefix(bls12381::Signature::AIP_80_PREFIX)
                .unwrap_or(&encoded);
            let stripped = stripped.strip_prefix("0x").unwrap_or(stripped);
            hex::decode(stripped).map_err(de::Error::custom)?
        } else {
            // Mirror `DeserializeKey`: a newtype struct named "Signature"
            // wrapping a borrowed byte slice. Capture the bytes WITHOUT calling
            // `bls12381::Signature::try_from` (which would decompress).
            #[derive(Deserialize)]
            #[serde(rename = "Signature")]
            struct Value<'a>(&'a [u8]);

            let value = Value::deserialize(deserializer)?;
            value.0.to_vec()
        };

        let arr: [u8; bls12381::Signature::LENGTH] = bytes.try_into().map_err(|v: Vec<u8>| {
            de::Error::custom(format!(
                "invalid BLS signature length: {} (expected {})",
                v.len(),
                bls12381::Signature::LENGTH
            ))
        })?;
        Ok(Self(arr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_crypto::{bls12381::PrivateKey, test_utils::TestAptosCrypto, SigningKey, Uniform};

    fn sample_signature() -> bls12381::Signature {
        let mut rng = rand::thread_rng();
        let sk = PrivateKey::generate(&mut rng);
        sk.sign(&TestAptosCrypto("lazy_bls".to_string())).unwrap()
    }

    /// `LazyBlsSignature` must BCS-encode bitwise-identically to
    /// `bls12381::Signature` so validators on either type interoperate on the
    /// wire and on-disk blobs round-trip.
    #[test]
    fn lazy_bls_wire_compat_bcs() {
        for _ in 0..16 {
            let sig = sample_signature();
            let lazy = LazyBlsSignature::from_signature(&sig);

            let bytes_sig = bcs::to_bytes(&sig).unwrap();
            let bytes_lazy = bcs::to_bytes(&lazy).unwrap();
            assert_eq!(bytes_sig, bytes_lazy, "BCS encoding must match Signature");

            // Bytes produced by Signature decode as LazyBlsSignature.
            let decoded: LazyBlsSignature = bcs::from_bytes(&bytes_sig).unwrap();
            assert_eq!(decoded, lazy);

            // ...and bytes produced by LazyBlsSignature decode back to Signature.
            let round: bls12381::Signature = bcs::from_bytes(&bytes_lazy).unwrap();
            assert_eq!(round, sig);

            // Deferred decompression yields the original signature.
            assert_eq!(decoded.decompress().unwrap(), sig);
        }
    }

    /// Human-readable (JSON) encoding must also match bitwise.
    #[test]
    fn lazy_bls_wire_compat_json() {
        let sig = sample_signature();
        let lazy = LazyBlsSignature::from_signature(&sig);

        let json_sig = serde_json::to_string(&sig).unwrap();
        let json_lazy = serde_json::to_string(&lazy).unwrap();
        assert_eq!(json_sig, json_lazy, "JSON encoding must match Signature");

        let decoded: LazyBlsSignature = serde_json::from_str(&json_sig).unwrap();
        assert_eq!(decoded, lazy);
        assert_eq!(decoded.decompress().unwrap(), sig);
    }

    /// A wrong-length payload must be rejected at deserialization, not silently
    /// truncated/extended.
    #[test]
    fn rejects_wrong_length() {
        // Encode a byte string of the wrong length the same way Signature would
        // (newtype-struct-wrapped serde_bytes), then attempt to decode as lazy.
        let short = serde_bytes::ByteBuf::from(vec![0u8; 95]);
        let bytes = bcs::to_bytes(&short).unwrap();
        assert!(bcs::from_bytes::<LazyBlsSignature>(&bytes).is_err());
    }
}
