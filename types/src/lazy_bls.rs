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
use std::{fmt, sync::OnceLock};

/// The serde data-model name used by `bls12381::Signature`'s `SerializeKey`
/// derive. Must match so the on-wire encoding (and traced format) is identical.
const SIGNATURE_NAME: &str = "Signature";

/// Compressed-bytes form of a `bls12381::Signature`. Wire-identical to
/// `bls12381::Signature`, but decoding does not decompress the G2 point.
#[derive(Clone)]
pub struct LazyBlsSignature {
    /// The compressed 96-byte wire encoding. This is the canonical identity of
    /// the signature: the only field serialized, compared, or hashed.
    bytes: [u8; bls12381::Signature::LENGTH],
    /// Lazily-populated decompressed point, cached so that a signature we
    /// constructed from an already-decompressed point (e.g. a freshly
    /// aggregated multi-signature) does not pay the G2 decompression again
    /// when it is immediately verified. Never serialized, and not part of the
    /// value's identity (`Eq`/`Hash` ignore it).
    ///
    /// `OnceLock` (not `OnceCell`) because signatures are shared and
    /// decompressed across rayon threads. `Box`ed so an *empty* cache costs
    /// only a pointer, not an inline ~192-byte point — this keeps the
    /// footprint of untrusted, still-compressed wire signatures small (a
    /// peer-supplied `SignedBatchInfoMsg` is decoded in full before its length
    /// cap is enforced, so per-signature memory is attacker-amplifiable).
    decompressed: OnceLock<Box<bls12381::Signature>>,
}

impl LazyBlsSignature {
    /// Capture the compressed wire bytes of a known-valid signature, keeping the
    /// already-decompressed point cached so a subsequent [`Self::decompress`]
    /// (e.g. the verify right after local aggregation) is free.
    pub fn from_signature(sig: &bls12381::Signature) -> Self {
        let decompressed = OnceLock::new();
        // Infallible on a fresh cell.
        let _ = decompressed.set(Box::new(sig.clone()));
        Self {
            bytes: sig.to_bytes(),
            decompressed,
        }
    }

    /// Subgroup-unchecked G2 decompression — the expensive operation we defer
    /// until a payload has cleared structural validation. (The subgroup check
    /// itself still happens later, inside signature verification.) The result
    /// is cached, so repeated calls — and signatures constructed via
    /// [`Self::from_signature`] — do not decompress again.
    pub fn decompress(&self) -> Result<bls12381::Signature, CryptoMaterialError> {
        if let Some(sig) = self.decompressed.get() {
            return Ok((**sig).clone());
        }
        let sig = bls12381::Signature::try_from(self.bytes.as_slice())?;
        // Ignore a race where another thread cached it first; the value is
        // deterministic either way.
        let _ = self.decompressed.set(Box::new(sig.clone()));
        Ok(sig)
    }

    /// The raw 96-byte compressed encoding. Lets callers that only need the
    /// bytes (e.g. API hex export) avoid decompression entirely.
    pub fn to_bytes(&self) -> [u8; bls12381::Signature::LENGTH] {
        self.bytes
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn from_raw_bytes_for_test(bytes: [u8; bls12381::Signature::LENGTH]) -> Self {
        Self {
            bytes,
            decompressed: OnceLock::new(),
        }
    }
}

// Identity is the compressed bytes only; the decompression cache is a transient
// performance aid and must not affect equality, hashing, or debug output.
impl PartialEq for LazyBlsSignature {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}

impl Eq for LazyBlsSignature {}

impl std::hash::Hash for LazyBlsSignature {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.bytes.hash(state);
    }
}

impl fmt::Debug for LazyBlsSignature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "LazyBlsSignature(0x{})", hex::encode(self.bytes))
    }
}

impl Serialize for LazyBlsSignature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            // Mirror `to_encoded_string`: "0x" + hex(bytes).
            serializer.serialize_str(&format!("0x{}", hex::encode(self.bytes)))
        } else {
            // Mirror `SerializeKey`: a newtype struct named "Signature" wrapping
            // a serde_bytes byte string (length-prefixed in BCS).
            serializer.serialize_newtype_struct(
                SIGNATURE_NAME,
                serde_bytes::Bytes::new(self.bytes.as_slice()),
            )
        }
    }
}

impl<'de> Deserialize<'de> for LazyBlsSignature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Copy out exactly `LENGTH` bytes from a length-checked slice. The
        // length is validated *before* the copy, so a wrong-length field
        // (up to the 64 MiB network message cap on the consensus ingress path)
        // is rejected without allocating an owned copy of the input.
        fn to_array<E: de::Error>(bytes: &[u8]) -> Result<[u8; bls12381::Signature::LENGTH], E> {
            <[u8; bls12381::Signature::LENGTH]>::try_from(bytes).map_err(|_| {
                E::custom(format!(
                    "invalid BLS signature length: {} (expected {})",
                    bytes.len(),
                    bls12381::Signature::LENGTH
                ))
            })
        }

        if deserializer.is_human_readable() {
            // Mirror `from_encoded_string`: tolerate an AIP-80 prefix and/or a
            // leading "0x", then hex-decode.
            let encoded = <String>::deserialize(deserializer)?;
            let stripped = encoded
                .strip_prefix(bls12381::Signature::AIP_80_PREFIX)
                .unwrap_or(&encoded);
            let stripped = stripped.strip_prefix("0x").unwrap_or(stripped);
            let bytes = hex::decode(stripped).map_err(de::Error::custom)?;
            Ok(Self {
                bytes: to_array(&bytes)?,
                decompressed: OnceLock::new(),
            })
        } else {
            // Mirror `DeserializeKey`: a newtype struct named "Signature"
            // wrapping a *borrowed* byte slice. Length-check the borrowed slice
            // and copy out exactly `LENGTH` bytes, WITHOUT calling
            // `bls12381::Signature::try_from` (which would decompress) and
            // without first copying the whole (possibly oversized) field.
            #[derive(Deserialize)]
            #[serde(rename = "Signature")]
            struct Value<'a>(&'a [u8]);

            let value = Value::deserialize(deserializer)?;
            Ok(Self {
                bytes: to_array(value.0)?,
                decompressed: OnceLock::new(),
            })
        }
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

    /// The empty (uncached) footprint must stay small: untrusted wire
    /// signatures are decoded before any length cap is enforced, so a bloated
    /// per-signature size is attacker-amplifiable. Boxing the cache keeps an
    /// empty `LazyBlsSignature` near the 96-byte payload plus a pointer, rather
    /// than reserving an inline ~192-byte decompressed point.
    #[test]
    fn uncached_footprint_is_small() {
        let size = std::mem::size_of::<LazyBlsSignature>();
        assert!(
            size <= 128,
            "LazyBlsSignature grew to {size} bytes; an empty cache must not \
             reserve the decompressed point inline (box it)",
        );
    }

    /// The decompression cache is a transient perf aid and must not affect
    /// identity: a cached (`from_signature`) and an uncached (`from_raw_bytes`)
    /// instance with the same bytes must be equal, hash equally, and serialize
    /// identically. `AggregateSignature`'s derived `Eq` relies on this.
    #[test]
    fn cache_does_not_affect_identity() {
        use std::hash::{Hash, Hasher};

        let sig = sample_signature();
        let cached = LazyBlsSignature::from_signature(&sig); // cache pre-filled
        let uncached = LazyBlsSignature::from_raw_bytes_for_test(sig.to_bytes()); // cache empty

        assert_eq!(cached, uncached, "cache must not affect equality");
        assert_eq!(
            bcs::to_bytes(&cached).unwrap(),
            bcs::to_bytes(&uncached).unwrap(),
            "cache must not affect encoding",
        );

        let hash = |v: &LazyBlsSignature| {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            v.hash(&mut h);
            h.finish()
        };
        assert_eq!(hash(&cached), hash(&uncached), "cache must not affect hash");

        // Both decompress to the same signature, cached or not.
        assert_eq!(cached.decompress().unwrap(), sig);
        assert_eq!(uncached.decompress().unwrap(), sig);
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

        // An oversized field (bounded on the wire only by the network message
        // cap) must also be rejected. The borrowed slice is length-checked
        // before any copy, so this rejects without allocating a ~1 MiB owned
        // buffer.
        let oversized = serde_bytes::ByteBuf::from(vec![0u8; 1 << 20]);
        let bytes = bcs::to_bytes(&oversized).unwrap();
        assert!(bcs::from_bytes::<LazyBlsSignature>(&bytes).is_err());
    }
}
