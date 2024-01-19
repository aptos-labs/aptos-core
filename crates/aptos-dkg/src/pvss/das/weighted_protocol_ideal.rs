// Copyright © Aptos Foundation
//! TODO(Performance): Would storing elements in affine representation after deserializing help?

use crate::pvss::test_utils::{insecure_random_g1_points, insecure_random_g2_points};
use crate::pvss::traits::transcript::MalleableTranscript;
use crate::{
    algebra::polynomials::shamir_secret_share,
    pvss,
    pvss::{
        contribution::{batch_verify_soks, Contribution},
        das,
        das::PublicParameters,
        encryption_dlog,
        encryption_dlog::g1::EncryptPubKey,
        fiat_shamir, schnorr,
        scrape::LowDegreeTest,
        traits,
        traits::{HasEncryptionPublicParams, SecretSharingConfig},
        Player, WeightedConfig,
    },
    utils::{
        g1_multi_exp, g2_multi_exp, multi_pairing,
        random::{random_g2_point, random_scalar, random_scalars},
    },
};
use anyhow::bail;
use aptos_crypto::{bls12381, CryptoMaterialError, Genesis, SigningKey, ValidCryptoMaterial};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use blstrs::{pairing, G1Projective, G2Projective, Gt};
use group::{Curve, Group};
use more_asserts::debug_assert_le;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use std::ops::{Add, Mul, Neg, Sub};

/// Scheme name
pub const WEIGHTED_DAS_SK_IN_G1: &'static str = "ideal_weighted_das_sk_in_g1";

/// Domain-separator tag (DST) for the Fiat-Shamir hashing used to derive randomness from the transcript.
const DAS_WEIGHTED_PVSS_FIAT_SHAMIR_DST: &[u8; 47] =
    b"APTOS_DAS_WEIGHTED_IDEALLY_PVSS_FIAT_SHAMIR_DST";

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
    /// TODO: update transcript size in tests? but the # of aggregations influences the size...
    soks: Vec<(
        Player,
        G2Projective,
        bls12381::Signature,
        schnorr::PoK<G2Projective>,
    )>,
    /// Commitment to encryption randomness $g_2^{r_j} \in G_2, \forall j \in [M]$
    R: Vec<G2Projective>,
    /// First $W$ elements are commitments to the evaluations of $p(X)$: $g_2^{p(\omega^i)}$,
    /// where $i \in [W]$. Last element is $g_2^{p(0)}$ (i.e., the dealt public key).
    V: Vec<G2Projective>,
    /// $C$ is a concatenation of two vectors $(C_0, C_1)$, where:
    ///
    /// $C_0$ stores commitments to encryption randomness $C_0[j - 1] = g_1^{r_j} \in G_1, \forall j \in [M]$.
    ///
    /// $C_1$ stores ElGamal encryption of the $j$th share of player $i$ $C_{i, j} = h_1^{p(\omega^{s_i + j - 1})} ek_i^{r_j}$.
    /// $\forall i \in [n], j \in [w_i], C_{i,j} = C_1[s_i+j-1]$
    C: Vec<G1Projective>,
}

impl ValidCryptoMaterial for Transcript {
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
        let (f, f_evals) = shamir_secret_share(sc.get_threshold_config(), s, rng);
        assert_eq!(f.len(), sc.get_threshold_weight());
        assert_eq!(f_evals.len(), W);

        let max_weight = sc.get_max_player_weight();

        // Pick ElGamal randomness r_j, \forall j \in [M]
        // r[j] = r_{j+1}, \forall j \in [0, M-1]
        let r = random_scalars(max_weight, &mut rng);
        let g_1 = pp.get_encryption_public_params().pubkey_base();
        let g_2 = pp.get_commitment_base();
        let h_1 = *pp.get_encryption_public_params().message_base();

        // NOTE: Recall s_i is the starting index of player i in the vector of shares
        //  - V[s_i + j - 1] = g_2^{f(s_i + j - 1)}
        //  - V[W] = g_2^{f(0)}
        let V = (0..W)
            .map(|k| g_2.mul(f_evals[k]))
            .chain([g_2.mul(f[0])])
            .collect::<Vec<G2Projective>>();
        assert_eq!(V.len(), W + 1);

        // R[j] = g_2^{r_{j + 1}},  \forall j \in [0, M-1]
        let R = (0..max_weight)
            .map(|j| g_2.mul(r[j]))
            .collect::<Vec<G2Projective>>();
        assert_eq!(R.len(), max_weight);

        // C = C_0 || C_1 such that:
        //  1. C_0[j] = g_1^{r_{j + 1}}, \forall j \in [0, M-1]
        let mut C = Vec::with_capacity(max_weight + W);
        for j in 0..max_weight {
            C.push(g_1.mul(r[j]))
        }
        assert_eq!(C.len(), max_weight);

        // ...
        //  2. C_1[s_i + j - 1] = h_1^{f(s_i + j - 1)} ek_i^{r_j}, for all i \in [n], j \in [w_i]
        for i in 0..n {
            let w_i = sc.get_player_weight(&sc.get_player(i));
            debug_assert_le!(w_i, max_weight);

            let bases = vec![h_1, Into::<G1Projective>::into(&eks[i])];
            for j in 0..w_i {
                let k = sc.get_share_index(i, j).unwrap();

                C.push(g1_multi_exp(
                    bases.as_slice(),
                    [f_evals[k], r[j]].as_slice(),
                ))
            }
        }
        assert_eq!(C.len(), max_weight + W);

        // Compute PoK of input secret committed in V[n]
        // TODO(fiat-shamir)
        let pok = schnorr::pok_prove(&f[0], g_2, &V[W], rng);

        // Sign the secret commitment, player ID and `aux`
        let sig = Transcript::sign_contribution(ssk, dealer, aux, &V[W]);

        Transcript {
            soks: vec![(*dealer, V[W], sig, pok)],
            R,
            V,
            C,
        }
    }

    #[allow(non_snake_case)]
    fn verify<A: Serialize + Clone>(
        &self,
        sc: &Self::SecretSharingConfig,
        pp: &Self::PublicParameters,
        spks: &Vec<Self::SigningPubKey>,
        eks: &Vec<Self::EncryptPubKey>,
        aux: &Vec<A>,
    ) -> anyhow::Result<()> {
        self.check_sizes(sc)?;
        let n = sc.get_total_num_players();
        if eks.len() != n {
            bail!("Expected {} encryption keys, but got {}", n, eks.len());
        }

        // Derive challenges deterministically via Fiat-Shamir; easier to debug for distributed systems
        let (f, extra) = fiat_shamir::fiat_shamir(
            self,
            sc.get_threshold_config(),
            pp,
            eks,
            &DAS_WEIGHTED_PVSS_FIAT_SHAMIR_DST[..],
            2,
        );

        let g_2 = pp.get_commitment_base();
        let W = sc.get_total_weight();
        batch_verify_soks::<G2Projective, A>(&self.soks, g_2, &self.V[W], spks, aux, &extra[0])?;

        let ldt = LowDegreeTest::new(
            f,
            sc.get_threshold_weight(),
            W + 1,
            true,
            sc.get_batch_evaluation_domain(),
        )?;
        ldt.low_degree_test_on_g2(&self.V)?;

        //
        // Correctness of encryptions check
        //
        // (See the [WVUF Overleaf](https://www.overleaf.com/project/654553237bd320376aadcbdd) for how this batch verification was derived.)
        //

        let max_weight = sc.get_max_player_weight();
        // TODO: 128 bit scalars from Merlin transcript
        let mut alphas_and_betas = random_scalars(max_weight + W, &mut thread_rng());
        // beta[s_i + j - 1] = \beta_{i, j}, \forall i\in[n] j \in [w_i]
        let (alphas, betas) = alphas_and_betas.split_at_mut(max_weight);
        assert_eq!(alphas.len(), max_weight);
        assert_eq!(betas.len(), W);

        let R_alphas = g2_multi_exp(self.R.as_slice(), alphas);

        // Last V[W] needs to be skipped, since it's not a share commitment; it's the SK commitment.
        let (V_split, _) = self.V.split_at(W);
        assert_eq!(V_split.len(), W);
        let V = g2_multi_exp(V_split, betas);

        // Fetch the encryption keys as G_1 group elements
        let eks = eks
            .iter()
            .map(|ek| Into::<G1Projective>::into(ek))
            .collect::<Vec<G1Projective>>();
        // eks_beta[j] = \prod_{i \in eligible(j)} ek_i^{\beta_{i, j}}
        let mut eks_to_beta_ijs = Vec::with_capacity(max_weight);
        let mut bases = Vec::with_capacity(max_weight);
        let mut exps = Vec::with_capacity(max_weight);
        for j in 0..max_weight {
            bases.clear();
            exps.clear();

            for i in 0..n {
                if let Some(k) = sc.get_share_index(i, j) {
                    bases.push(eks[i]);
                    exps.push(betas[k]);
                }
            }

            eks_to_beta_ijs.push(g1_multi_exp(bases.as_slice(), exps.as_slice()));
        }

        // WARNING: Computing this last since the beta's must be negated
        betas.iter_mut().for_each(|beta| *beta = beta.neg());
        let C = g1_multi_exp(self.C.as_slice(), alphas_and_betas.as_slice());

        // The vector of left-hand-side ($\mathbb{G}_1$) inputs to each pairing in the multi-pairing.
        let g_1_neg = pp.get_encryption_public_params().pubkey_base().neg();
        let h_1 = pp.get_encryption_public_params().message_base();
        let lhs = [&C, h_1]
            .into_iter()
            .chain(eks_to_beta_ijs.iter())
            .chain([&g_1_neg].into_iter())
            .into_iter();
        // The vector of right-hand-side ($\mathbb{G}_2$) inputs to each pairing in the multi-pairing.
        let rhs = [g_2, &V]
            .into_iter()
            .chain(self.R.iter())
            .chain([&R_alphas].into_iter())
            .into_iter();

        let res = multi_pairing(lhs, rhs);
        if res != Gt::identity() {
            bail!("Expected zero, but got {} during multi-pairing check", res);
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
        let max_weight = sc.get_max_player_weight();
        let W = sc.get_total_weight();

        debug_assert_eq!(self.R.len(), max_weight);
        debug_assert_eq!(self.C.len(), max_weight + W);
        debug_assert_eq!(self.V.len(), W + 1);
        debug_assert_eq!(self.R.len(), other.R.len());
        debug_assert_eq!(self.C.len(), other.C.len());
        debug_assert_eq!(self.V.len(), other.V.len());

        for i in 0..self.R.len() {
            self.R[i] += other.R[i];
        }

        for i in 0..self.C.len() {
            self.C[i] += other.C[i];
        }
        for i in 0..self.V.len() {
            self.V[i] += other.V[i];
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
                Self::DealtPubKey::new(self.V[k]),
            ));
        }

        pk_shares
    }

    fn get_dealt_public_key(&self) -> Self::DealtPubKey {
        Self::DealtPubKey::new(*self.V.last().unwrap())
    }

    #[allow(non_snake_case)]
    fn decrypt_own_share(
        &self,
        sc: &Self::SecretSharingConfig,
        player: &Player,
        dk: &Self::DecryptPrivKey,
    ) -> (Self::DealtSecretKeyShare, Self::DealtPubKeyShare) {
        // TODO(Security): Could get an out-of-bounds error here if the wrong Player is passed in. Maybe improve the design.
        let max_weight = sc.get_max_player_weight();
        let (C_0, C_1) = self.C.split_at(max_weight);

        let weight = sc.get_player_weight(player);
        let mut sk_shares = Vec::with_capacity(weight);
        let pk_shares = self.get_public_key_share(sc, player);

        for j in 0..weight {
            let k = sc.get_share_index(player.id, j).unwrap();

            let ctxt = C_1[k]; // C_1[s_i + j - 1] = C_{i,j} = h_1^{f(s_i + j - 1)} \ek_i^{r_j}
            let ephemeral_key = C_0[j].mul(dk.dk); // C_0[j-1] = (g_1^{r_j})
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
        let max_weight = sc.get_max_player_weight();

        let sk = bls12381::PrivateKey::genesis();
        Transcript {
            soks: vec![(
                sc.get_player(0),
                random_g2_point(rng),
                Transcript::sign_contribution::<usize>(
                    &sk,
                    &sc.get_player(0),
                    &0usize,
                    &random_g2_point(rng),
                ),
                (random_g2_point(rng), random_scalar(rng)),
            )],
            R: insecure_random_g2_points(max_weight, rng),
            V: insecure_random_g2_points(W + 1, rng),
            C: insecure_random_g1_points(max_weight + W, rng),
        }
    }
}

impl Transcript {
    #[allow(non_snake_case)]
    fn check_sizes(&self, sc: &WeightedConfig) -> anyhow::Result<()> {
        let max_weight = sc.get_max_player_weight();
        let W = sc.get_total_weight();

        if self.R.len() != max_weight {
            bail!(
                "Expected {} G_2 commitment(s) to ElGamal randomness, but got {}",
                max_weight,
                self.R.len()
            );
        }
        if self.C.len() != max_weight + W {
            bail!(
                "Expected C of length {}, but got {}",
                max_weight + W,
                self.C.len()
            );
        }

        if self.V.len() != W + 1 {
            bail!(
                "Expected {} (polynomial) commitment elements, but got {}",
                W + 1,
                self.V.len()
            );
        }

        Ok(())
    }

    /// For testing.
    #[allow(non_snake_case, unused)]
    fn slow_verify(
        &self,
        sc: &&WeightedConfig,
        pp: &PublicParameters,
        eks: &Vec<EncryptPubKey>,
    ) -> anyhow::Result<()> {
        let n = sc.get_total_num_players();
        let g_2 = pp.get_commitment_base();
        let h_1 = pp.get_encryption_public_params().message_base();
        let W = sc.get_total_weight();

        let eks_g1 = eks
            .iter()
            .map(|ek| Into::<G1Projective>::into(ek))
            .collect::<Vec<G1Projective>>();

        let (_, C_1) = self.C.split_at(sc.get_max_player_weight());
        assert_eq!(C_1.len(), W);

        for i in 0..n {
            for j in 0..sc.get_player_weight(&sc.get_player(i)) {
                let k = sc.get_share_index(i, j).unwrap();
                println!("{} = s_{} + {}", k, i, j);

                assert_eq!(
                    pairing(&h_1.to_affine(), &self.V[k].to_affine())
                        .add(pairing(&eks_g1[i].to_affine(), &self.R[j].to_affine())),
                    pairing(&C_1[k].to_affine(), &g_2.to_affine())
                );
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
        comm: &G2Projective,
    ) -> bls12381::Signature {
        sk.sign(&Contribution::<G2Projective, A> {
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
            V: vec![],
            C: vec![],
        }
    }
}
