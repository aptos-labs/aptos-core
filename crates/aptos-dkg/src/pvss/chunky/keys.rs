// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{pvss::chunky::chunked_elgamal, traits, Scalar};
use aptos_crypto::{
    arkworks,
    arkworks::serialization::{ark_de, ark_se},
    CryptoMaterialError, Uniform, ValidCryptoMaterial,
};
use aptos_crypto_derive::{SilentDebug, SilentDisplay};
use ark_ec::{pairing::Pairing, CurveGroup};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use serde::{Deserialize, Serialize};
use std::ops::Mul;

/// The *encryption (public)* key used to encrypt shares of the dealt secret for each PVSS player.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct EncryptPubKey<E: Pairing> {
    /// A group element $H^{dk^{-1}} \in G_1$.
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub(crate) ek: E::G1Affine,
}

impl<E: Pairing> ValidCryptoMaterial for EncryptPubKey<E> {
    const AIP_80_PREFIX: &'static str = "";

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.ek.serialize_compressed(&mut bytes).unwrap();
        bytes
    }
}

impl<E: Pairing> TryFrom<&[u8]> for EncryptPubKey<E> {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let ek = <E::G1Affine as CanonicalDeserialize>::deserialize_compressed(bytes)
            .map_err(|_| CryptoMaterialError::DeserializationError)?;

        Ok(EncryptPubKey { ek })
    }
}

/// The *decryption (secret) key* used by each PVSS player to decrypt their share of the dealt secret.
#[derive(SilentDisplay, SilentDebug)]
pub struct DecryptPrivKey<E: Pairing> {
    /// A scalar $dk \in F$.
    pub(crate) dk: E::ScalarField,
}

impl<E: Pairing> Uniform for DecryptPrivKey<E> {
    fn generate<R>(rng: &mut R) -> Self
    where
        R: rand_core::RngCore + rand::Rng + rand_core::CryptoRng + rand::CryptoRng,
    {
        DecryptPrivKey::<E> {
            dk: arkworks::random::sample_field_element(rng),
        }
    }
}

impl<E: Pairing> traits::Convert<EncryptPubKey<E>, chunked_elgamal::PublicParameters<E>>
    for DecryptPrivKey<E>
{
    /// Given a decryption key $dk$, computes its associated encryption key $H^{dk}$
    fn to(&self, pp_elgamal: &chunked_elgamal::PublicParameters<E>) -> EncryptPubKey<E> {
        EncryptPubKey::<E> {
            ek: pp_elgamal.pubkey_base().mul(self.dk).into_affine(),
        }
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct DealtPubKey<E: Pairing> {
    /// A group element $G$ \in G_2$
    #[serde(serialize_with = "ark_se")]
    G: E::G2Affine,
}

#[allow(non_snake_case)]
impl<E: Pairing> DealtPubKey<E> {
    pub fn new(G: E::G2Affine) -> Self {
        Self { G }
    }

    pub fn as_g2(&self) -> E::G2Affine {
        self.G
    }
}



#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DealtPubKeyShare<E: Pairing>(pub(crate) DealtPubKey<E>); // TODO: Copied from `das`, but should review this at some point!!

impl<E: Pairing> DealtPubKeyShare<E> {
    pub fn new(dealt_pk: DealtPubKey<E>) -> Self {
        DealtPubKeyShare(dealt_pk)
    }

    pub fn as_g2(&self) -> E::G2Affine {
        self.0.as_g2()
    }
}

// TODO: maybe make these actual structs
#[allow(type_alias_bounds)]
pub type DealtSecretKey<E: Pairing> = Scalar<E>;
#[allow(type_alias_bounds)]
pub type DealtSecretKeyShare<E: Pairing> = Scalar<E>;
