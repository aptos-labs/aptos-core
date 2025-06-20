// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module provides APIs for private keys and public keys used in Boneh-Lynn-Shacham (BLS)
//! aggregate signatures (including individual signatures and multisignatures) implemented on top of
//! Barreto-Lynn-Scott BLS12-381 elliptic curves (<https://github.com/supranational/blst>).
//!
//! The `PublicKey` struct is used to represent both the public key of an individual signer
//! as well as the aggregate public key of several signers. Before passing this struct as an
//! argument, the caller should *always* verify its proof-of-possession (PoP) via
//! `ProofOfPossession::verify`.
//!
//! The `PublicKey::aggregate` API assumes the caller has already verified
//! proofs-of-possession for all the given public keys and therefore all public keys are valid,
//! prime-order subgroup elements.
//!
//! In general, with the exception of `ProofOfPossession::verify` no library function should
//! be given a public key as argument without first verifying that public key's PoP. Note that
//! for aggregate public keys obtained via `PublicKey::aggregate` there is no PoP to verify, but
//! the security assumption will be that all public keys given as input to this function have had
//! their PoPs verified.

use crate::{
    bls12381, bls12381::DST_BLS_SIG_IN_G2_WITH_POP, hash::CryptoHash, signing_message, traits,
    CryptoMaterialError, Genesis, Length, Uniform, ValidCryptoMaterial,
    ValidCryptoMaterialStringExt, VerifyingKey,
};
use anyhow::{anyhow, Result};
use aptos_crypto_derive::{DeserializeKey, SerializeKey, SilentDebug, SilentDisplay};
use serde::Serialize;
use std::{convert::TryFrom, fmt};

#[derive(Clone, Eq, SerializeKey, DeserializeKey)]
/// A BLS12381 public key
pub struct PublicKey {
    pub(crate) pubkey: blst::min_pk::PublicKey,
    // NOTE: In order to minimize the size of this struct, we do not keep the PoP here.
    // One reason for this is these PKs are stored in the root of the Merkle accumulator.
}

#[derive(SerializeKey, DeserializeKey, SilentDebug, SilentDisplay)]
/// A BLS12381 private key
pub struct PrivateKey {
    pub(crate) privkey: blst::min_pk::SecretKey,
}

//////////////////////////////////////////////////////
// Implementation of public-and-private key structs //
//////////////////////////////////////////////////////

impl PublicKey {
    /// The length of a serialized PublicKey struct.
    // NOTE: We have to hardcode this here because there is no library-defined constant.
    pub const LENGTH: usize = 48;

    /// Serialize a PublicKey.
    pub fn to_bytes(&self) -> [u8; Self::LENGTH] {
        self.pubkey.to_bytes()
    }

    /// Subgroup-checks the public key (i.e., verifies the public key is an element of the prime-order
    /// subgroup and it is not the identity element).
    ///
    /// WARNING: Subgroup-checking is done implicitly when verifying the proof-of-possession (PoP) for
    /// this public key  in `ProofOfPossession::verify`, so this function should not be called
    /// separately for most use-cases. We leave it here just in case.
    pub fn subgroup_check(&self) -> Result<()> {
        self.pubkey.validate().map_err(|e| anyhow!("{:?}", e))
    }

    /// Aggregates the public keys of several signers into an aggregate public key, which can be later
    /// used to verify a multisig aggregated from those signers.
    ///
    /// WARNING: This function assumes all public keys have had their proofs-of-possession verified
    /// and have thus been group-checked.
    pub fn aggregate(pubkeys: Vec<&Self>) -> Result<PublicKey> {
        let blst_pubkeys: Vec<_> = pubkeys.iter().map(|pk| &pk.pubkey).collect();

        // CRYPTONOTE(Alin): We assume the PKs have had their PoPs verified and thus have also been subgroup-checked
        let aggpk = blst::min_pk::AggregatePublicKey::aggregate(&blst_pubkeys[..], false)
            .map_err(|e| anyhow!("{:?}", e))?;

        Ok(PublicKey {
            pubkey: aggpk.to_public_key(),
        })
    }
}

impl PrivateKey {
    /// The length of a serialized PrivateKey struct.
    // NOTE: We have to hardcode this here because there is no library-defined constant
    pub const LENGTH: usize = 32;

    /// Serialize a PrivateKey.
    pub fn to_bytes(&self) -> [u8; Self::LENGTH] {
        self.privkey.to_bytes()
    }
}

///////////////////////
// PrivateKey Traits //
///////////////////////

impl traits::PrivateKey for PrivateKey {
    type PublicKeyMaterial = PublicKey;
}

impl traits::SigningKey for PrivateKey {
    type SignatureMaterial = bls12381::Signature;
    type VerifyingKeyMaterial = PublicKey;

    fn sign<T: CryptoHash + Serialize>(
        &self,
        message: &T,
    ) -> Result<bls12381::Signature, CryptoMaterialError> {
        Ok(bls12381::Signature {
            sig: self
                .privkey
                .sign(&signing_message(message)?, DST_BLS_SIG_IN_G2_WITH_POP, &[]),
        })
    }

    #[cfg(any(test, feature = "fuzzing"))]
    fn sign_arbitrary_message(&self, message: &[u8]) -> bls12381::Signature {
        bls12381::Signature {
            sig: self.privkey.sign(message, DST_BLS_SIG_IN_G2_WITH_POP, &[]),
        }
    }
}

impl traits::ValidCryptoMaterial for PrivateKey {
    const AIP_80_PREFIX: &'static str = "bls12381-priv-";

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }
}

impl Length for PrivateKey {
    fn length(&self) -> usize {
        Self::LENGTH
    }
}

impl TryFrom<&[u8]> for PrivateKey {
    type Error = CryptoMaterialError;

    /// Deserializes a PrivateKey from a sequence of bytes.
    fn try_from(bytes: &[u8]) -> std::result::Result<Self, CryptoMaterialError> {
        Ok(Self {
            privkey: blst::min_pk::SecretKey::from_bytes(bytes)
                .map_err(|_| CryptoMaterialError::DeserializationError)?,
        })
    }
}

impl Uniform for PrivateKey {
    fn generate<R>(rng: &mut R) -> Self
    where
        R: ::rand::RngCore + ::rand::CryptoRng,
    {
        // CRYPTONOTE(Alin): This "initial key material (IKM)" is the randomness used inside key_gen
        // below to pseudo-randomly derive the secret key via an HKDF
        // (see <https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-bls-signature#section-2.3>)
        let mut ikm = [0u8; 32];
        rng.fill_bytes(&mut ikm);
        let privkey =
            blst::min_pk::SecretKey::key_gen(&ikm, &[]).expect("ikm length should be higher");
        Self { privkey }
    }
}

impl Genesis for PrivateKey {
    fn genesis() -> Self {
        let mut buf = [0u8; Self::LENGTH];
        buf[Self::LENGTH - 1] = 1;
        Self::try_from(buf.as_ref()).unwrap()
    }
}

#[cfg(feature = "assert-private-keys-not-cloneable")]
static_assertions::assert_not_impl_any!(PrivateKey: Clone);

#[cfg(any(test, feature = "cloneable-private-keys"))]
impl Clone for PrivateKey {
    fn clone(&self) -> Self {
        let serialized: &[u8] = &(self.to_bytes());
        PrivateKey::try_from(serialized).unwrap()
    }
}

//////////////////////
// PublicKey Traits //
//////////////////////

impl From<&PrivateKey> for PublicKey {
    fn from(private_key: &PrivateKey) -> Self {
        Self {
            pubkey: private_key.privkey.sk_to_pk(),
        }
    }
}

impl traits::PublicKey for PublicKey {
    type PrivateKeyMaterial = PrivateKey;
}

impl VerifyingKey for PublicKey {
    type SignatureMaterial = bls12381::Signature;
    type SigningKeyMaterial = PrivateKey;
}

impl ValidCryptoMaterial for PublicKey {
    const AIP_80_PREFIX: &'static str = "bls12381-pub-";

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }
}

impl Length for PublicKey {
    fn length(&self) -> usize {
        Self::LENGTH
    }
}

impl TryFrom<&[u8]> for PublicKey {
    type Error = CryptoMaterialError;

    /// Deserializes a PublicKey from a sequence of bytes.
    ///
    /// WARNING: Does NOT subgroup-check the public key! Instead, the caller is responsible for
    /// verifying the public key's proof-of-possession (PoP) via `ProofOfPossession::verify`,
    /// which implicitly subgroup-checks the public key.
    ///
    /// NOTE: This function will only check that the PK is a point on the curve:
    ///  - `blst::min_pk::PublicKey::from_bytes(bytes)` calls `blst::min_pk::PublicKey::deserialize(bytes)`,
    ///    which calls `$pk_deser` in <https://github.com/supranational/blst/blob/711e1eec747772e8cae15d4a1885dd30a32048a4/bindings/rust/src/lib.rs#L734>,
    ///    which is mapped to `blst_p1_deserialize` in <https://github.com/supranational/blst/blob/711e1eec747772e8cae15d4a1885dd30a32048a4/bindings/rust/src/lib.rs#L1652>
    ///  - `blst_p1_deserialize` eventually calls `POINTonE1_Deserialize_BE`, which checks
    ///    the point is on the curve: <https://github.com/supranational/blst/blob/711e1eec747772e8cae15d4a1885dd30a32048a4/src/e1.c#L296>
    fn try_from(bytes: &[u8]) -> std::result::Result<Self, CryptoMaterialError> {
        Ok(Self {
            pubkey: blst::min_pk::PublicKey::from_bytes(bytes)
                .map_err(|_| CryptoMaterialError::DeserializationError)?,
        })
    }
}

impl std::hash::Hash for PublicKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let encoded_pubkey = self.to_bytes();
        state.write(&encoded_pubkey);
    }
}

// PartialEq trait implementation is required by the std::hash::Hash trait implementation above
impl PartialEq for PublicKey {
    fn eq(&self, other: &Self) -> bool {
        self.to_bytes()[..] == other.to_bytes()[..]
    }
}

impl fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.to_bytes()))
    }
}

impl fmt::Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.to_bytes()))
    }
}

#[cfg(any(test, feature = "fuzzing"))]
use crate::test_utils::KeyPair;
#[cfg(any(test, feature = "fuzzing"))]
use proptest::prelude::*;

/// Produces a uniformly random BLS keypair from a seed
#[cfg(any(test, feature = "fuzzing"))]
pub fn keypair_strategy() -> impl Strategy<Value = KeyPair<PrivateKey, PublicKey>> {
    crate::test_utils::uniform_keypair_strategy::<PrivateKey, PublicKey>()
}

#[cfg(any(test, feature = "fuzzing"))]
impl proptest::arbitrary::Arbitrary for PublicKey {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        crate::test_utils::uniform_keypair_strategy::<PrivateKey, PublicKey>()
            .prop_map(|v| v.public_key)
            .boxed()
    }
}
