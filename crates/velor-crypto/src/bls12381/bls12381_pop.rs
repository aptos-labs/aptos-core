// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module provides APIs for _proofs-of-possesion (PoPs)_ used to prevent _rogue-key attacks_,
//! both for multisignatures and aggregate signatures.
//!
//! Rogue-key attacks were first introduced by Micali, Ohta and Reyzin [^MOR01] and PoPs were first
//! introduced by Ristenpart and Yilek [^RY07].
//!
//! [^MOR01]: Accountable-Subgroup Multisignatures: Extended Abstract; by Micali, Silvio and Ohta, Kazuo and Reyzin, Leonid; in Proceedings of the 8th ACM Conference on Computer and Communications Security; 2001;
//! [^RY07]: The Power of Proofs-of-Possession: Securing Multiparty Signatures against Rogue-Key Attacks; by Ristenpart, Thomas and Yilek, Scott; in Advances in Cryptology - EUROCRYPT 2007; 2007

use crate::{
    bls12381::bls12381_keys::{PrivateKey, PublicKey},
    CryptoMaterialError, Length, ValidCryptoMaterial, ValidCryptoMaterialStringExt,
};
use anyhow::{anyhow, Result};
use velor_crypto_derive::{DeserializeKey, SerializeKey};
use blst::BLST_ERROR;
use std::{convert::TryFrom, fmt};

/// Domain separation tag (DST) for hashing a public key before computing its proof-of-possesion (PoP),
/// which is also just a signature.
pub const DST_BLS_POP_IN_G2: &[u8] = b"BLS_POP_BLS12381G2_XMD:SHA-256_SSWU_RO_POP_";

#[derive(Clone, Eq, SerializeKey, DeserializeKey)]
/// A proof-of-possesion (PoP) of a BLS12381 private key.
/// This is just a BLS signature on the corresponding public key.
pub struct ProofOfPossession {
    pub(crate) pop: blst::min_pk::Signature,
}

impl ProofOfPossession {
    /// The length of a serialized ProofOfPossession struct.
    // NOTE: We have to hardcode this here because there is no library-defined constant
    pub const LENGTH: usize = 96;

    /// Serialize a ProofOfPossession.
    pub fn to_bytes(&self) -> [u8; Self::LENGTH] {
        self.pop.to_bytes()
    }

    /// Subgroup-check the PoP (i.e., verifies the PoP is a valid group element).
    ///
    /// WARNING: Subgroup-checking is done implicitly in `verify` below, so this function need not be called
    /// separately for most use-cases, as it incurs a performance penalty. We leave it here just in case.
    pub fn subgroup_check(&self) -> Result<()> {
        self.pop.validate(true).map_err(|e| anyhow!("{:?}", e))
    }

    /// Verifies the proof-of-possesion (PoP) of the private key corresponding to the specified
    /// BLS public key. Implicitly, subgroup checks the PoP and the specified public key, so
    /// the caller is not responsible for doing it manually.
    pub fn verify(&self, pk: &PublicKey) -> Result<()> {
        // CRYPTONOTE(Alin): We call the signature verification function with pk_validate set to true
        // since we do not necessarily trust the PK we deserialized over the network whose PoP we are
        // verifying here.
        let result = self.pop.verify(
            true,
            &pk.to_bytes(),
            DST_BLS_POP_IN_G2,
            &[],
            &pk.pubkey,
            true,
        );
        if result == BLST_ERROR::BLST_SUCCESS {
            Ok(())
        } else {
            Err(anyhow!(
                "Proof-of-possession (PoP) did NOT verify: {:?}",
                result
            ))
        }
    }

    /// Creates a proof-of-possesion (PoP) of the specified BLS private key. This function
    /// inefficiently recomputes the public key from the private key. To avoid this, the caller can
    /// use `create_with_pubkey` instead, which accepts the public key as a second input.
    pub fn create(sk: &PrivateKey) -> ProofOfPossession {
        // CRYPTONOTE(Alin): The standard does not detail how the PK should be serialized for hashing purposes; we just do the obvious.
        let pk = PublicKey {
            pubkey: sk.privkey.sk_to_pk(),
        };

        ProofOfPossession::create_with_pubkey(sk, &pk)
    }

    /// Creates a proof-of-possesion (PoP) of the specified BLS private key. Takes the
    /// corresponding public key as input, to avoid inefficiently recomputing it from the
    /// private key.
    ///
    /// WARNING: Does not subgroup-check the PK, since this function will be typically called on
    /// a freshly-generated key-pair or on a correctly-deserialized keypair.
    pub fn create_with_pubkey(sk: &PrivateKey, pk: &PublicKey) -> ProofOfPossession {
        // CRYPTONOTE(Alin): The standard does not detail how the PK should be serialized for hashing purposes; we just do the obvious.
        let pk_bytes = pk.to_bytes();

        // CRYPTONOTE(Alin): We hash with DST_BLS_POP_IN_G2 as per https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-bls-signature#section-4.2.3
        ProofOfPossession {
            pop: sk.privkey.sign(&pk_bytes, DST_BLS_POP_IN_G2, &[]),
        }
    }
}

//////////////////////////////
// ProofOfPossession Traits //
//////////////////////////////

impl ValidCryptoMaterial for ProofOfPossession {
    const AIP_80_PREFIX: &'static str = "bls12381-pop-";

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }
}
impl Length for ProofOfPossession {
    fn length(&self) -> usize {
        Self::LENGTH
    }
}

impl PartialEq for ProofOfPossession {
    fn eq(&self, other: &Self) -> bool {
        self.pop.to_bytes() == other.to_bytes()
    }
}

impl TryFrom<&[u8]> for ProofOfPossession {
    type Error = CryptoMaterialError;

    /// Deserializes a BLS PoP from a sequence of bytes.
    ///
    /// WARNING: Does NOT subgroup-check the PoP! This is done implicitly when verifying the PoP in
    /// `ProofOfPossession::verify`
    fn try_from(bytes: &[u8]) -> std::result::Result<ProofOfPossession, CryptoMaterialError> {
        Ok(Self {
            pop: blst::min_pk::Signature::from_bytes(bytes)
                .map_err(|_| CryptoMaterialError::DeserializationError)?,
        })
    }
}

impl std::hash::Hash for ProofOfPossession {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let encoded_signature = self.to_bytes();
        state.write(&encoded_signature);
    }
}

impl fmt::Debug for ProofOfPossession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.to_bytes()))
    }
}

impl fmt::Display for ProofOfPossession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.to_bytes()))
    }
}
