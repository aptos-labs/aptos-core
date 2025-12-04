// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::pvss::traits::transcript::NonAggregatableTranscript;
use crate::traits::Transcript;
use aptos_crypto::bls12381;
use serde::Serialize;
use rand_core::CryptoRng;
use rand_core::RngCore;
use aptos_crypto::player::Player;
use aptos_crypto::SigningKey;
use serde::Deserialize;
use aptos_crypto_derive::CryptoHasher;
use aptos_crypto_derive::BCSCryptoHash;
use aptos_crypto::ValidCryptoMaterial;
use aptos_crypto::CryptoMaterialError;
use aptos_crypto::Uniform;
use crate::pvss::test_utils::NoAux;
use aptos_crypto::Signature;

/// A generic transformation from a non-malleable PVSS to a signed and non-malleable PVSS.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct GenericSigning<T> {
    trs: T,
    sig: bls12381::Signature
}

impl<T: Transcript> ValidCryptoMaterial for GenericSigning<T> {
    const AIP_80_PREFIX: &'static str = "";

    fn to_bytes(&self) -> Vec<u8> {
        // TODO: using `Result<Vec<u8>>` and `.map_err(|_| CryptoMaterialError::DeserializationError)` would be more consistent here?
        bcs::to_bytes(&self).expect("Unexpected error during PVSS transcript serialization")
    }
}

impl<T: Transcript> TryFrom<&[u8]> for GenericSigning<T> {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        bcs::from_bytes::<GenericSigning<T>>(bytes)
            .map_err(|_| CryptoMaterialError::DeserializationError)
    }
}

#[derive(Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct Contribution<T, S> {
    pub trs: T,
    pub sid: S, // the session id
}

/// Currently has requirements on the `SigningPubKey` and `SigningSecretKey`, in order 
/// to get a signature of type `bls12381::Signature`; this can be relaxed
impl<T: Transcript<SigningPubKey = bls12381::PublicKey, SigningSecretKey = bls12381::PrivateKey>> Transcript
    for GenericSigning<T>
{
    type DealtPubKey = T::DealtPubKey;
    type DealtPubKeyShare = T::DealtPubKeyShare;
    type DealtSecretKey = T::DealtSecretKey;
    type DealtSecretKeyShare = T::DealtSecretKeyShare;
    type DecryptPrivKey = T::DecryptPrivKey;
    type EncryptPubKey = T::EncryptPubKey;
    type InputSecret = T::InputSecret;
    type PublicParameters = T::PublicParameters;
    type SecretSharingConfig = T::SecretSharingConfig;
    type SigningPubKey = T::SigningPubKey;
    type SigningSecretKey = T::SigningSecretKey;

    fn dst() -> Vec<u8> {
        let mut result = b"SIGNED_".to_vec();
        result.extend(T::dst());
        result
    }

    fn scheme_name() -> String {
        format!("signed_{}", T::scheme_name())
    }

    fn deal<A: Serialize + Clone, R: rand_core::RngCore + rand_core::CryptoRng>(
        sc: &Self::SecretSharingConfig,
        pp: &Self::PublicParameters,
        ssk: &Self::SigningSecretKey,
        spk: &Self::SigningPubKey,
        eks: &[Self::EncryptPubKey],
        s: &Self::InputSecret,
        sid: &A,
        dealer: &Player,
        rng: &mut R,
    ) -> Self {

        let trs = T::deal(
                sc,
                pp,
                ssk,
                spk,
                eks,
                s,
                sid,
                dealer,
                rng,
            );

        // Sign the contribution
        let sig = ssk
            .sign(&Contribution {
                trs: trs.clone(), sid
            })
            .expect("signing of `chunky` PVSS transcript failed");

        GenericSigning {
            trs,
            sig
        }
    }

    fn get_dealers(&self) -> Vec<Player> {
        T::get_dealers(&self.trs)
    }

    fn get_public_key_share(
        &self,
        sc: &Self::SecretSharingConfig,
        player: &Player,
    ) -> Self::DealtPubKeyShare {
        T::get_public_key_share(&self.trs, sc, player)
    }

    fn get_dealt_public_key(&self) -> Self::DealtPubKey {
        T::get_dealt_public_key(&self.trs)
    }

    fn decrypt_own_share(
        &self,
        sc: &Self::SecretSharingConfig,
        player: &Player,
        dk: &Self::DecryptPrivKey,
        pp: &Self::PublicParameters,
    ) -> (Self::DealtSecretKeyShare, Self::DealtPubKeyShare) {
        T::decrypt_own_share(&self.trs, sc, player, dk, pp)
    }

    fn generate<R>(sc: &Self::SecretSharingConfig, pp: &Self::PublicParameters, rng: &mut R) -> Self
    where
        R: RngCore + CryptoRng,
    {
        let trs = T::generate(sc, pp, rng);

        let ssk = bls12381::PrivateKey::generate(rng);

        let sig = ssk
            .sign(&Contribution {
                trs: trs.clone(), sid: NoAux,
            })
            .expect("signing of PVSS transcript should have succeeded");

        GenericSigning {
            trs, sig,
        }
    }
}

impl<T: NonAggregatableTranscript<SigningPubKey = bls12381::PublicKey, SigningSecretKey = bls12381::PrivateKey>> NonAggregatableTranscript for GenericSigning<T> {
    fn verify<A: Serialize + Clone>(
        &self,
        sc: &Self::SecretSharingConfig,
        pp: &Self::PublicParameters,
        spks: &[Self::SigningPubKey],
        eks: &[Self::EncryptPubKey],
        sid: &A,
    ) -> anyhow::Result<()> {

        self.sig.verify(
            &Contribution {
                trs: self.trs.clone(), sid,
            },
            &spks[self.get_dealers()[0].id],
        )?;

        T::verify(
            &self.trs,
            sc,
            pp,
            spks,
            eks,
            sid,
        )
    }
}