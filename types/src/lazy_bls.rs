// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Lazy wire types for BLS public keys and aggregate/multi-signatures.
//!
//! `LazyBlsPublicKey` carries the same on-wire encoding as
//! `aptos_crypto::bls12381::PublicKey` but leaves its compressed G1 point as
//! bytes until an authenticated validator set is accepted. This keeps a forged
//! `EpochChangeProof` from forcing one curve decompression per advertised
//! validator during network deserialization.
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

use crate::lazy_crypto::{LazyCryptoMaterial, LazyCryptoMaterialType};
use aptos_crypto::{bls12381, CryptoMaterialError};
use serde::{Deserialize, Serialize};
use std::fmt;

impl LazyCryptoMaterialType<{ bls12381::PublicKey::LENGTH }> for bls12381::PublicKey {
    const SERDE_NAME: &'static str = "PublicKey";

    fn fixed_bytes(&self) -> [u8; Self::LENGTH] {
        self.to_bytes()
    }
}

impl LazyCryptoMaterialType<{ bls12381::Signature::LENGTH }> for bls12381::Signature {
    const SERDE_NAME: &'static str = "Signature";

    fn fixed_bytes(&self) -> [u8; Self::LENGTH] {
        self.to_bytes()
    }
}

/// Wire-identical to a BLS public key, with G1 decompression deferred until use.
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LazyBlsPublicKey(
    LazyCryptoMaterial<bls12381::PublicKey, { bls12381::PublicKey::LENGTH }>,
);

impl LazyBlsPublicKey {
    pub fn from_public_key(public_key: &bls12381::PublicKey) -> Self {
        Self(LazyCryptoMaterial::from_material(public_key))
    }

    /// Decompress and curve-check on first use, then reuse the cached point.
    pub fn decompress(&self) -> Result<&bls12381::PublicKey, CryptoMaterialError> {
        self.0.materialize()
    }

    pub fn to_bytes(&self) -> [u8; bls12381::PublicKey::LENGTH] {
        self.0.to_bytes()
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn from_raw_bytes_for_test(bytes: [u8; bls12381::PublicKey::LENGTH]) -> Self {
        Self(LazyCryptoMaterial::from_raw_bytes_for_test(bytes))
    }
}

impl fmt::Debug for LazyBlsPublicKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<bls12381::PublicKey> for LazyBlsPublicKey {
    fn from(public_key: bls12381::PublicKey) -> Self {
        Self::from_public_key(&public_key)
    }
}

#[cfg(any(test, feature = "fuzzing"))]
impl proptest::arbitrary::Arbitrary for LazyBlsPublicKey {
    type Parameters = ();
    type Strategy = proptest::strategy::BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        use proptest::strategy::Strategy;

        proptest::arbitrary::any::<bls12381::PublicKey>()
            .prop_map(|key| Self::from_public_key(&key))
            .boxed()
    }
}

/// Wire-identical to a BLS signature, with G2 decompression deferred until use.
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LazyBlsSignature(
    LazyCryptoMaterial<bls12381::Signature, { bls12381::Signature::LENGTH }>,
);

impl LazyBlsSignature {
    pub fn from_signature(sig: &bls12381::Signature) -> Self {
        Self(LazyCryptoMaterial::from_material(sig))
    }

    /// Decompress on first use. Subgroup checking still happens in verification.
    pub fn decompress(&self) -> Result<bls12381::Signature, CryptoMaterialError> {
        self.0.materialize().cloned()
    }

    pub fn to_bytes(&self) -> [u8; bls12381::Signature::LENGTH] {
        self.0.to_bytes()
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn from_raw_bytes_for_test(bytes: [u8; bls12381::Signature::LENGTH]) -> Self {
        Self(LazyCryptoMaterial::from_raw_bytes_for_test(bytes))
    }
}

impl fmt::Debug for LazyBlsSignature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "LazyBlsSignature(0x{})", hex::encode(self.to_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_crypto::{bls12381::PrivateKey, test_utils::TestAptosCrypto, SigningKey, Uniform};

    fn sample_public_key() -> bls12381::PublicKey {
        let mut rng = rand::thread_rng();
        let sk = PrivateKey::generate(&mut rng);
        bls12381::PublicKey::from(&sk)
    }

    #[test]
    fn lazy_public_key_wire_compat_bcs_and_json() {
        for _ in 0..16 {
            let public_key = sample_public_key();
            let lazy = LazyBlsPublicKey::from_public_key(&public_key);

            let public_key_bcs = bcs::to_bytes(&public_key).unwrap();
            let lazy_bcs = bcs::to_bytes(&lazy).unwrap();
            assert_eq!(public_key_bcs, lazy_bcs);

            let decoded: LazyBlsPublicKey = bcs::from_bytes(&public_key_bcs).unwrap();
            assert_eq!(decoded, lazy);
            assert_eq!(decoded.decompress().unwrap(), &public_key);

            let public_key_json = serde_json::to_string(&public_key).unwrap();
            let lazy_json = serde_json::to_string(&lazy).unwrap();
            assert_eq!(public_key_json, lazy_json);
            let decoded: LazyBlsPublicKey = serde_json::from_str(&public_key_json).unwrap();
            assert_eq!(decoded.decompress().unwrap(), &public_key);
        }
    }

    #[test]
    fn invalid_public_key_is_rejected_only_when_materialized() {
        let invalid = LazyBlsPublicKey::from_raw_bytes_for_test([0u8; bls12381::PublicKey::LENGTH]);
        let bytes = bcs::to_bytes(&invalid).unwrap();

        let decoded: LazyBlsPublicKey = bcs::from_bytes(&bytes).unwrap();
        assert!(decoded.decompress().is_err());
        assert!(bcs::from_bytes::<bls12381::PublicKey>(&bytes).is_err());
    }

    #[test]
    fn uncached_public_key_footprint_is_small() {
        let size = std::mem::size_of::<LazyBlsPublicKey>();
        assert!(
            size <= 64,
            "LazyBlsPublicKey grew to {size} bytes; its decompressed cache must remain boxed",
        );
    }

    #[test]
    fn public_key_rejects_wrong_length_before_materialization() {
        for length in [
            0,
            bls12381::PublicKey::LENGTH - 1,
            bls12381::PublicKey::LENGTH + 1,
            1 << 20,
        ] {
            let bytes = bcs::to_bytes(&serde_bytes::ByteBuf::from(vec![0; length])).unwrap();
            assert!(bcs::from_bytes::<LazyBlsPublicKey>(&bytes).is_err());
        }
    }

    #[test]
    fn lazy_material_accepts_aip80_prefixes() {
        use aptos_crypto::traits::ValidCryptoMaterial;

        let key = sample_public_key();
        let encoded = format!(
            "{}0x{}",
            bls12381::PublicKey::AIP_80_PREFIX,
            hex::encode(key.to_bytes())
        );
        let decoded: LazyBlsPublicKey =
            serde_json::from_str(&serde_json::to_string(&encoded).unwrap()).unwrap();
        assert_eq!(decoded.decompress().unwrap(), &key);

        let sig = sample_signature();
        let encoded = format!(
            "{}0x{}",
            bls12381::Signature::AIP_80_PREFIX,
            hex::encode(sig.to_bytes())
        );
        let decoded: LazyBlsSignature =
            serde_json::from_str(&serde_json::to_string(&encoded).unwrap()).unwrap();
        assert_eq!(decoded.decompress().unwrap(), sig);
    }

    #[test]
    fn materialization_populates_and_reuses_cache() {
        let key = sample_public_key();
        let lazy = LazyBlsPublicKey::from_raw_bytes_for_test(key.to_bytes());
        assert!(lazy.0.decoded_for_test().is_none());
        let materialized = lazy.decompress().unwrap();
        assert!(std::ptr::eq(materialized, lazy.decompress().unwrap()));
        assert_eq!(materialized, &key);

        let sig = sample_signature();
        let lazy = LazyBlsSignature::from_raw_bytes_for_test(sig.to_bytes());
        assert!(lazy.0.decoded_for_test().is_none());
        assert_eq!(lazy.decompress().unwrap(), sig);
        assert!(lazy.0.decoded_for_test().is_some());
    }

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
