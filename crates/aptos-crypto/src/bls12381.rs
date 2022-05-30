// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This module provides API for the BLS12-381 signature library (https://github.com/supranational/blst).

use crate::{
    hash::CryptoHash, signing_message, CryptoMaterialError, Length, PrivateKey, PublicKey,
    Signature, SigningKey, Uniform, ValidCryptoMaterial, ValidCryptoMaterialStringExt,
    VerifyingKey,
};
use anyhow::{anyhow, Result};
use aptos_crypto_derive::{DeserializeKey, SerializeKey, SilentDebug, SilentDisplay};
use blst::{blst_scalar, BLST_ERROR};
use rand_core::{OsRng, RngCore};
use serde::Serialize;
use std::convert::TryFrom;

const DST: &[u8] = b"BLS_SIG_BLS12381G1_XMD:SHA-256_SSWU_RO_NUL_";

#[derive(Clone, Eq, PartialEq, SerializeKey, DeserializeKey)]
/// A BLS12381 public key
pub struct BLS12381PublicKey {
    pubkey: blst::min_sig::PublicKey,
}

#[derive(SerializeKey, DeserializeKey, SilentDebug, SilentDisplay)]
/// A BLS12381 private key
pub struct BLS12381PrivateKey {
    privkey: blst::min_sig::SecretKey,
}

#[derive(Debug, Clone, Eq, PartialEq, SerializeKey, DeserializeKey)]
/// A BLS12381 signature
pub struct BLS12381Signature {
    sig: blst::min_sig::Signature,
}

impl BLS12381PublicKey {
    /// The length of the BLS12381PublicKey
    pub const LENGTH: usize = 96;

    /// Serialize a BLS12381PublicKey.
    pub fn to_bytes(&self) -> [u8; Self::LENGTH] {
        self.pubkey.to_bytes()
    }

    /// Validate the public key.
    pub fn validate(&self) -> Result<()> {
        self.pubkey.validate().map_err(|e| anyhow!("{:?}", e))
    }
}

impl BLS12381PrivateKey {
    /// The length of the BLS12381PrivateKey
    pub const LENGTH: usize = 32;

    /// Serialize a BLS12381PrivateKey.
    pub fn to_bytes(&self) -> [u8; Self::LENGTH] {
        self.privkey.to_bytes()
    }
}

impl BLS12381Signature {
    /// The length of the BLS12381Signature
    pub const LENGTH: usize = 48;

    /// Serialize a BLS12381Signature.
    pub fn to_bytes(&self) -> [u8; Self::LENGTH] {
        self.sig.to_bytes()
    }

    /// Validate the signature.
    pub fn validate(&self) -> Result<()> {
        self.sig.validate(true).map_err(|e| anyhow!("{:?}", e))
    }

    /// Aggregate multiple signatures, assume individual signature are validated.
    pub fn aggregate(sigs: Vec<Self>) -> Result<Self> {
        let sigs: Vec<_> = sigs.iter().map(|s| &s.sig).collect();
        let agg_sig = blst::min_sig::AggregateSignature::aggregate(&sigs[..], false)
            .map_err(|e| anyhow!("{:?}", e))?;
        Ok(Self {
            sig: agg_sig.to_signature(),
        })
    }

    /// Verify aggregated signature, assume public keys are checked.
    pub fn aggregate_verify<T: CryptoHash + Serialize>(
        &self,
        message: &T,
        public_keys: Vec<&BLS12381PublicKey>,
    ) -> Result<()> {
        let bytes = signing_message(message);
        let pubkeys: Vec<_> = public_keys.iter().map(|p| &p.pubkey).collect();
        let result = self
            .sig
            .fast_aggregate_verify(true, &bytes, DST, &pubkeys[..]);
        if result == BLST_ERROR::BLST_SUCCESS {
            Ok(())
        } else {
            Err(anyhow!("{:?}", result))
        }
    }
}

///////////////////////
// PrivateKey Traits //
///////////////////////

impl PrivateKey for BLS12381PrivateKey {
    type PublicKeyMaterial = BLS12381PublicKey;
}

impl SigningKey for BLS12381PrivateKey {
    type VerifyingKeyMaterial = BLS12381PublicKey;
    type SignatureMaterial = BLS12381Signature;

    fn sign<T: CryptoHash + Serialize>(&self, message: &T) -> BLS12381Signature {
        BLS12381Signature {
            sig: self.privkey.sign(&signing_message(message), DST, &[]),
        }
    }

    #[cfg(any(test, feature = "fuzzing"))]
    fn sign_arbitrary_message(&self, message: &[u8]) -> BLS12381Signature {
        BLS12381Signature {
            sig: self.privkey.sign(message, DST, &[]),
        }
    }
}

impl ValidCryptoMaterial for BLS12381PrivateKey {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }
}

impl Length for BLS12381PrivateKey {
    fn length(&self) -> usize {
        Self::LENGTH
    }
}

impl TryFrom<&[u8]> for BLS12381PrivateKey {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> std::result::Result<Self, CryptoMaterialError> {
        Ok(Self {
            privkey: blst::min_sig::SecretKey::from_bytes(bytes)
                .map_err(|_| CryptoMaterialError::DeserializationError)?,
        })
    }
}

impl Uniform for BLS12381PrivateKey {
    fn generate<R>(rng: &mut R) -> Self
    where
        R: ::rand::RngCore + ::rand::CryptoRng,
    {
        let mut ikm = [0u8; 32];
        rng.fill_bytes(&mut ikm);
        let privkey =
            blst::min_sig::SecretKey::key_gen(&ikm, &[]).expect("ikm length should be higher");
        Self { privkey }
    }
}

//////////////////////
// PublicKey Traits //
//////////////////////

impl From<&BLS12381PrivateKey> for BLS12381PublicKey {
    fn from(private_key: &BLS12381PrivateKey) -> Self {
        Self {
            pubkey: private_key.privkey.sk_to_pk(),
        }
    }
}

impl PublicKey for BLS12381PublicKey {
    type PrivateKeyMaterial = BLS12381PrivateKey;
}

impl VerifyingKey for BLS12381PublicKey {
    type SigningKeyMaterial = BLS12381PrivateKey;
    type SignatureMaterial = BLS12381Signature;
}

impl ValidCryptoMaterial for BLS12381PublicKey {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }
}

impl Length for BLS12381PublicKey {
    fn length(&self) -> usize {
        Self::LENGTH
    }
}

impl TryFrom<&[u8]> for BLS12381PublicKey {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> std::result::Result<Self, CryptoMaterialError> {
        Ok(Self {
            pubkey: blst::min_sig::PublicKey::from_bytes(bytes)
                .map_err(|_| CryptoMaterialError::DeserializationError)?,
        })
    }
}

impl std::hash::Hash for BLS12381PublicKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let encoded_pubkey = self.to_bytes();
        state.write(&encoded_pubkey);
    }
}

//////////////////////
// Signature Traits //
//////////////////////
impl Signature for BLS12381Signature {
    type VerifyingKeyMaterial = BLS12381PublicKey;
    type SigningKeyMaterial = BLS12381PrivateKey;

    fn verify<T: CryptoHash + Serialize>(
        &self,
        message: &T,
        public_key: &BLS12381PublicKey,
    ) -> Result<()> {
        self.verify_arbitrary_msg(&signing_message(message), public_key)
    }

    fn verify_arbitrary_msg(&self, message: &[u8], public_key: &BLS12381PublicKey) -> Result<()> {
        let result = self
            .sig
            .verify(true, message, DST, &[], &public_key.pubkey, true);
        if result == BLST_ERROR::BLST_SUCCESS {
            Ok(())
        } else {
            Err(anyhow!("{:?}", result))
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }

    fn batch_verify<T: CryptoHash + Serialize>(
        message: &T,
        keys_and_signatures: Vec<(Self::VerifyingKeyMaterial, Self)>,
    ) -> Result<()> {
        let num_sigs = keys_and_signatures.len();
        let mut rands: Vec<blst_scalar> = Vec::with_capacity(num_sigs);
        let mut rng = OsRng;

        for _ in 0..num_sigs {
            let mut b = [0u8; 32];
            rng.fill_bytes(&mut b);
            rands.push(blst_scalar { b });
        }

        let message_bytes = signing_message(message);
        let msgs_refs = (0..num_sigs)
            .map(|_| &message_bytes[..])
            .collect::<Vec<_>>();

        let mut pubkeys = Vec::with_capacity(num_sigs);
        let mut sigs = Vec::with_capacity(num_sigs);
        for (key, sig) in &keys_and_signatures {
            pubkeys.push(&key.pubkey);
            sigs.push(&sig.sig);
        }

        let result = blst::min_sig::Signature::verify_multiple_aggregate_signatures(
            &msgs_refs[..],
            DST,
            &pubkeys[..],
            false,
            &sigs[..],
            true,
            &rands,
            64,
        );
        if result == BLST_ERROR::BLST_SUCCESS {
            Ok(())
        } else {
            Err(anyhow!("{:?}", result))
        }
    }
}

impl ValidCryptoMaterial for BLS12381Signature {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }
}
impl Length for BLS12381Signature {
    fn length(&self) -> usize {
        Self::LENGTH
    }
}

impl TryFrom<&[u8]> for BLS12381Signature {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> std::result::Result<BLS12381Signature, CryptoMaterialError> {
        Ok(Self {
            sig: blst::min_sig::Signature::from_bytes(bytes)
                .map_err(|_| CryptoMaterialError::DeserializationError)?,
        })
    }
}

impl std::hash::Hash for BLS12381Signature {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let encoded_signature = self.to_bytes();
        state.write(&encoded_signature);
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        bls12381::*,
        test_utils::{KeyPair, TestAptosCrypto},
    };
    use rand_core::OsRng;

    #[test]
    fn bls12381_single_message() {
        let message = b"Hello world";
        let mut rng = OsRng;
        let key_pair = KeyPair::<BLS12381PrivateKey, BLS12381PublicKey>::generate(&mut rng);
        let signature = key_pair.private_key.sign_arbitrary_message(message);
        assert!(signature
            .verify_arbitrary_msg(message, &key_pair.public_key)
            .is_ok());
    }

    #[test]
    fn bls12381_batch_verify() {
        let message = TestAptosCrypto("lalala".into());
        let mut rng = OsRng;
        let mut key_pairs = vec![];
        for _ in 0..1000 {
            key_pairs.push(KeyPair::<BLS12381PrivateKey, BLS12381PublicKey>::generate(
                &mut rng,
            ));
        }
        let mut keys_and_signatures = vec![];
        for keys in key_pairs {
            let signature = keys.private_key.sign(&message);
            keys_and_signatures.push((keys.public_key.clone(), signature));
        }
        assert!(BLS12381Signature::batch_verify(&message, keys_and_signatures).is_ok());
    }

    #[test]
    fn bls12381_aggregate_verify() {
        let mut rng = OsRng;
        let message = TestAptosCrypto("lalala".into());
        let mut key_pairs = vec![];
        for _ in 0..1000 {
            key_pairs.push(KeyPair::<BLS12381PrivateKey, BLS12381PublicKey>::generate(
                &mut rng,
            ));
        }
        let mut signatures = vec![];
        let mut pubkeys = vec![];
        for keys in key_pairs.iter().step_by(2) {
            let signature = keys.private_key.sign(&message);
            signatures.push(signature);
            pubkeys.push(&keys.public_key);
        }
        let agg_sig = BLS12381Signature::aggregate(signatures).unwrap();

        assert!(agg_sig.aggregate_verify(&message, pubkeys).is_ok());
    }
    // TODO: add some negative tests/proptests
}
