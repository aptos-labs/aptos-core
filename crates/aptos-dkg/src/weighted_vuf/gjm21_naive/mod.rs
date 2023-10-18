//! # References
//! \[GJM+21\] Aggregatable Distributed Key Generation; by Kobi Gurkan and Philipp Jovanovic and Mary Maller and Sarah Meiklejohn and Gilad Stern and Alin Tomescu; in Cryptology ePrint Archive, Report 2021/005; 2021; https://eprint.iacr.org/2021/005

use crate::algebra::lagrange::lagrange_coefficients;
use crate::algebra::polynomials::get_powers_of_tau;
use crate::pvss;
use crate::pvss::traits::HasEncryptionPublicParams;
use crate::pvss::{Player, WeightedConfig};
use crate::utils::random::random_scalar;
use crate::utils::{g1_multi_exp, g2_multi_exp, multi_pairing};
use crate::weighted_vuf::traits::WeightedVUF;
use anyhow::bail;
use blstrs::{pairing, G1Projective, G2Projective, Gt, Scalar};
use ff::Field;
use group::{Curve, Group};
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use std::ops::{Add, Mul, Neg};

pub struct GjmNaiveWVUF;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Randomizers {
    alpha: Scalar,
    beta: Scalar,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedSKs {
    p_1: G1Projective,
    p_2: G1Projective,
    hat_p_1: G2Projective,
    hat_p_2: Vec<G2Projective>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicParameters {
    g_1: G1Projective,
    hat_h_1: G2Projective,
    hat_h_2: G2Projective,
    hat_h_3: G2Projective,
    hat_h_4: G2Projective,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AggregatedProof {
    pi_1: G1Projective,
    pi_2: G1Projective,
    p_1: G1Projective,
    p_2: G1Projective,
    hat_p_1: G2Projective,
    hat_p_2: G2Projective,
}

impl From<&pvss::scrape::PublicParameters> for PublicParameters {
    fn from(pp: &pvss::scrape::PublicParameters) -> Self {
        let g_1 = pp.get_commitment_base().clone();
        let hat_h_1 = pp.get_encryption_public_params().as_group_element().clone();
        let seed = pp.to_bytes();

        // TODO(Security): domain separators / DSTs
        let dst = b"ScrapeToGjm21WvufNaive";
        let hat_h_2 = G2Projective::hash_to_curve(seed.as_slice(), dst, b"hat_h_2");
        let hat_h_3 = G2Projective::hash_to_curve(seed.as_slice(), dst, b"hat_h_3");
        let hat_h_4 = G2Projective::hash_to_curve(seed.as_slice(), dst, b"hat_h_4");

        debug_assert_ne!(hat_h_2, hat_h_3);
        debug_assert_ne!(hat_h_2, hat_h_4);
        debug_assert_ne!(hat_h_3, hat_h_4);

        PublicParameters {
            g_1,
            hat_h_1,
            hat_h_2,
            hat_h_3,
            hat_h_4,
        }
    }
}

/// Implements a weighted variant of the \[GJM+21e\] VUF scheme, compatible with *any* PVSS scheme with the right kind
/// of secret key and public key.
impl WeightedVUF for GjmNaiveWVUF {
    type PublicParameters = PublicParameters;
    type PubKey = pvss::dealt_pub_key::g1::DealtPubKey;
    type SecretKey = pvss::dealt_secret_key::g2::DealtSecretKey;
    type PubKeyShare = Vec<pvss::dealt_pub_key_share::g1::DealtPubKeyShare>;
    type SecretKeyShare = Vec<pvss::dealt_secret_key_share::g2::DealtSecretKeyShare>;

    type Delta = EncryptedSKs;

    type AugmentedPubKeyShare = (EncryptedSKs, Self::PubKeyShare);
    type AugmentedSecretKeyShare = (Randomizers, Self::SecretKeyShare);

    type ProofShare = (G1Projective, G1Projective);
    type Proof = AggregatedProof;

    type Evaluation = Gt;

    fn augment_key_pair<R: rand_core::RngCore + rand_core::CryptoRng>(
        pp: &Self::PublicParameters,
        sk: Self::SecretKeyShare,
        pk: Self::PubKeyShare,
        rng: &mut R,
    ) -> (Self::AugmentedSecretKeyShare, Self::AugmentedPubKeyShare) {
        // TODO: ensure they are not zero, in case of bad RNG (should probably panic).
        let randomizers = Randomizers {
            alpha: random_scalar(rng),
            beta: random_scalar(rng),
        };

        let neg_alpha = randomizers.alpha.neg();
        let neg_beta = randomizers.beta.neg();
        let blinding_factor = pp.hat_h_3.mul(&neg_alpha).add(pp.hat_h_4.mul(&neg_beta));

        let enc_sks = EncryptedSKs {
            p_1: pp.g_1.mul(&randomizers.alpha),
            p_2: pp.g_1.mul(&randomizers.beta),
            // \hat{p}_1 = \hat{h}_1^-\alpha \hat{h}_2^-\beta
            hat_p_1: pp.hat_h_1.mul(&neg_alpha).add(pp.hat_h_2.mul(&neg_beta)),
            // \hat{p}_{2,j} = sk_j \hat{h}_3^-\alpha \hat{h}_4^-\beta
            hat_p_2: sk
                .iter()
                // TODO(Security): This is likely where the construction becomes insecure: reusing the same blinding factor
                .map(|sk| sk.as_group_element().add(blinding_factor))
                .collect::<Vec<G2Projective>>(),
        };

        ((randomizers, sk), (enc_sks, pk))
    }

    fn get_public_delta(apk: &Self::AugmentedPubKeyShare) -> &Self::Delta {
        let (enc_sks, _) = apk;

        enc_sks
    }

    fn augment_pubkey(
        pp: &Self::PublicParameters,
        pk: Self::PubKeyShare,
        // lpk: &Self::BlsPubKey,
        delta: Self::Delta,
    ) -> anyhow::Result<Self::AugmentedPubKeyShare> {
        if delta.hat_p_2.len() != pk.len() {
            bail!(
                "Expected PKs and encrypted SKs to be of the same length. Got {} and {}, respectively.",
                delta.hat_p_2.len(),
                pk.len()
            );
        }

        // TODO: Fiat-Shamir transform instead of RNG
        let r = random_scalar(&mut thread_rng());

        // Fetch this player's weight (denoted by W_i in the equations below)
        let n = pk.len();
        // Pick n + 1 random scalars,
        let mut rs = get_powers_of_tau(&r, n + 1);
        // Compute \sum_{j\in[W_i]} r_j
        let sum_of_rs = rs.iter().take(n).sum();

        //
        // Inputs for the multi-pairing:
        //
        //   e(g_1, \hatp_{i,1}^\gamma \prod_{j\in[W_i]} \hatp_{i,2,j}^{r_j})
        //   e(\p_{i,1}^{\sum_{j\in[W_i]} r_j}, \hat{h}_3)
        //   e(\p_{i,2}, \hat{h}_2^\gamma \cdot \hat{h}_4^{\sum_{j\in[W_i]} r_j})
        //   e(\p_{i,1}^{-\gamma} \cdot (\prod_{j\in[W_i]} \A_{i,j}^{r_j}), \hat{h}_1^{-1})
        //   = 1
        //

        // Computes $\hatp_{i,1}^\gamma \prod_{j\in[W_i]} \hatp_{i,2,j}^{r_j}$
        let rhs_1_bases = delta
            .hat_p_2
            .iter()
            .map(|e| e.clone())
            .chain([delta.hat_p_1])
            .collect::<Vec<G2Projective>>();
        let rhs_1 = g2_multi_exp(rhs_1_bases.as_slice(), rs.as_slice());

        // Computes $\p_{i,1}^{-\gamma} \cdot (\prod_{j\in[W_i]} \A_{i,j}^{r_j})$
        // Note: We need $-\gamma$ in the multiexp below, rather than $\gamma$.
        let lhs_4_bases = pk
            .iter()
            .map(|pk| pk.as_group_element().clone())
            .chain([delta.p_1])
            .collect::<Vec<G1Projective>>();
        let gamma = rs[n];
        rs[n] = gamma.neg();
        let lhs_4 = g1_multi_exp(lhs_4_bases.as_slice(), rs.as_slice());

        // Update `rs` by popping off the gamma scalar (must be done after this point; do NOT move higher up)
        rs.pop();

        // Computes $\hat{h}_2^\gamma \cdot \hat{h}_4^{\sum_{j\in[W_i]} r_j}$
        let rhs_3_bases = vec![pp.hat_h_2, pp.hat_h_4];
        let rhs_3 = g2_multi_exp(rhs_3_bases.as_slice(), vec![gamma, sum_of_rs].as_slice());

        // Computes $\p_{i,1}^{\sum_{j\in[W_i]} r_j}$
        let lhs_2 = delta.p_1.mul(sum_of_rs);

        let lhs = vec![pp.g_1, lhs_2, delta.p_2, lhs_4];
        let rhs = vec![rhs_1, pp.hat_h_3, rhs_3, pp.hat_h_1.neg()];

        let result = multi_pairing(lhs.iter(), rhs.iter());

        if result != Gt::identity() {
            bail!("Failed to verify the PK delta");
        }

        Ok((delta, pk))
    }

    fn create_share(ask: &Self::AugmentedSecretKeyShare, msg: &[u8]) -> Self::ProofShare {
        let (r, _) = ask;

        let hash = Self::hash_to_curve(msg);

        (hash.mul(&r.alpha), hash.mul(&r.beta))
    }

    fn verify_share(
        pp: &Self::PublicParameters,
        apk: &Self::AugmentedPubKeyShare,
        msg: &[u8],
        proof: &Self::ProofShare,
    ) -> anyhow::Result<()> {
        let delta = Self::get_public_delta(apk);
        let hash = Self::hash_to_curve(msg);
        let lhs = [hash, proof.0, proof.1];
        let rhs = [delta.hat_p_1, pp.hat_h_1, pp.hat_h_2];

        let result = multi_pairing(lhs.iter(), rhs.iter());

        if result != Gt::identity() {
            bail!("Failed to verify VUF proof share on message");
        }

        Ok(())
    }

    fn aggregate_shares(
        wc: &WeightedConfig,
        apks_and_proofs: &[(Player, Self::AugmentedPubKeyShare, Self::ProofShare)],
    ) -> Self::Proof {
        // Collect all the evaluation points associated with each player's augmented pubkey sub shares.
        let mut sub_player_ids = Vec::with_capacity(wc.get_total_weight());

        for (i, apk_share, _) in apks_and_proofs {
            for j in 0..apk_share.1.len() {
                sub_player_ids.push(wc.get_virtual_player(i, j).id);
            }
        }

        // Compute the Lagrange coefficients associated with those evaluation points
        let batch_dom = wc.get_batch_evaluation_domain();
        let lagr = lagrange_coefficients(batch_dom, &sub_player_ids[..], &Scalar::ZERO);

        // Interpolate the WVUF Proof
        let mut k = 0;
        let mut pi_1_bases = Vec::with_capacity(apks_and_proofs.len());
        let mut pi_2_bases = Vec::with_capacity(apks_and_proofs.len());
        let mut p_1_bases = Vec::with_capacity(apks_and_proofs.len());
        let mut p_2_bases = Vec::with_capacity(apks_and_proofs.len());
        let mut hat_p_1_bases = Vec::with_capacity(apks_and_proofs.len());
        let mut hat_p_2_bases = Vec::with_capacity(apks_and_proofs.len());
        let mut summed_exps = Vec::with_capacity(apks_and_proofs.len());
        for (_, apk_share, proof) in apks_and_proofs {
            // println!(
            //     "Flattening {} share(s) for player {player}",
            //     sub_shares.len()
            // );
            let num_shares = apk_share.1.len();
            let lagr_sum = lagr[k..k + num_shares].iter().sum();
            summed_exps.push(lagr_sum);

            pi_1_bases.push(proof.0);
            pi_2_bases.push(proof.1);

            let delta = &apk_share.0;
            p_1_bases.push(delta.p_1);
            p_2_bases.push(delta.p_2);
            hat_p_1_bases.push(delta.hat_p_1);

            for hat_p_2 in &delta.hat_p_2 {
                hat_p_2_bases.push(hat_p_2.clone());
            }

            k += num_shares;
        }

        let pi_1 = g1_multi_exp(pi_1_bases.as_slice(), summed_exps.as_slice());
        let pi_2 = g1_multi_exp(pi_2_bases.as_slice(), summed_exps.as_slice());
        let p_1 = g1_multi_exp(p_1_bases.as_slice(), summed_exps.as_slice());
        let p_2 = g1_multi_exp(p_2_bases.as_slice(), summed_exps.as_slice());
        let hat_p_1 = g2_multi_exp(hat_p_1_bases.as_slice(), summed_exps.as_slice());
        let hat_p_2 = g2_multi_exp(hat_p_2_bases.as_slice(), lagr.as_slice());

        Self::Proof {
            pi_1,
            pi_2,
            p_1,
            p_2,
            hat_p_1,
            hat_p_2,
        }
    }

    fn eval(sk: &Self::SecretKey, msg: &[u8]) -> Self::Evaluation {
        let hash = Self::hash_to_curve(msg).to_affine();

        pairing(&hash, &sk.as_group_element().to_affine())
    }

    // NOTE: This VUF has the same evaluation as its proof
    fn derive_eval(
        pp: &Self::PublicParameters,
        msg: &[u8],
        proof: &Self::Proof,
    ) -> Self::Evaluation {
        let hash = Self::hash_to_curve(msg);

        let lhs = [hash, proof.pi_1, proof.pi_2];
        let rhs = [proof.hat_p_2, pp.hat_h_3, pp.hat_h_4];

        multi_pairing(lhs.iter(), rhs.iter())
    }

    /// Used for testing only.
    fn create_proof(_sk: &Self::SecretKey, _msg: &[u8]) -> Self::Proof {
        // TODO: impl
        Self::Proof {
            pi_1: G1Projective::identity(),
            pi_2: G1Projective::identity(),
            p_1: G1Projective::identity(),
            p_2: G1Projective::identity(),
            hat_p_1: G2Projective::identity(),
            hat_p_2: G2Projective::identity(),
        }
    }

    fn verify_eval(
        pp: &Self::PublicParameters,
        pk: &Self::PubKey,
        msg: &[u8],
        proof: &Self::Proof,
        _eval: &Self::Evaluation,
    ) -> anyhow::Result<()> {
        let delta = EncryptedSKs {
            p_1: proof.p_1,
            p_2: proof.p_2,
            hat_p_1: proof.hat_p_1,
            hat_p_2: vec![proof.hat_p_2], // just one element
        };

        let pk_share = vec![pvss::dealt_pub_key_share::g1::DealtPubKeyShare::new(
            pk.clone(),
        )];
        let apk = Self::augment_pubkey(pp, pk_share, delta)?;

        let proof_share = (proof.pi_1, proof.pi_2);
        Self::verify_share(pp, &apk, msg, &proof_share)
    }
}

impl GjmNaiveWVUF {
    fn hash_to_curve(msg: &[u8]) -> G1Projective {
        // TODO: add DST and aug
        let dst = b"none";
        let aug = b"none";
        let hash = blstrs::G1Projective::hash_to_curve(msg, &dst[..], &aug[..]);
        hash
    }
}
