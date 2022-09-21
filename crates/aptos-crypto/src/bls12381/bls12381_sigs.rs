// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This module provides APIs for aggregating and verifying Boneh-Lynn-Shacham (BLS) aggregate
//! signatures (including individual signatures and multisignatures), implemented on top of
//! Barreto-Lynn-Scott BLS12-381 elliptic curves (https://github.com/supranational/blst).
//!
//! The `Signature` struct is used to represent either a:
//!
//!  1. signature share from an individual signer
//!  2. multisignature on a single message from many signers
//!  3. aggregate signature on different messages from many signers
//!
//! The signature verification APIs in `Signature::verify`, `Signature::verify_arbitrary_msg`,
//! `Signature::verify_aggregate` and `Signature::verify_aggregate_arbitrary_msg` do NOT
//! assume the signature to be a valid group element and will implicitly "subgroup-check" it. This
//! makes the caller's job easier and, more importantly, makes the library safer to use.

use crate::{
    bls12381::{
        bls12381_keys::{PrivateKey, PublicKey},
        DST_BLS_SIG_IN_G2_WITH_POP,
    },
    hash::CryptoHash,
    signing_message, traits, CryptoMaterialError, Length, ValidCryptoMaterial,
    ValidCryptoMaterialStringExt,
};
use anyhow::{anyhow, Result};
use aptos_crypto_derive::{DeserializeKey, SerializeKey};
use blst::BLST_ERROR;
use serde::Serialize;
use std::convert::TryFrom;
use std::fmt;

#[derive(Clone, Eq, SerializeKey, DeserializeKey)]
/// Either (1) a BLS signature share from an individual signer, (2) a BLS multisignature or (3) a
/// BLS aggregate signature
pub struct Signature {
    pub(crate) sig: blst::min_pk::Signature,
}

////////////////////////////////////////
// Implementation of Signature struct //
////////////////////////////////////////

impl Signature {
    /// The length of a serialized Signature struct.
    // NOTE: We have to hardcode this here because there is no library-defined constant
    pub const LENGTH: usize = 96;

    /// Serialize a Signature.
    pub fn to_bytes(&self) -> [u8; Self::LENGTH] {
        self.sig.to_bytes()
    }

    /// Subgroup-checks the signature (i.e., verifies the signature is a valid group element).
    ///
    /// WARNING: Subgroup-checking is done implicitly when verifying signatures via
    /// `Signature::verify_arbitrary_msg`. Therefore, this function should not be called separately
    /// for most use-cases. We leave it here just in case.
    pub fn subgroup_check(&self) -> Result<()> {
        self.sig.validate(true).map_err(|e| anyhow!("{:?}", e))
    }

    /// Optimistically-aggregate signatures shares into either (1) a multisignature or (2) an aggregate
    /// signature. The individual signature shares could be adversarial. Nonetheless, for performance
    /// reasons, we do not subgroup-check the signature shares here, since the verification of the
    /// returned multi-or-aggregate signature includes such a subgroup check. As a result, adversarial
    /// signature shares cannot lead to forgeries.
    pub fn aggregate(sigs: Vec<Self>) -> Result<Signature> {
        let sigs: Vec<_> = sigs.iter().map(|s| &s.sig).collect();
        let agg_sig = blst::min_pk::AggregateSignature::aggregate(&sigs[..], false)
            .map_err(|e| anyhow!("{:?}", e))?;
        Ok(Signature {
            sig: agg_sig.to_signature(),
        })
    }

    /// Verifies an aggregate signature on the messages in `msgs` under the public keys in `pks`.
    /// Specifically, verifies that each `msgs[i]` is signed under `pks[i]`. The messages in `msgs`
    /// do *not* have to be all different, since we use proofs-of-possession (PoPs) to prevent rogue
    /// key attacks.
    ///
    /// WARNING: This function assumes that the public keys have been subgroup-checked by the caller
    /// implicitly when verifying their proof-of-possession (PoP) in `ProofOfPossession::verify`.
    pub fn verify_aggregate_arbitrary_msg(&self, msgs: &[&[u8]], pks: &[&PublicKey]) -> Result<()> {
        let pks = pks
            .iter()
            .map(|&pk| &pk.pubkey)
            .collect::<Vec<&blst::min_pk::PublicKey>>();

        let result = self
            .sig
            .aggregate_verify(true, msgs, DST_BLS_SIG_IN_G2_WITH_POP, &pks, false);

        if result == BLST_ERROR::BLST_SUCCESS {
            Ok(())
        } else {
            Err(anyhow!("{:?}", result))
        }
    }

    /// Serializes the messages of type `T` to bytes and calls `Signature::verify_aggregate_arbitrary_msg`.
    pub fn verify_aggregate<T: CryptoHash + Serialize>(
        &self,
        msgs: &[&T],
        pks: &[&PublicKey],
    ) -> Result<()> {
        let mut messages: Vec<Vec<u8>> = vec![];
        for message in msgs {
            messages.push(signing_message(*message)?);
        }

        let msgs_refs = messages
            .iter()
            .map(|m| m.as_slice())
            .collect::<Vec<&[u8]>>();

        self.verify_aggregate_arbitrary_msg(&msgs_refs, pks)
    }

    /// Return a dummy signature for testing.
    #[cfg(any(test, feature = "fuzzing"))]
    pub fn dummy_signature() -> Self {
        use crate::{Genesis, SigningKey};

        let private_key = PrivateKey::genesis();

        let msg = b"hello foo";
        private_key.sign_arbitrary_message(msg)
    }
}

///////////////////////////
// SignatureShare Traits //
///////////////////////////
impl traits::Signature for Signature {
    type VerifyingKeyMaterial = PublicKey;
    type SigningKeyMaterial = PrivateKey;

    /// Serializes the message of type `T` to bytes and calls `Signature::verify_arbitrary_msg`.
    fn verify<T: CryptoHash + Serialize>(&self, message: &T, public_key: &PublicKey) -> Result<()> {
        self.verify_arbitrary_msg(&signing_message(message)?, public_key)
    }

    /// Verifies a BLS signature share or multisignature. Does not assume the signature to be
    /// subgroup-checked. (For verifying aggregate signatures on different messages, a different
    /// `verify_aggregate_arbitray_msg` function can be used.)
    ///
    /// WARNING: This function does assume the public key has been subgroup-checked by the caller,
    /// either (1) implicitly when verifying the public key's proof-of-possession (PoP) in
    /// `ProofOfPossession::verify` or (2) via `Validatable::<PublicKey>::validate()`.
    fn verify_arbitrary_msg(&self, message: &[u8], public_key: &PublicKey) -> Result<()> {
        let result = self.sig.verify(
            true,
            message,
            DST_BLS_SIG_IN_G2_WITH_POP,
            &[],
            &public_key.pubkey,
            false,
        );
        if result == BLST_ERROR::BLST_SUCCESS {
            Ok(())
        } else {
            Err(anyhow!("{:?}", result))
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }
}

impl ValidCryptoMaterial for Signature {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }
}
impl Length for Signature {
    fn length(&self) -> usize {
        Self::LENGTH
    }
}

impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.to_bytes()))
    }
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.to_bytes()))
    }
}

impl TryFrom<&[u8]> for Signature {
    type Error = CryptoMaterialError;

    /// Deserializes a Signature from a sequence of bytes.
    ///
    /// WARNING: Does NOT subgroup-check the signature! Instead, this will be done implicitly when
    /// verifying the signature.
    fn try_from(bytes: &[u8]) -> std::result::Result<Signature, CryptoMaterialError> {
        Ok(Self {
            sig: blst::min_pk::Signature::from_bytes(bytes)
                .map_err(|_| CryptoMaterialError::DeserializationError)?,
        })
    }
}

impl std::hash::Hash for Signature {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let encoded_signature = self.to_bytes();
        state.write(&encoded_signature);
    }
}

// PartialEq trait implementation is required by the std::hash::Hash trait implementation above
impl PartialEq for Signature {
    fn eq(&self, other: &Self) -> bool {
        self.to_bytes()[..] == other.to_bytes()[..]
    }
}
