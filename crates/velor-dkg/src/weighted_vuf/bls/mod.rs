// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    algebra::{lagrange::lagrange_coefficients, polynomials::get_powers_of_tau},
    pvss,
    pvss::{Player, WeightedConfig},
    utils::{g1_multi_exp, multi_pairing, random::random_scalar, HasMultiExp},
    weighted_vuf::traits::WeightedVUF,
};
use anyhow::bail;
use blstrs::{G1Projective, G2Projective, Gt, Scalar};
use ff::Field;
use group::Group;
use rand::thread_rng;
use rayon::ThreadPool;
use serde::{Deserialize, Serialize};
use std::ops::{Mul, Neg};

pub const BLS_WVUF_DST: &[u8; 18] = b"VELOR_BLS_WVUF_DST";

pub struct BlsWUF;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicParameters {
    g: G2Projective,
}

impl From<&pvss::das::PublicParameters> for PublicParameters {
    fn from(pp: &pvss::das::PublicParameters) -> Self {
        PublicParameters {
            g: *pp.get_commitment_base(),
        }
    }
}

impl WeightedVUF for BlsWUF {
    type AugmentedPubKeyShare = Self::PubKeyShare;
    type AugmentedSecretKeyShare = Self::SecretKeyShare;
    type Delta = ();
    type Evaluation = G1Projective;
    type Proof = Self::Evaluation;
    type ProofShare = Vec<G1Projective>;
    type PubKey = pvss::dealt_pub_key::g2::DealtPubKey;
    type PubKeyShare = Vec<pvss::dealt_pub_key_share::g2::DealtPubKeyShare>;
    type PublicParameters = PublicParameters;
    type SecretKey = Scalar;
    type SecretKeyShare = Vec<Scalar>;

    fn augment_key_pair<R: rand_core::RngCore + rand_core::CryptoRng>(
        _pp: &Self::PublicParameters,
        sk: Self::SecretKeyShare,
        pk: Self::PubKeyShare,
        _rng: &mut R,
    ) -> (Self::AugmentedSecretKeyShare, Self::AugmentedPubKeyShare) {
        (sk, pk)
    }

    fn get_public_delta(_apk: &Self::AugmentedPubKeyShare) -> &Self::Delta {
        &()
    }

    fn augment_pubkey(
        _pp: &Self::PublicParameters,
        pk: Self::PubKeyShare,
        _delta: Self::Delta,
    ) -> anyhow::Result<Self::AugmentedPubKeyShare> {
        Ok(pk)
    }

    fn create_share(ask: &Self::AugmentedSecretKeyShare, msg: &[u8]) -> Self::ProofShare {
        let hash = Self::hash_to_curve(msg);

        ask.iter()
            .map(|sk| hash.mul(sk))
            .collect::<Vec<G1Projective>>()
    }

    fn verify_share(
        pp: &Self::PublicParameters,
        apk: &Self::AugmentedPubKeyShare,
        msg: &[u8],
        proof: &Self::ProofShare,
    ) -> anyhow::Result<()> {
        let hash = Self::hash_to_curve(msg);
        // TODO: Use Fiat-Shamir instead of random_scalar
        let coeffs = get_powers_of_tau(&random_scalar(&mut thread_rng()), apk.len());

        let pks = apk
            .iter()
            .map(|pk| *pk.as_group_element())
            .collect::<Vec<G2Projective>>();
        // TODO: Calling multi-exp seems to decrease performance by 100+ microseconds even when |coeffs| = 1 and the coefficient is 1. Not sure what's going on here.
        let agg_pk = G2Projective::multi_exp_slice(pks.as_slice(), coeffs.as_slice());
        let agg_sig = G1Projective::multi_exp_slice(proof.to_vec().as_slice(), coeffs.as_slice());

        if multi_pairing(
            [&hash, &agg_sig].into_iter(),
            [&agg_pk, &pp.g.neg()].into_iter(),
        ) != Gt::identity()
        {
            bail!("BlsWVUF ProofShare failed to verify.");
        }

        Ok(())
    }

    fn aggregate_shares(
        wc: &WeightedConfig,
        apks_and_proofs: &[(Player, Self::AugmentedPubKeyShare, Self::ProofShare)],
    ) -> Self::Proof {
        // Collect all the evaluation points associated with each player
        let mut sub_player_ids = Vec::with_capacity(wc.get_total_weight());

        for (player, _, _) in apks_and_proofs {
            for j in 0..wc.get_player_weight(player) {
                sub_player_ids.push(wc.get_virtual_player(player, j).id);
            }
        }

        // Compute the Lagrange coefficients associated with those evaluation points
        let batch_dom = wc.get_batch_evaluation_domain();
        let lagr = lagrange_coefficients(batch_dom, &sub_player_ids[..], &Scalar::ZERO);

        // Interpolate the signature
        let mut bases = Vec::with_capacity(apks_and_proofs.len());
        for (_, _, share) in apks_and_proofs {
            // println!(
            //     "Flattening {} share(s) for player {player}",
            //     sub_shares.len()
            // );
            bases.extend_from_slice(share.as_slice())
        }

        g1_multi_exp(bases.as_slice(), lagr.as_slice())
    }

    fn eval(sk: &Self::SecretKey, msg: &[u8]) -> Self::Evaluation {
        let h = Self::hash_to_curve(msg);
        h.mul(sk)
    }

    // NOTE: This VUF has the same evaluation as its proof.
    fn derive_eval(
        _wc: &WeightedConfig,
        _pp: &Self::PublicParameters,
        _msg: &[u8],
        _apks: &[Option<Self::AugmentedPubKeyShare>],
        proof: &Self::Proof,
        _thread_pool: &ThreadPool,
    ) -> anyhow::Result<Self::Evaluation> {
        Ok(*proof)
    }

    /// Verifies the proof shares one by one
    fn verify_proof(
        pp: &Self::PublicParameters,
        pk: &Self::PubKey,
        _apks: &[Option<Self::AugmentedPubKeyShare>],
        msg: &[u8],
        proof: &Self::Proof,
    ) -> anyhow::Result<()> {
        let hash = Self::hash_to_curve(msg);

        if multi_pairing(
            [&hash, proof].into_iter(),
            [pk.as_group_element(), &pp.g.neg()].into_iter(),
        ) != Gt::identity()
        {
            bail!("BlsWVUF Proof failed to verify.");
        }

        Ok(())
    }
}

impl BlsWUF {
    fn hash_to_curve(msg: &[u8]) -> G1Projective {
        G1Projective::hash_to_curve(msg, BLS_WVUF_DST, b"H(m)")
    }
}
