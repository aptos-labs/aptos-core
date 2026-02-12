// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    pvss::chunky::{chunked_elgamal::decrypt_chunked_scalars, keys, PublicParameters},
    traits::{transcript::Aggregated, Aggregatable, TranscriptCore},
    Scalar,
};
use aptos_crypto::{
    arkworks::serialization::{ark_de, ark_se},
    player::Player,
    weighted_config::WeightedConfigArkworks,
    CryptoMaterialError, TSecretSharingConfig, ValidCryptoMaterial,
};
use ark_ec::{pairing::Pairing, CurveGroup};
use ark_ff::{Fp, FpConfig};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use serde::{Deserialize, Serialize};

// TODO: not sure we need both CanonicalSerialize and Serialize here?
#[allow(non_snake_case)]
#[derive(
    CanonicalSerialize, CanonicalDeserialize, Serialize, Deserialize, Clone, Debug, PartialEq, Eq,
)]
pub struct Subtranscript<P: Pairing> {
    // The dealt public key
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub V0: P::G2Affine,
    // The dealt public key shares
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub Vs: Vec<Vec<P::G2Affine>>,
    /// First chunked ElGamal component: C[i][j] = s_{i,j} * G + r_j * ek_i. Here s_i = \sum_j s_{i,j} * B^j // TODO: change notation because B is not a group element?
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub Cs: Vec<Vec<Vec<P::G1Affine>>>,
    /// Second chunked ElGamal component: R[j] = r_j * H
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub Rs: Vec<Vec<P::G1Affine>>,
}

// There doesn't seem to be a need to serialize or deserialize this
#[allow(non_snake_case)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubtranscriptProjective<P: Pairing> {
    // The dealt public key (in projective form)
    pub V0_proj: P::G2,
    // The dealt public key shares (in projective form)
    pub Vs_proj: Vec<Vec<P::G2>>,
    /// First chunked ElGamal component (in projective form)
    pub Cs_proj: Vec<Vec<Vec<P::G1>>>,
    /// Second chunked ElGamal component (in projective form)
    pub Rs_proj: Vec<Vec<P::G1>>,
}

impl<P: Pairing> ValidCryptoMaterial for Subtranscript<P> {
    const AIP_80_PREFIX: &'static str = "";

    fn to_bytes(&self) -> Vec<u8> {
        bcs::to_bytes(&self).expect("Unexpected error during PVSS transcript serialization")
    }
}

impl<P: Pairing> TryFrom<&[u8]> for Subtranscript<P> {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        bcs::from_bytes::<Subtranscript<P>>(bytes)
            .map_err(|_| CryptoMaterialError::DeserializationError)
    }
}

impl<E: Pairing> Aggregatable for Subtranscript<E> {
    type Aggregated = SubtranscriptProjective<E>;
    type SecretSharingConfig = WeightedConfigArkworks<E::ScalarField>;

    fn to_aggregated(&self) -> Self::Aggregated {
        SubtranscriptProjective {
            V0_proj: self.V0.into(),
            Vs_proj: self
                .Vs
                .iter()
                .map(|row| row.iter().map(|x| (*x).into()).collect())
                .collect(),
            Cs_proj: self
                .Cs
                .iter()
                .map(|i| {
                    i.iter()
                        .map(|j| j.iter().map(|x| (*x).into()).collect())
                        .collect()
                })
                .collect(),
            Rs_proj: self
                .Rs
                .iter()
                .map(|row| row.iter().map(|x| (*x).into()).collect())
                .collect(),
        }
    }
}

impl<E: Pairing> Aggregated<Subtranscript<E>> for SubtranscriptProjective<E> {
    #[allow(non_snake_case)]
    fn aggregate_with(
        &mut self,
        sc: &WeightedConfigArkworks<E::ScalarField>,
        other: &Subtranscript<E>,
    ) -> anyhow::Result<()> {
        debug_assert_eq!(self.Cs_proj.len(), sc.get_total_num_players());
        debug_assert_eq!(self.Vs_proj.len(), sc.get_total_num_players());
        debug_assert_eq!(self.Cs_proj.len(), other.Cs.len());
        debug_assert_eq!(self.Rs_proj.len(), other.Rs.len());
        debug_assert_eq!(self.Vs_proj.len(), other.Vs.len());

        // Aggregate the V0s
        self.V0_proj += other.V0;

        // Aggregate Vs (nested) element-wise
        for i in 0..self.Vs_proj.len() {
            debug_assert_eq!(self.Vs_proj[i].len(), other.Vs[i].len());
            for j in 0..self.Vs_proj[i].len() {
                // Aggregate the V_{i,j}s
                self.Vs_proj[i][j] += other.Vs[i][j];
            }
        }

        for i in 0..sc.get_total_num_players() {
            for j in 0..self.Cs_proj[i].len() {
                for k in 0..self.Cs_proj[i][j].len() {
                    // Aggregate the C_{i,j,k}s
                    self.Cs_proj[i][j][k] += other.Cs[i][j][k];
                }
            }
        }

        for j in 0..self.Rs_proj.len() {
            for (R_jk, other_R_jk) in self.Rs_proj[j].iter_mut().zip(&other.Rs[j]) {
                // Aggregate the R_{j,k}s
                *R_jk += *other_R_jk;
            }
        }

        Ok(())
    }

    fn normalize(self) -> Subtranscript<E> {
        // Collect all G1 elements (from Cs and Rs)
        let mut g1_elems = Vec::new();
        for player_cs in &self.Cs_proj {
            for chunks in player_cs {
                g1_elems.extend(chunks.iter().copied());
            }
        }
        for weight_rs in &self.Rs_proj {
            g1_elems.extend(weight_rs.iter().copied());
        }

        // Collect all G2 elements (from V0 and Vs)
        let mut g2_elems = vec![self.V0_proj];
        for row in &self.Vs_proj {
            g2_elems.extend(row.iter().copied());
        }

        // Batch normalize
        let g1_affine = E::G1::normalize_batch(&g1_elems);
        let g2_affine = E::G2::normalize_batch(&g2_elems);

        // Reconstruct nested structures in affine form
        let mut g1_iter = g1_affine.into_iter();
        let mut g2_iter = g2_affine.into_iter();

        let result = Subtranscript {
            V0: g2_iter.next().unwrap(),
            Vs: self
                .Vs_proj
                .iter()
                .map(|row| row.iter().map(|_| g2_iter.next().unwrap()).collect())
                .collect(),
            Cs: self
                .Cs_proj
                .iter()
                .map(|mat| {
                    mat.iter()
                        .map(|row| row.iter().map(|_| g1_iter.next().unwrap()).collect())
                        .collect()
                })
                .collect(),
            Rs: self
                .Rs_proj
                .iter()
                .map(|row| row.iter().map(|_| g1_iter.next().unwrap()).collect())
                .collect(),
        };
        debug_assert!(g1_iter.next().is_none());
        debug_assert!(g2_iter.next().is_none());
        result
    }
}

impl<const N: usize, P: FpConfig<N>, E: Pairing<ScalarField = Fp<P, N>>> TranscriptCore
    for Subtranscript<E>
{
    type DealtPubKey = keys::DealtPubKey<E>;
    type DealtPubKeyShare = Vec<keys::DealtPubKeyShare<E>>;
    type DealtSecretKey = keys::DealtSecretKey<E::ScalarField>;
    type DealtSecretKeyShare = Vec<keys::DealtSecretKeyShare<E::ScalarField>>;
    type DecryptPrivKey = keys::DecryptPrivKey<E>;
    type EncryptPubKey = keys::EncryptPubKey<E>;
    type PublicParameters = PublicParameters<E>;
    type SecretSharingConfig = WeightedConfigArkworks<E::ScalarField>;

    #[allow(non_snake_case)]
    fn get_public_key_share(
        &self,
        _sc: &Self::SecretSharingConfig,
        player: &Player,
    ) -> Self::DealtPubKeyShare {
        self.Vs[player.id]
            .iter()
            .map(|&V_i| keys::DealtPubKeyShare::<E>::new(keys::DealtPubKey::new(V_i)))
            .collect()
    }

    fn get_dealt_public_key(&self) -> Self::DealtPubKey {
        Self::DealtPubKey::new(self.V0)
    }

    #[allow(non_snake_case)]
    fn decrypt_own_share(
        &self,
        sc: &Self::SecretSharingConfig,
        player: &Player,
        dk: &Self::DecryptPrivKey,
        pp: &Self::PublicParameters,
    ) -> (Self::DealtSecretKeyShare, Self::DealtPubKeyShare) {
        let Cs = &self.Cs[player.id];
        debug_assert_eq!(Cs.len(), sc.get_player_weight(player));

        if !Cs.is_empty() {
            if let Some(first_key) = self.Rs.first() {
                debug_assert_eq!(
                    first_key.len(),
                    Cs[0].len(),
                    "Number of ephemeral keys does not match the number of ciphertext chunks"
                );
            }
        }

        let pk_shares = self.get_public_key_share(sc, player);

        let sk_shares: Vec<_> = decrypt_chunked_scalars(
            &Cs,
            &self.Rs,
            &dk.dk,
            &pp.pp_elgamal,
            &pp.dlog_table,
            pp.get_dlog_range_bound(),
            pp.ell,
        );

        (
            Scalar::vec_from_inner(sk_shares),
            pk_shares, // TODO: review this formalism... why do we need this here?
        )
    }
}
