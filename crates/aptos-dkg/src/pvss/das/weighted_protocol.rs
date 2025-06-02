// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    algebra::polynomials::shamir_secret_share,
    pvss,
    pvss::{
        contribution::{batch_verify_soks, Contribution, SoK},
        das, encryption_dlog, fiat_shamir, schnorr, traits,
        traits::{transcript::MalleableTranscript, HasEncryptionPublicParams, SecretSharingConfig},
        LowDegreeTest, Player, WeightedConfig,
    },
    utils::{
        g1_multi_exp, g2_multi_exp, multi_pairing,
        random::{
            insecure_random_g1_points, insecure_random_g2_points, random_g1_point, random_scalar,
            random_scalars,
        },
        HasMultiExp,
    },
};
use anyhow::bail;
use aptos_crypto::{bls12381, CryptoMaterialError, Genesis, SigningKey, ValidCryptoMaterial};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use blstrs::{pairing, G1Affine, G1Projective, G2Affine, G2Projective, Gt};
use group::{Curve, Group};
use serde::{Deserialize, Serialize};
use std::ops::{Add, Mul, Neg, Sub};

/// Scheme name
pub const WEIGHTED_DAS_SK_IN_G1: &'static str = "provable_weighted_das_sk_in_g1";

/// Domain-separator tag (DST) for the Fiat-Shamir hashing used to derive randomness from the transcript.
const DAS_WEIGHTED_PVSS_FIAT_SHAMIR_DST: &[u8; 48] =
    b"APTOS_DAS_WEIGHTED_PROVABLY_PVSS_FIAT_SHAMIR_DST";

/// A weighted transcript where the max player weight is $M$.
/// Each player has weight $w_i$ and the threshold weight is $w$.
/// The total weight is $W = \sum_{i=1}^n w_i$.
/// Let $s_i = \sum_{j = 1}^{i-1} w_i$.
/// Player $i$ will own shares $p(s_i), p(s_i + 1), \ldots, p(s_i + j - 1)$ in the degree-$w$
/// polynomial $p(X)$.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, BCSCryptoHash, CryptoHasher)]
#[allow(non_snake_case)]
pub struct Transcript {
    /// Proofs-of-knowledge (PoKs) for the dealt secret committed in $c = g_2^{p(0)}$.
    /// Since the transcript could have been aggregated from other transcripts with their own
    /// committed secrets in $c_i = g_2^{p_i(0)}$, this is a vector of PoKs for all these $c_i$'s
    /// such that $\prod_i c_i = c$.
    ///
    /// Also contains BLS signatures from each player $i$ on that player's contribution $c_i$, the
    /// player ID $i$ and auxiliary information `aux[i]` provided during dealing.
    soks: Vec<SoK<G1Projective>>,
    /// Commitment to encryption randomness $g_1^{r_j} \in G_1, \forall j \in [W]$
    R: Vec<G1Projective>,
    /// Same as $R$ except uses $g_2$.
    R_hat: Vec<G2Projective>,
    /// First $W$ elements are commitments to the evaluations of $p(X)$: $g_1^{p(\omega^i)}$,
    /// where $i \in [W]$. Last element is $g_1^{p(0)}$ (i.e., the dealt public key).
    V: Vec<G1Projective>,
    /// Same as $V$ except uses $g_2$.
    V_hat: Vec<G2Projective>,
    /// ElGamal encryption of the $j$th share of player $i$:
    /// i.e., $C[s_i+j-1] = h_1^{p(\omega^{s_i + j - 1})} ek_i^{r_j}, \forall i \in [n], j \in [w_i]$.
    /// We sometimes denote $C[s_i+j-1]$ by C_{i, j}.
    C: Vec<G1Projective>,
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
        // NOTE: The `serde` implementation in `blstrs` already performs the necessary point validation
        // by ultimately calling `GroupEncoding::from_bytes`.
        bcs::from_bytes::<Transcript>(bytes).map_err(|_| CryptoMaterialError::DeserializationError)
    }
}

impl traits::Transcript for Transcript {
    type DealtPubKey = pvss::dealt_pub_key::g2::DealtPubKey;
    type DealtPubKeyShare = Vec<pvss::dealt_pub_key_share::g2::DealtPubKeyShare>;
    type DealtSecretKey = pvss::dealt_secret_key::g1::DealtSecretKey;
    type DealtSecretKeyShare = Vec<pvss::dealt_secret_key_share::g1::DealtSecretKeyShare>;
    type DecryptPrivKey = encryption_dlog::g1::DecryptPrivKey;
    type EncryptPubKey = encryption_dlog::g1::EncryptPubKey;
    type InputSecret = pvss::input_secret::InputSecret;
    type PublicParameters = das::PublicParameters;
    type SecretSharingConfig = WeightedConfig;
    type SigningPubKey = bls12381::PublicKey;
    type SigningSecretKey = bls12381::PrivateKey;

    fn scheme_name() -> String {
        WEIGHTED_DAS_SK_IN_G1.to_string()
    }

    #[allow(non_snake_case)]
    fn deal<A: Serialize + Clone, R: rand_core::RngCore + rand_core::CryptoRng>(
        sc: &Self::SecretSharingConfig,
        pp: &Self::PublicParameters,
        ssk: &Self::SigningSecretKey,
        eks: &Vec<Self::EncryptPubKey>,
        s: &Self::InputSecret,
        aux: &A,
        dealer: &Player,
        mut rng: &mut R,
    ) -> Self {
        let n = sc.get_total_num_players();
        assert_eq!(eks.len(), n);

        // f_evals[k] = f(\omega^k), \forall k \in [0, W-1]
        let W = sc.get_total_weight();
        let (f_coeff, f_evals) = shamir_secret_share(sc.get_threshold_config(), s, rng);
        assert_eq!(f_coeff.len(), sc.get_threshold_weight());
        assert_eq!(f_evals.len(), W);

        // Pick ElGamal randomness r_j, \forall j \in [W]
        // r[j] = r_{j+1}, \forall j \in [0, W-1]
        let r = random_scalars(W, &mut rng);
        let g_1 = pp.get_encryption_public_params().pubkey_base();
        let g_2 = pp.get_commitment_base();
        let h = *pp.get_encryption_public_params().message_base();

        // NOTE: Recall s_i is the starting index of player i in the vector of shares
        //  - V[s_i + j - 1] = g_2^{f(s_i + j - 1)}
        //  - V[W] = g_2^{f(0)}
        let V = (0..W)
            .map(|k| g_1.mul(f_evals[k]))
            .chain([g_1.mul(f_coeff[0])])
            .collect::<Vec<G1Projective>>();
        let V_hat = (0..W)
            .map(|k| g_2.mul(f_evals[k]))
            .chain([g_2.mul(f_coeff[0])])
            .collect::<Vec<G2Projective>>();

        // R[j] = g_1^{r_{j + 1}},  \forall j \in [0, W-1]
        let R = (0..W).map(|j| g_1.mul(r[j])).collect::<Vec<G1Projective>>();
        let R_hat = (0..W).map(|j| g_2.mul(r[j])).collect::<Vec<G2Projective>>();

        let mut C = Vec::with_capacity(W);
        for i in 0..n {
            let w_i = sc.get_player_weight(&sc.get_player(i));

            let bases = vec![h, Into::<G1Projective>::into(&eks[i])];
            for j in 0..w_i {
                let k = sc.get_share_index(i, j).unwrap();

                C.push(g1_multi_exp(
                    bases.as_slice(),
                    [f_evals[k], r[k]].as_slice(),
                ))
            }
        }

        // Compute PoK of input secret committed in V[n]
        let pok = schnorr::pok_prove(&f_coeff[0], g_1, &V[W], rng);

        // Sign the secret commitment, player ID and `aux`
        let sig = Self::sign_contribution(ssk, dealer, aux, &V[W]);

        let t = Transcript {
            soks: vec![(*dealer, V[W], sig, pok)],
            V,
            R_hat,
            R,
            V_hat,
            C,
        };
        debug_assert!(t.check_sizes(sc).is_ok());
        t
    }

    #[allow(non_snake_case)]
    fn verify<A: Serialize + Clone>(
        &self,
        sc: &Self::SecretSharingConfig,
        pp: &Self::PublicParameters,
        spks: &Vec<Self::SigningPubKey>,
        eks: &Vec<Self::EncryptPubKey>,
        auxs: &Vec<A>,
    ) -> anyhow::Result<()> {
        self.check_sizes(sc)?;
        let n = sc.get_total_num_players();
        if eks.len() != n {
            bail!("Expected {} encryption keys, but got {}", n, eks.len());
        }
        let W = sc.get_total_weight();

        // Derive challenges deterministically via Fiat-Shamir; easier to debug for distributed systems
        let (f, extra) = fiat_shamir::fiat_shamir(
            self,
            sc.get_threshold_config(),
            pp,
            spks,
            eks,
            auxs,
            &DAS_WEIGHTED_PVSS_FIAT_SHAMIR_DST[..],
            2 + W * 3, // 3W+1 for encryption check, 1 for SoK verification.
        );

        let sok_vrfy_challenge = &extra[W * 3 + 1];
        let g_2 = pp.get_commitment_base();
        let g_1 = pp.get_encryption_public_params().pubkey_base();
        batch_verify_soks::<G1Projective, A>(
            self.soks.as_slice(),
            g_1,
            &self.V[W],
            spks,
            auxs,
            sok_vrfy_challenge,
        )?;

        let ldt = LowDegreeTest::new(
            f,
            sc.get_threshold_weight(),
            W + 1,
            true,
            sc.get_batch_evaluation_domain(),
        )?;
        ldt.low_degree_test_on_g1(&self.V)?;

        //
        // Correctness of encryptions check
        //

        let alphas_betas_and_gammas = &extra[0..W * 3 + 1];
        let (alphas_and_betas, gammas) = alphas_betas_and_gammas.split_at(2 * W + 1);
        let (alphas, betas) = alphas_and_betas.split_at(W + 1);
        assert_eq!(alphas.len(), W + 1);
        assert_eq!(betas.len(), W);
        assert_eq!(gammas.len(), W);

        let lc_VR_hat = G2Projective::multi_exp_iter(
            self.V_hat.iter().chain(self.R_hat.iter()),
            alphas_and_betas.iter(),
        );
        let lc_VRC = G1Projective::multi_exp_iter(
            self.V.iter().chain(self.R.iter()).chain(self.C.iter()),
            alphas_betas_and_gammas.iter(),
        );
        let lc_V_hat = G2Projective::multi_exp_iter(self.V_hat.iter().take(W), gammas.iter());
        let mut lc_R_hat = Vec::with_capacity(n);

        for i in 0..n {
            let p = sc.get_player(i);
            let weight = sc.get_player_weight(&p);
            let s_i = sc.get_player_starting_index(&p);

            lc_R_hat.push(g2_multi_exp(
                &self.R_hat[s_i..s_i + weight],
                &gammas[s_i..s_i + weight],
            ));
        }

        let h = pp.get_encryption_public_params().message_base();
        let g_2_neg = g_2.neg();
        let eks = eks
            .iter()
            .map(Into::<G1Projective>::into)
            .collect::<Vec<G1Projective>>();
        // The vector of left-hand-side ($\mathbb{G}_2$) inputs to each pairing in the multi-pairing.
        let lhs = [g_1, &lc_VRC, h].into_iter().chain(&eks);
        // The vector of right-hand-side ($\mathbb{G}_2$) inputs to each pairing in the multi-pairing.
        let rhs = [&lc_VR_hat, &g_2_neg, &lc_V_hat]
            .into_iter()
            .chain(&lc_R_hat);

        let res = multi_pairing(lhs, rhs);
        if res != Gt::identity() {
            bail!(
                "Expected zero during multi-pairing check for {} {}, but got {}",
                sc,
                Self::scheme_name(),
                res
            );
        }

        return Ok(());
    }

    fn get_dealers(&self) -> Vec<Player> {
        self.soks
            .iter()
            .map(|(p, _, _, _)| *p)
            .collect::<Vec<Player>>()
    }

    #[allow(non_snake_case)]
    fn aggregate_with(&mut self, sc: &Self::SecretSharingConfig, other: &Transcript) {
        let W = sc.get_total_weight();

        debug_assert!(self.check_sizes(sc).is_ok());
        debug_assert!(other.check_sizes(sc).is_ok());

        for i in 0..self.V.len() {
            self.V[i] += other.V[i];
            self.V_hat[i] += other.V_hat[i];
        }

        for i in 0..W {
            self.R[i] += other.R[i];
            self.R_hat[i] += other.R_hat[i];
            self.C[i] += other.C[i];
        }

        for sok in &other.soks {
            self.soks.push(sok.clone());
        }
    }

    fn get_public_key_share(
        &self,
        sc: &Self::SecretSharingConfig,
        player: &Player,
    ) -> Self::DealtPubKeyShare {
        let weight = sc.get_player_weight(player);
        let mut pk_shares = Vec::with_capacity(weight);

        for j in 0..weight {
            let k = sc.get_share_index(player.id, j).unwrap();
            pk_shares.push(pvss::dealt_pub_key_share::g2::DealtPubKeyShare::new(
                Self::DealtPubKey::new(self.V_hat[k]),
            ));
        }

        pk_shares
    }

    fn get_dealt_public_key(&self) -> Self::DealtPubKey {
        Self::DealtPubKey::new(*self.V_hat.last().unwrap())
    }

    #[allow(non_snake_case)]
    fn decrypt_own_share(
        &self,
        sc: &Self::SecretSharingConfig,
        player: &Player,
        dk: &Self::DecryptPrivKey,
    ) -> (Self::DealtSecretKeyShare, Self::DealtPubKeyShare) {
        let weight = sc.get_player_weight(player);
        let mut sk_shares = Vec::with_capacity(weight);
        let pk_shares = self.get_public_key_share(sc, player);

        for j in 0..weight {
            let k = sc.get_share_index(player.id, j).unwrap();

            let ctxt = self.C[k]; // h_1^{f(s_i + j - 1)} \ek_i^{r_{s_i + j}}
            let ephemeral_key = self.R[k].mul(dk.dk); // (g_1^{r_{s_i + j}})
            let dealt_secret_key_share = ctxt.sub(ephemeral_key);

            sk_shares.push(pvss::dealt_secret_key_share::g1::DealtSecretKeyShare::new(
                Self::DealtSecretKey::new(dealt_secret_key_share),
            ));
        }

        (sk_shares, pk_shares)
    }

    #[allow(non_snake_case)]
    fn generate<R>(sc: &Self::SecretSharingConfig, rng: &mut R) -> Self
    where
        R: rand_core::RngCore + rand_core::CryptoRng,
    {
        let W = sc.get_total_weight();
        let sk = bls12381::PrivateKey::genesis();
        Transcript {
            soks: vec![(
                sc.get_player(0),
                random_g1_point(rng),
                sk.sign(&Contribution::<G1Projective, usize> {
                    comm: random_g1_point(rng),
                    player: sc.get_player(0),
                    aux: 0,
                })
                .unwrap(),
                (random_g1_point(rng), random_scalar(rng)),
            )],
            R: insecure_random_g1_points(W, rng),
            R_hat: insecure_random_g2_points(W, rng),
            V: insecure_random_g1_points(W + 1, rng),
            V_hat: insecure_random_g2_points(W + 1, rng),
            C: insecure_random_g1_points(W, rng),
        }
    }
}

impl Transcript {
    #[allow(non_snake_case)]
    fn check_sizes(&self, sc: &WeightedConfig) -> anyhow::Result<()> {
        let W = sc.get_total_weight();

        if self.V.len() != W + 1 {
            bail!(
                "Expected {} G_2 (polynomial) commitment elements, but got {}",
                W + 1,
                self.V.len()
            );
        }

        if self.V_hat.len() != W + 1 {
            bail!(
                "Expected {} G_2 (polynomial) commitment elements, but got {}",
                W + 1,
                self.V_hat.len()
            );
        }

        if self.R.len() != W {
            bail!(
                "Expected {} G_1 commitment(s) to ElGamal randomness, but got {}",
                W,
                self.R.len()
            );
        }

        if self.R_hat.len() != W {
            bail!(
                "Expected {} G_2 commitment(s) to ElGamal randomness, but got {}",
                W,
                self.R_hat.len()
            );
        }

        if self.C.len() != W {
            bail!("Expected C of length {}, but got {}", W, self.C.len());
        }

        Ok(())
    }

    /// For testing.
    #[allow(non_snake_case, unused)]
    fn slow_verify(
        &self,
        sc: &WeightedConfig,
        pp: &das::PublicParameters,
        eks: &Vec<encryption_dlog::g1::EncryptPubKey>,
    ) -> anyhow::Result<()> {
        let n = sc.get_total_num_players();
        let g_2 = pp.get_commitment_base();
        let g_1 = pp.get_encryption_public_params().pubkey_base();
        let h_1 = pp.get_encryption_public_params().message_base();
        let W = sc.get_total_weight();

        let g_1_aff = g_1.to_affine();
        let g_2_aff = g_2.to_affine();
        let V_hat_aff = self
            .V_hat
            .iter()
            .map(|p| p.to_affine())
            .collect::<Vec<G2Affine>>();
        for i in 0..W + 1 {
            let lhs = pairing(&g_1_aff, &V_hat_aff[i]);
            let rhs = pairing(&self.V[i].to_affine(), &g_2_aff);
            if lhs != rhs {
                bail!("V[{}] and V_hat[{}] did not match", i, i);
            }
        }

        let R_hat_aff = self
            .R_hat
            .iter()
            .map(|p| p.to_affine())
            .collect::<Vec<G2Affine>>();
        for i in 0..W {
            let lhs = pairing(&g_1_aff, &R_hat_aff[i]);
            let rhs = pairing(&self.R[i].to_affine(), &g_2_aff);
            if lhs != rhs {
                bail!("R[{}] and R_hat[{}] did not match", i, i);
            }
        }

        let h_1_aff = h_1.to_affine();
        let eks = eks
            .iter()
            .map(Into::<G1Projective>::into)
            .map(|p| p.to_affine())
            .collect::<Vec<G1Affine>>();
        for i in 0..n {
            let p = sc.get_player(i);
            let weight = sc.get_player_weight(&p);
            for j in 0..weight {
                let k = sc.get_share_index(i, j).unwrap();
                let lhs = pairing(&h_1_aff, &V_hat_aff[k]).add(pairing(&eks[i], &R_hat_aff[k]));
                let rhs = pairing(&self.C[k].to_affine(), &g_2_aff);
                if lhs != rhs {
                    bail!("C[{},{}] = C[{}] did not match", i, j, k);
                }
            }
        }

        Ok(())
    }
}

impl MalleableTranscript for Transcript {
    fn maul_signature<A: Serialize + Clone>(
        &mut self,
        ssk: &Self::SigningSecretKey,
        aux: &A,
        player: &Player,
    ) {
        let comm = self.V.last().unwrap();
        let sig = Transcript::sign_contribution(ssk, player, aux, comm);
        self.soks[0].0 = *player;
        self.soks[0].1 = *comm;
        self.soks[0].2 = sig;
    }
}

impl Transcript {
    pub fn sign_contribution<A: Serialize + Clone>(
        sk: &bls12381::PrivateKey,
        player: &Player,
        aux: &A,
        comm: &G1Projective,
    ) -> bls12381::Signature {
        sk.sign(&Contribution::<G1Projective, A> {
            comm: *comm,
            player: *player,
            aux: aux.clone(),
        })
        .expect("signing of PVSS contribution should have succeeded")
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn dummy() -> Self {
        Self {
            soks: vec![],
            R: vec![],
            R_hat: vec![],
            V: vec![],
            V_hat: vec![],
            C: vec![],
        }
    }
}
