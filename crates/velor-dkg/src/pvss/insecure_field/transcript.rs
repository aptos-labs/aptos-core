// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    algebra::polynomials::shamir_secret_share,
    pvss,
    pvss::{
        das, encryption_dlog, traits,
        traits::{transcript::MalleableTranscript, Convert, SecretSharingConfig},
        Player, ThresholdConfig,
    },
    utils::{
        random::{insecure_random_g2_points, random_scalars},
        HasMultiExp,
    },
};
use anyhow::bail;
use velor_crypto::{bls12381, CryptoMaterialError, ValidCryptoMaterial};
use velor_crypto_derive::{BCSCryptoHash, CryptoHasher};
use blstrs::{G2Projective, Scalar};
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use std::ops::Mul;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, BCSCryptoHash, CryptoHasher)]
#[allow(non_snake_case)]
pub struct Transcript {
    dealers: Vec<Player>,
    /// Public key shares from 0 to n-1, public key is in V[n]
    V: Vec<G2Projective>,
    /// Secret key shares
    C: Vec<Scalar>,
}

impl ValidCryptoMaterial for Transcript {
    const AIP_80_PREFIX: &'static str = "";

    fn to_bytes(&self) -> Vec<u8> {
        bcs::to_bytes(&self).expect("unexpected error during PVSS transcript serialization")
    }
}

impl TryFrom<&[u8]> for Transcript {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        bcs::from_bytes::<Transcript>(bytes).map_err(|_| CryptoMaterialError::DeserializationError)
    }
}

impl Convert<Scalar, das::PublicParameters> for pvss::input_secret::InputSecret {
    fn to(&self, _with: &das::PublicParameters) -> Scalar {
        *self.get_secret_a()
    }
}

impl traits::Transcript for Transcript {
    type DealtPubKey = pvss::dealt_pub_key::g2::DealtPubKey;
    type DealtPubKeyShare = pvss::dealt_pub_key_share::g2::DealtPubKeyShare;
    type DealtSecretKey = Scalar;
    type DealtSecretKeyShare = Scalar;
    type DecryptPrivKey = encryption_dlog::g1::DecryptPrivKey;
    type EncryptPubKey = encryption_dlog::g1::EncryptPubKey;
    type InputSecret = pvss::input_secret::InputSecret;
    type PublicParameters = das::PublicParameters;
    type SecretSharingConfig = ThresholdConfig;
    type SigningPubKey = bls12381::PublicKey;
    type SigningSecretKey = bls12381::PrivateKey;

    fn scheme_name() -> String {
        "insecure_field_pvss".to_string()
    }

    #[allow(non_snake_case)]
    fn deal<A: Serialize + Clone, R: rand_core::RngCore + rand_core::CryptoRng>(
        sc: &Self::SecretSharingConfig,
        pp: &Self::PublicParameters,
        _ssk: &Self::SigningSecretKey,
        eks: &Vec<Self::EncryptPubKey>,
        s: &Self::InputSecret,
        _aux: &A,
        dealer: &Player,
        rng: &mut R,
    ) -> Self {
        assert_eq!(eks.len(), sc.n);

        let (f, C) = shamir_secret_share(sc, s, rng);

        let g_2 = pp.get_commitment_base();

        let V = (0..sc.n)
            .map(|i| g_2.mul(C[i]))
            .chain([g_2.mul(f[0])])
            .collect::<Vec<G2Projective>>();

        debug_assert_eq!(V.len(), sc.n + 1);
        debug_assert_eq!(C.len(), sc.n);

        Transcript {
            dealers: vec![*dealer],
            V,
            C,
        }
    }

    fn verify<A: Serialize + Clone>(
        &self,
        sc: &Self::SecretSharingConfig,
        pp: &Self::PublicParameters,
        _spks: &Vec<Self::SigningPubKey>,
        eks: &Vec<Self::EncryptPubKey>,
        _aux: &Vec<A>,
    ) -> anyhow::Result<()> {
        if eks.len() != sc.n {
            bail!("Expected {} encryption keys, but got {}", sc.n, eks.len());
        }

        if self.C.len() != sc.n {
            bail!("Expected {} ciphertexts, but got {}", sc.n, self.C.len());
        }

        if self.V.len() != sc.n + 1 {
            bail!(
                "Expected {} (polynomial) commitment elements, but got {}",
                sc.n + 1,
                self.V.len()
            );
        }

        let alphas = random_scalars(sc.n, &mut thread_rng());
        let g_2 = pp.get_commitment_base();

        let lc_1 = g_2.mul(
            self.C
                .iter()
                .zip(alphas.iter())
                .map(|(&c, &alpha)| c * alpha)
                .sum::<Scalar>(),
        );
        let lc_2 = G2Projective::multi_exp_iter(self.V.iter().take(sc.n), alphas.iter());

        if lc_1 != lc_2 {
            bail!("Expected linear combination check test to pass")
        }

        return Ok(());
    }

    fn get_dealers(&self) -> Vec<Player> {
        self.dealers.clone()
    }

    fn aggregate_with(&mut self, sc: &Self::SecretSharingConfig, other: &Transcript) {
        debug_assert_eq!(self.C.len(), sc.n);
        debug_assert_eq!(self.V.len(), sc.n + 1);

        for i in 0..sc.n {
            self.C[i] += other.C[i];
            self.V[i] += other.V[i];
        }
        self.V[sc.n] += other.V[sc.n];
        self.dealers.extend_from_slice(other.dealers.as_slice());

        debug_assert_eq!(self.C.len(), other.C.len());
        debug_assert_eq!(self.V.len(), other.V.len());
    }

    fn get_public_key_share(
        &self,
        _sc: &Self::SecretSharingConfig,
        player: &Player,
    ) -> Self::DealtPubKeyShare {
        Self::DealtPubKeyShare::new(Self::DealtPubKey::new(self.V[player.id]))
    }

    fn get_dealt_public_key(&self) -> Self::DealtPubKey {
        Self::DealtPubKey::new(*self.V.last().unwrap())
    }

    fn decrypt_own_share(
        &self,
        sc: &Self::SecretSharingConfig,
        player: &Player,
        _dk: &Self::DecryptPrivKey,
    ) -> (Self::DealtSecretKeyShare, Self::DealtPubKeyShare) {
        (self.C[player.id], self.get_public_key_share(sc, player))
    }

    #[allow(non_snake_case)]
    fn generate<R>(sc: &Self::SecretSharingConfig, rng: &mut R) -> Self
    where
        R: rand_core::RngCore + rand_core::CryptoRng,
    {
        Transcript {
            dealers: vec![sc.get_player(0)],
            V: insecure_random_g2_points(sc.n + 1, rng),
            C: random_scalars(sc.n, rng),
        }
    }
}

impl MalleableTranscript for Transcript {
    fn maul_signature<A: Serialize + Clone>(
        &mut self,
        _ssk: &Self::SigningSecretKey,
        _aux: &A,
        player: &Player,
    ) {
        self.dealers = vec![*player];
    }
}
