// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This module provides APIs for aggregating and verifying BLS multi-signatures
//! implemented on top of BLS12-381 elliptic curves (https://github.com/supranational/blst).
//!
//! The `Signature` struct is used to represent either a signature share from an individual
//! signer or a multisignature aggregated from many such signers.
//!
//! The signature verification APIs in `Signature::verify` and `Signature::verify_arbitrary_msg` do NOT
//! assume the signature to be a valid group element and will implicitly "group-check" it. This
//! makes the caller's job easier and, more importantly, makes the library safer to use.

use crate::{
    bls12381::{
        bls12381_keys::{PrivateKey, PublicKey},
        DST_BLS_MULTISIG_IN_G2_WITH_POP,
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

#[derive(Debug, Clone, Eq, SerializeKey, DeserializeKey)]
/// Either A BLS signature share from an individual signer or a BLS multisignature aggregate from
/// multiple such signers
pub struct Signature {
    pub(crate) sig: blst::min_pk::Signature,
}

////////////////////////////////////////////////
// Implementation of (multi)signature structs //
////////////////////////////////////////////////

impl Signature {
    /// The length of a serialized Signature struct.
    // NOTE: We have to hardcode this here because there is no library-defined constant
    pub const LENGTH: usize = 96;

    /// Serialize a Signature.
    pub fn to_bytes(&self) -> [u8; Self::LENGTH] {
        self.sig.to_bytes()
    }

    /// Group-checks the signature (i.e., verifies the signature is a valid group element).
    /// WARNING: This is called implicitly when verifying the signature in this struct's
    /// `Signature::verify_arbitrary_msg` trait implementation. Therefore, this function should not
    /// be called separately for most use-cases. We leave it here just in case.
    pub fn group_check(&self) -> Result<()> {
        self.sig.validate(true).map_err(|e| anyhow!("{:?}", e))
    }

    /// Optimistically-aggregate multiple signatures. The individual signature shares could be
    /// adversarial. Nonetheless, for performance reasons, we do not group-check the signature shares
    /// here, since the verification of the returned multisignature includes such a group check. As
    /// a result, adversarial signature shares cannot lead to forgeries.
    pub fn aggregate(sigs: Vec<Self>) -> Result<Signature> {
        let sigs: Vec<_> = sigs.iter().map(|s| &s.sig).collect();
        let agg_sig = blst::min_pk::AggregateSignature::aggregate(&sigs[..], false)
            .map_err(|e| anyhow!("{:?}", e))?;
        Ok(Signature {
            sig: agg_sig.to_signature(),
        })
    }

    /// Return a dummy signature.
    #[cfg(any(test, feature = "fuzzing"))]
    pub fn dummy_signature() -> Self {
        // TODO: maybe Alin knows better way to generate a dummy signature
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

    fn verify<T: CryptoHash + Serialize>(&self, message: &T, public_key: &PublicKey) -> Result<()> {
        self.verify_arbitrary_msg(&signing_message(message), public_key)
    }

    /// Verifies a BLS signature share or multisignature. Does not assume the signature to be
    /// group-checked.
    /// WARNING: This function does assume the public key has been group-checked by the caller
    /// implicitly when verifying the public key's proof-of-possession (PoP) in
    /// `ProofOfPossession::verify`.
    fn verify_arbitrary_msg(&self, message: &[u8], public_key: &PublicKey) -> Result<()> {
        let result = self.sig.verify(
            true,
            message,
            DST_BLS_MULTISIG_IN_G2_WITH_POP,
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

impl TryFrom<&[u8]> for Signature {
    type Error = CryptoMaterialError;

    /// Deserializes a Signature from a sequence of bytes.
    /// WARNING: Does NOT group-check the signature! Instead, this will be done implicitly when
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
