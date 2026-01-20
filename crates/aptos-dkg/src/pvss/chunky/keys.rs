// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{pvss::chunky::chunked_elgamal_pp, traits, Scalar};
use aptos_crypto::{
    arkworks,
    arkworks::serialization::{ark_de, ark_se},
    CryptoMaterialError, Uniform, ValidCryptoMaterial,
};
use aptos_crypto_derive::{SilentDebug, SilentDisplay};
use ark_ec::{pairing::Pairing, CurveGroup};
use ark_ff::PrimeField;
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

impl From<&aptos_crypto::bls12381::PublicKey> for EncryptPubKey<ark_bls12_381::Bls12_381> {
    fn from(value: &aptos_crypto::bls12381::PublicKey) -> Self {
        Self {
            // I believe this unwrap is safe, because value should always serialize to a valid
            // bls12-381 curve point.
            ek: <ark_bls12_381::Bls12_381 as ark_ec::pairing::Pairing>::G1Affine::deserialize_compressed(&value.to_bytes()[..]).unwrap()
        }
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

impl<E: Pairing> traits::Convert<EncryptPubKey<E>, chunked_elgamal_pp::PublicParameters<E::G1>>
    for DecryptPrivKey<E>
{
    /// Given a decryption key $dk$, computes its associated encryption key $H^{dk}$
    fn to(&self, pp_elgamal: &chunked_elgamal_pp::PublicParameters<E::G1>) -> EncryptPubKey<E> {
        EncryptPubKey::<E> {
            ek: pp_elgamal.pubkey_base().mul(self.dk).into_affine(),
        }
    }
}

impl From<&aptos_crypto::bls12381::PrivateKey> for DecryptPrivKey<ark_bls12_381::Bls12_381> {
    fn from(value: &aptos_crypto::bls12381::PrivateKey) -> Self {
        Self {
            dk: <ark_bls12_381::Bls12_381 as ark_ec::pairing::Pairing>::ScalarField::from_be_bytes_mod_order(&value.to_bytes())
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
pub type DealtSecretKey<F: PrimeField> = Scalar<F>;
#[allow(type_alias_bounds)]
pub type DealtSecretKeyShare<F: PrimeField> = Scalar<F>;

#[cfg(test)]
mod tests {
    use super::{DecryptPrivKey, EncryptPubKey};
    use crate::pvss::{chunky::chunked_elgamal_pp::PublicParameters, traits::Convert};
    use aptos_crypto::{
        bls12381::{PrivateKey, PublicKey},
        Uniform,
    };
    use ark_bls12_381::Bls12_381;
    use rand::thread_rng;

    #[test]
    fn test_conversion_from_blst_types() {
        let mut rng = thread_rng();
        let sk: PrivateKey = PrivateKey::generate(&mut rng);
        let pk: PublicKey = PublicKey::from(&sk);

        let decryption_key: DecryptPrivKey<Bls12_381> = DecryptPrivKey::from(&sk);
        let encryption_key_from_decryption_key: EncryptPubKey<Bls12_381> =
            decryption_key.to(&PublicParameters::new(3));

        let encryption_key_from_blst_pk = EncryptPubKey::from(&pk);

        assert_eq!(
            encryption_key_from_decryption_key,
            encryption_key_from_blst_pk
        );
    }
}
