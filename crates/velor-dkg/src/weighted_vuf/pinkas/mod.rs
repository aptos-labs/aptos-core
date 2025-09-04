// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    algebra::{lagrange::lagrange_coefficients, polynomials::get_powers_of_tau},
    pvss,
    pvss::{
        dealt_pub_key_share::g2::DealtPubKeyShare, traits::HasEncryptionPublicParams, Player,
        WeightedConfig,
    },
    utils::{
        g1_multi_exp, g2_multi_exp, multi_pairing, parallel_multi_pairing,
        random::{random_nonzero_scalar, random_scalar},
    },
    weighted_vuf::traits::WeightedVUF,
};
use anyhow::{anyhow, bail};
use blstrs::{pairing, G1Projective, G2Projective, Gt, Scalar};
use ff::Field;
use group::{Curve, Group};
use rand::thread_rng;
use rayon::{
    iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator},
    ThreadPool,
};
use serde::{Deserialize, Serialize};
use std::ops::{Mul, Neg, Range};

pub const PINKAS_WVUF_DST: &[u8; 21] = b"VELOR_PINKAS_WVUF_DST";

// For the worst-case (higher number of players, with fewer shares each), setting to 1 or 4 is not good, so using 2.
pub const MIN_MULTIEXP_NUM_JOBS: usize = 2;

// TODO: Getting this choice to be right might be tricky.
//  Anything between 2 and 5 seems to give 2.5 ms for size-50 batch.
pub const MIN_MULTIPAIR_NUM_JOBS: usize = 4;

pub struct PinkasWUF;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandomizedPKs {
    pi: G1Projective,       // \hat{g}^{r}
    rks: Vec<G1Projective>, // g^{r \sk_i}, for all shares i
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicParameters {
    g: G1Projective,
    g_neg: G1Projective,
    g_hat: G2Projective,
}

impl From<&pvss::das::PublicParameters> for PublicParameters {
    fn from(pp: &pvss::das::PublicParameters) -> Self {
        let g = pp.get_encryption_public_params().message_base().clone();
        PublicParameters {
            g,
            g_neg: g.neg(),
            g_hat: pp.get_commitment_base().clone(),
        }
    }
}

/// Implements the Pinkas weighted VUF scheme, compatible with *any* PVSS scheme with the right kind
/// of secret key and public key.
impl WeightedVUF for PinkasWUF {
    type AugmentedPubKeyShare = (RandomizedPKs, Self::PubKeyShare);
    type AugmentedSecretKeyShare = (Scalar, Self::SecretKeyShare);
    // /// Note: Our BLS PKs are currently in G_1.
    // type BlsPubKey = bls12381::PublicKey;
    // type BlsSecretKey = bls12381::PrivateKey;

    type Delta = RandomizedPKs;
    type Evaluation = Gt;
    /// Naive aggregation by concatenation. It is an open problem to get constant-sized aggregation.
    type Proof = Vec<(Player, Self::ProofShare)>;
    type ProofShare = G2Projective;
    type PubKey = pvss::dealt_pub_key::g2::DealtPubKey;
    type PubKeyShare = Vec<pvss::dealt_pub_key_share::g2::DealtPubKeyShare>;
    type PublicParameters = PublicParameters;
    type SecretKey = pvss::dealt_secret_key::g1::DealtSecretKey;
    type SecretKeyShare = Vec<pvss::dealt_secret_key_share::g1::DealtSecretKeyShare>;

    fn augment_key_pair<R: rand_core::RngCore + rand_core::CryptoRng>(
        pp: &Self::PublicParameters,
        sk: Self::SecretKeyShare,
        pk: Self::PubKeyShare,
        // lsk: &Self::BlsSecretKey,
        rng: &mut R,
    ) -> (Self::AugmentedSecretKeyShare, Self::AugmentedPubKeyShare) {
        let r = random_nonzero_scalar(rng);

        let rpks = RandomizedPKs {
            pi: pp.g.mul(&r),
            rks: sk
                .iter()
                .map(|sk| sk.as_group_element().mul(&r))
                .collect::<Vec<G1Projective>>(),
        };

        ((r.invert().unwrap(), sk), (rpks, pk))
    }

    fn get_public_delta(apk: &Self::AugmentedPubKeyShare) -> &Self::Delta {
        let (rpks, _) = apk;

        rpks
    }

    fn augment_pubkey(
        pp: &Self::PublicParameters,
        pk: Self::PubKeyShare,
        // lpk: &Self::BlsPubKey,
        delta: Self::Delta,
    ) -> anyhow::Result<Self::AugmentedPubKeyShare> {
        if delta.rks.len() != pk.len() {
            bail!(
                "Expected PKs and RKs to be of the same length. Got {} and {}, respectively.",
                delta.rks.len(),
                pk.len()
            );
        }

        // TODO: Fiat-Shamir transform instead of RNG
        let tau = random_scalar(&mut thread_rng());

        let pks = pk
            .iter()
            .map(|pk| *pk.as_group_element())
            .collect::<Vec<G2Projective>>();
        let taus = get_powers_of_tau(&tau, pks.len());

        let pks_combined = g2_multi_exp(&pks[..], &taus[..]);
        let rks_combined = g1_multi_exp(&delta.rks[..], &taus[..]);

        if multi_pairing(
            [&delta.pi, &rks_combined].into_iter(),
            [&pks_combined, &pp.g_hat.neg()].into_iter(),
        ) != Gt::identity()
        {
            panic!("RPKs were not correctly randomized.");
        }

        Ok((delta, pk))
    }

    fn create_share(ask: &Self::AugmentedSecretKeyShare, msg: &[u8]) -> Self::ProofShare {
        let (r_inv, _) = ask;

        let hash = Self::hash_to_curve(msg);

        hash.mul(r_inv)
    }

    fn verify_share(
        pp: &Self::PublicParameters,
        apk: &Self::AugmentedPubKeyShare,
        msg: &[u8],
        proof: &Self::ProofShare,
    ) -> anyhow::Result<()> {
        let delta = Self::get_public_delta(apk);

        let h = Self::hash_to_curve(msg);

        if multi_pairing([&delta.pi, &pp.g_neg].into_iter(), [proof, &h].into_iter())
            != Gt::identity()
        {
            bail!("PinkasWVUF ProofShare failed to verify.");
        }

        Ok(())
    }

    fn aggregate_shares(
        _wc: &WeightedConfig,
        apks_and_proofs: &[(Player, Self::AugmentedPubKeyShare, Self::ProofShare)],
    ) -> Self::Proof {
        let mut players_and_shares = Vec::with_capacity(apks_and_proofs.len());

        for (p, _, share) in apks_and_proofs {
            players_and_shares.push((p.clone(), share.clone()));
        }

        players_and_shares
    }

    fn eval(sk: &Self::SecretKey, msg: &[u8]) -> Self::Evaluation {
        let h = Self::hash_to_curve(msg).to_affine();

        pairing(&sk.as_group_element().to_affine(), &h)
    }

    // NOTE: This VUF has the same evaluation as its proof.
    fn derive_eval(
        wc: &WeightedConfig,
        _pp: &Self::PublicParameters,
        _msg: &[u8],
        apks: &[Option<Self::AugmentedPubKeyShare>],
        proof: &Self::Proof,
        thread_pool: &ThreadPool,
    ) -> anyhow::Result<Self::Evaluation> {
        let (rhs, rks, lagr, ranges) =
            Self::collect_lagrange_coeffs_shares_and_rks(wc, apks, proof)?;

        // Compute the RK multiexps in parallel
        let lhs = Self::rk_multiexps(proof, rks, &lagr, &ranges, thread_pool);

        // Interpolate the WVUF evaluation in parallel
        Ok(Self::multi_pairing(lhs, rhs, thread_pool))
    }

    /// Verifies the proof shares (using batch verification)
    fn verify_proof(
        pp: &Self::PublicParameters,
        _pk: &Self::PubKey,
        apks: &[Option<Self::AugmentedPubKeyShare>],
        msg: &[u8],
        proof: &Self::Proof,
    ) -> anyhow::Result<()> {
        if proof.len() >= apks.len() {
            bail!("Number of proof shares ({}) exceeds number of APKs ({}) when verifying aggregated WVUF proof", proof.len(), apks.len());
        }

        // TODO: Fiat-Shamir transform instead of RNG
        let tau = random_scalar(&mut thread_rng());
        let taus = get_powers_of_tau(&tau, proof.len());

        // [share_i^{\tau^i}]_{i \in [0, n)}
        let shares = proof
            .iter()
            .map(|(_, share)| share)
            .zip(taus.iter())
            .map(|(share, tau)| share.mul(tau))
            .collect::<Vec<G2Projective>>();

        let mut pis = Vec::with_capacity(proof.len());
        for (player, _) in proof {
            if player.id >= apks.len() {
                bail!(
                    "Player index {} falls outside APK vector of length {}",
                    player.id,
                    apks.len()
                );
            }

            pis.push(
                apks[player.id]
                    .as_ref()
                    .ok_or_else(|| anyhow!("Missing APK for player {}", player.get_id()))?
                    .0
                    .pi,
            );
        }

        let h = Self::hash_to_curve(msg);
        let sum_of_taus: Scalar = taus.iter().sum();

        if multi_pairing(
            pis.iter().chain([pp.g_neg].iter()),
            shares.iter().chain([h.mul(sum_of_taus)].iter()),
        ) != Gt::identity()
        {
            bail!("Multipairing check in batched aggregate verification failed");
        }

        Ok(())
    }
}

impl PinkasWUF {
    fn hash_to_curve(msg: &[u8]) -> G2Projective {
        G2Projective::hash_to_curve(msg, &PINKAS_WVUF_DST[..], b"H(m)")
    }

    pub fn collect_lagrange_coeffs_shares_and_rks<'a>(
        wc: &WeightedConfig,
        apks: &'a [Option<(RandomizedPKs, Vec<DealtPubKeyShare>)>],
        proof: &'a Vec<(Player, <Self as WeightedVUF>::ProofShare)>,
    ) -> anyhow::Result<(
        Vec<&'a G2Projective>,
        Vec<&'a Vec<G1Projective>>,
        Vec<Scalar>,
        Vec<Range<usize>>,
    )> {
        // Collect all the evaluation points associated with each player's augmented pubkey sub shares.
        let mut sub_player_ids = Vec::with_capacity(wc.get_total_weight());
        // The G2 shares
        let mut shares = Vec::with_capacity(proof.len());
        // The RKs of each player
        let mut rks = Vec::with_capacity(proof.len());
        // The starting & ending index of each player in the `lagr` coefficients vector
        let mut ranges = Vec::with_capacity(proof.len());

        let mut k = 0;
        for (player, share) in proof {
            for j in 0..wc.get_player_weight(player) {
                sub_player_ids.push(wc.get_virtual_player(player, j).id);
            }

            let apk = apks[player.id]
                .as_ref()
                .ok_or_else(|| anyhow!("Missing APK for player {}", player.get_id()))?;

            rks.push(&apk.0.rks);
            shares.push(share);

            let w = wc.get_player_weight(player);
            ranges.push(k..k + w);
            k += w;
        }

        // Compute the Lagrange coefficients associated with those evaluation points
        let batch_dom = wc.get_batch_evaluation_domain();
        let lagr = lagrange_coefficients(batch_dom, &sub_player_ids[..], &Scalar::ZERO);
        Ok((shares, rks, lagr, ranges))
    }

    pub fn rk_multiexps(
        proof: &Vec<(Player, G2Projective)>,
        rks: Vec<&Vec<G1Projective>>,
        lagr: &Vec<Scalar>,
        ranges: &Vec<Range<usize>>,
        thread_pool: &ThreadPool,
    ) -> Vec<G1Projective> {
        thread_pool.install(|| {
            proof
                .par_iter()
                .with_min_len(MIN_MULTIEXP_NUM_JOBS)
                .enumerate()
                .map(|(idx, _)| {
                    let rks = rks[idx];
                    let lagr = &lagr[ranges[idx].clone()];
                    g1_multi_exp(rks, lagr)
                })
                .collect::<Vec<G1Projective>>()
        })
    }

    pub fn multi_pairing(
        lhs: Vec<G1Projective>,
        rhs: Vec<&G2Projective>,
        thread_pool: &ThreadPool,
    ) -> Gt {
        parallel_multi_pairing(
            lhs.iter().map(|r| r),
            rhs.into_iter(),
            thread_pool,
            MIN_MULTIPAIR_NUM_JOBS,
        )
    }
}
