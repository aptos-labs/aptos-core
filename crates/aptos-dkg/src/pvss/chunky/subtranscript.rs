// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Subtranscript type for weighted chunky PVSS: the aggregatable core of a transcript.
//!
//! A subtranscript holds the dealt public key `V0`, per-player share commitments `Vs`, chunked
//! ElGamal ciphertexts `Cs`, and ephemeral components `Rs`. It implements [TranscriptCore] and
//! [Aggregatable] so it can be verified and aggregated (via [SubtranscriptProjective]).

use crate::{
    pvss::chunky::{chunked_elgamal::decrypt_chunked_scalars, keys, PublicParameters},
    traits::{transcript::Aggregated, Aggregatable, TranscriptCore},
    Scalar,
};
use aptos_crypto::{
    arkworks::{
        random::{unsafe_random_point, unsafe_random_points},
        serialization::{ark_de, ark_se},
    },
    player::Player,
    weighted_config::WeightedConfigArkworks,
    CryptoMaterialError, TSecretSharingConfig, ValidCryptoMaterial,
};
use ark_ec::{pairing::Pairing, CurveGroup};
use ark_ff::{Fp, FpConfig};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use rand_core::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use std::iter::repeat_with;

// Serialize/Deserialize are required for ValidCryptoMaterial (BCS). CanonicalSerialize/
// CanonicalDeserialize are used by the transcripts to automatically derive serialization, since
// Pairing types do not implement serde.
#[allow(non_snake_case)]
#[derive(
    CanonicalSerialize, CanonicalDeserialize, Serialize, Deserialize, Clone, Debug, PartialEq, Eq,
)]
pub struct Subtranscript<E: Pairing> {
    // The dealt public key
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub V0: E::G2Affine,
    // The dealt public key shares
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub Vs: Vec<Vec<E::G2Affine>>,
    /// First chunked ElGamal component: C[i][j] = s_{i,j} * G + r_j * ek_i. Here s_i = \sum_j s_{i,j} * B^j // TODO: change notation because B is not a group element? maybe β or radix?
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub Cs: Vec<Vec<Vec<E::G1Affine>>>,
    /// Second chunked ElGamal component: R[j] = r_j * H
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub Rs: Vec<Vec<E::G1Affine>>,
}

impl<E: Pairing> ValidCryptoMaterial for Subtranscript<E> {
    const AIP_80_PREFIX: &'static str = "";

    fn to_bytes(&self) -> Vec<u8> {
        bcs::to_bytes(&self)
            .expect("Unexpected error during chunky PVSS subtranscript serialization")
    }
}

impl<E: Pairing> TryFrom<&[u8]> for Subtranscript<E> {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        bcs::from_bytes::<Subtranscript<E>>(bytes)
            .map_err(|_| CryptoMaterialError::DeserializationError)
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
            pk_shares, // TODO: Trait requires returning pk_shares for verification and VRF use?
        )
    }
}

// In contrast to Subtranscript, there is no need to serialize or deserialize this;
// it is only used as an intermediate type during aggregation by validators.
#[allow(non_snake_case)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubtranscriptProjective<E: Pairing> {
    // The dealt public key (in projective form)
    pub V0_proj: E::G2,
    // The dealt public key shares (in projective form)
    pub Vs_proj: Vec<Vec<E::G2>>,
    /// First chunked ElGamal component (in projective form)
    pub Cs_proj: Vec<Vec<Vec<E::G1>>>,
    /// Second chunked ElGamal component (in projective form)
    pub Rs_proj: Vec<Vec<E::G1>>,
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
        for (vs_row, other_row) in self.Vs_proj.iter_mut().zip(&other.Vs) {
            debug_assert_eq!(vs_row.len(), other_row.len());
            for (v_ij, other_v_ij) in vs_row.iter_mut().zip(other_row) {
                *v_ij += *other_v_ij;
            }
        }

        // Aggregate Cs (nested) element-wise
        for (cs_player, other_player) in self.Cs_proj.iter_mut().zip(&other.Cs) {
            for (cs_chunks, other_chunks) in cs_player.iter_mut().zip(other_player) {
                for (c_ijk, other_c_ijk) in cs_chunks.iter_mut().zip(other_chunks) {
                    *c_ijk += *other_c_ijk;
                }
            }
        }

        // Aggregate Rs element-wise
        for (rs_row, other_row) in self.Rs_proj.iter_mut().zip(&other.Rs) {
            for (r_jk, other_r_jk) in rs_row.iter_mut().zip(other_row) {
                *r_jk += *other_r_jk;
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

impl<E: Pairing> Subtranscript<E> {
    /// Generates a subtranscript with random (V0, Vs, Cs, Rs) for use in `generate()` of
    /// weighted chunky transcripts. `Vs` is built as `sc.group_by_player(&Vs_flat)` so the
    /// layout matches the rest of the codebase.
    ///
    /// Would make this part of the Subtranscript trait, but we'd be computing `num_chunks_per_share` twice
    /// inside the larger transcripts
    #[allow(non_snake_case)]
    pub fn generate<R: RngCore + CryptoRng>(
        sc: &WeightedConfigArkworks<E::ScalarField>,
        num_chunks_per_share: usize,
        rng: &mut R,
    ) -> Self {
        let V0 = unsafe_random_point::<E::G2, _>(rng);
        let Vs_flat = unsafe_random_points::<E::G2, _>(sc.get_total_weight(), rng);
        let Vs = sc.group_by_player(&Vs_flat);

        let Cs: Vec<Vec<Vec<E::G1Affine>>> = (0..sc.get_total_num_players())
            .map(|i| {
                let player = sc.get_player(i);
                let w = sc.get_player_weight(&player);
                repeat_with(|| unsafe_random_points::<E::G1, _>(num_chunks_per_share, rng))
                    .take(w)
                    .collect()
            })
            .collect();

        let Rs: Vec<Vec<E::G1Affine>> =
            repeat_with(|| unsafe_random_points::<E::G1, _>(num_chunks_per_share, rng))
                .take(sc.get_max_weight())
                .collect();

        Self { V0, Vs, Cs, Rs }
    }

    /// Builds the vector used as input to the SCRAPE low-degree test: all share
    /// commitments in player/share order, then the dealt public key `V0`.
    #[allow(non_snake_case)]
    pub fn all_Vs_flat(&self) -> Vec<E::G2Affine> {
        let mut Vs: Vec<E::G2Affine> = self.Vs.iter().flatten().copied().collect();
        Vs.push(self.V0);
        Vs
    }
}
