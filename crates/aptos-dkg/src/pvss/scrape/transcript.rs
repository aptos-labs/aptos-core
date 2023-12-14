// Copyright © Aptos Foundation

use crate::{
    algebra::polynomials::shamir_secret_share,
    pvss,
    pvss::{
        encryption_dlog, fiat_shamir,
        player::Player,
        scrape,
        scrape::{LowDegreeTest, SCRAPE_SK_IN_G2},
        threshold_config::ThresholdConfig,
        traits,
        traits::SecretSharingConfig,
    },
    utils::{
        g2_multi_exp, multi_pairing,
        random::{random_g1_point, random_g2_point},
    },
};
use anyhow::bail;
use aptos_crypto::{bls12381, CryptoMaterialError, ValidCryptoMaterial};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use blstrs::{G1Projective, G2Projective, Gt, Scalar};
use ff::Field;
use group::Group;
use serde::{Deserialize, Serialize};
use std::ops::{Mul, Neg};

/// Domain-separator tag (DST) for the Fiat-Shamir hashing used to derive randomness from the transcript.
const SCRAPE_PVSS_FIAT_SHAMIR_DST: &[u8; 33] = b"APTOS_SCRAPE_PVSS_FIAT_SHAMIR_DST";

/// A PVSS *transcript*.
///
/// We use the normal serde `Serialize` and `Deserialize` macros because `aptos_crypto`'s `SerializeKey`
/// macros override serde's serialization to call into `ValidCryptoMaterial::to_bytes()`. This makes
/// it difficult to serialize complex types because if we call serde serialization inside `to_bytes`
/// on the struct itself, it triggers infinite recursion by having `serde` call back into `to_bytes`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, CryptoHasher, BCSCryptoHash)]
#[allow(non_snake_case)]
pub struct Transcript {
    /// TODO(Security): Add a PoK and signature on the dealt secret `A[n]`. Otherwise, not secure in a publicly-verifiable DKG (see the Das PVSS)
    soks: Vec<Player>,
    /// Commitment to $f(0)$: $\hat{u}_2 = \hat{u}_1^{a_0}$
    u2_hat: G2Projective,
    /// `A[0], ..., A[n-1]` are commitments to the $n$ evaluations of $f(X)$: $g_1^{f(\omega^i)}$
    /// `A[n]` is a commitment to $f(0)$
    A: Vec<G1Projective>,
    /// $n$ encryptions, one for each player's share of $f(X)$: $ek^{f(\omega^i)}, \forall i\in[0,n)$
    Y_hat: Vec<G2Projective>,
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
    type DealtPubKey = pvss::dealt_pub_key::g1::DealtPubKey;
    type DealtPubKeyShare = pvss::dealt_pub_key_share::g1::DealtPubKeyShare;
    type DealtSecretKey = pvss::dealt_secret_key::g2::DealtSecretKey;
    type DealtSecretKeyShare = pvss::dealt_secret_key_share::g2::DealtSecretKeyShare;
    type DecryptPrivKey = encryption_dlog::g2::DecryptPrivKey;
    type EncryptPubKey = encryption_dlog::g2::EncryptPubKey;
    type InputSecret = pvss::input_secret::InputSecret;
    type PublicParameters = scrape::PublicParameters;
    // TODO: remove scrape typedefs, so as to be able to macro things later
    type SecretSharingConfig = ThresholdConfig;
    type SigningPubKey = bls12381::PublicKey;
    type SigningSecretKey = bls12381::PrivateKey;

    fn scheme_name() -> String {
        SCRAPE_SK_IN_G2.to_string()
    }

    fn deal<A: Serialize, R: rand_core::RngCore + rand_core::CryptoRng>(
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

        let (f, f_evals) = shamir_secret_share(sc, s, rng);

        let g1 = pp.get_commitment_base();
        let u1_hat = pp.get_public_key_base();

        Transcript {
            soks: vec![*dealer],
            u2_hat: u1_hat.mul(f[0]),
            A: (0..sc.n)
                .map(|i| g1.mul(f_evals[i]))
                .chain([g1.mul(f[0])].into_iter())
                .collect(),
            Y_hat: (0..sc.n)
                .map(|i| Into::<G2Projective>::into(&eks[i]).mul(f_evals[i]))
                .collect(),
        }
    }

    #[allow(non_snake_case)]
    fn verify<A: Serialize>(
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

        if self.Y_hat.len() != sc.n {
            bail!(
                "Expected {} ciphertexts, but got {}",
                sc.n,
                self.Y_hat.len()
            );
        }

        if self.A.len() != sc.n + 1 {
            bail!(
                "Expected {} (polynomial) commitment elements, but got {}",
                sc.n + 1,
                self.A.len()
            );
        }

        // Derive challenges deterministically via Fiat-Shamir; it's easier to debug for distributed systems
        let (f, r) =
            fiat_shamir::fiat_shamir(self, sc, pp, eks, &SCRAPE_PVSS_FIAT_SHAMIR_DST[..], 1);
        let r = r[0];

        let ldt = LowDegreeTest::new(f, sc.t, sc.n + 1, true, sc.get_batch_evaluation_domain())?;

        let _ = ldt.low_degree_test_on_g1(&self.A)?;

        //
        // Correctness of encryptions check
        // (This could be done via DLEQ proofs too. Check out repo at [this commit](https://github.com/aptos-labs/aptos-dkg/commit/c04ab0d6fa73c3f6ca18d7fe3d6465ef722556b6).)
        //

        // We need to check the following equations hold:
        //
        //                    e(A_i,       ek_i) = e(g_1,        \hat{Y}_i),     \forall i \in [0,n) <=>
        //                    e(A_i,       ek_i)   e(g_1^{-1},   \hat{Y}_i) = 1, \forall i \in [0,n) <=>
        //  \prod_{i\in[0,n)} e(A_i^{r_i}, ek_i)   e(g_1^{-r_i}, \hat{Y}_i) = 1
        //  \prod_{i\in[0,n)} e(A_i^{r_i}, ek_i)   e(g_1, \prod_{i\in[0,n)} \hat{Y}_i^{-r_i}) = 1
        //
        // Lastly, we can add the check for (\hat{u}_1, \hat{u}_2) against F_0 by appending its pairing
        // equation to the multipairing check above:
        //
        //     e(F_0^{r_n}, \hat{u}_1) e(g_1^{-r_n}, \hat{u}_2)
        //
        // We let r_i = r^i, for a random r.

        // TODO(Performance): Would storing elements in affine representation after deserializing help?
        let g1_inverse = pp.get_commitment_base().neg();
        let mut r_i = Vec::with_capacity(sc.n + 1);
        let mut r_i_negated = Vec::with_capacity(sc.n + 1);
        r_i.push(Scalar::ONE);
        r_i_negated.push(-r_i[0]);

        // First, compute r_i = r^i, for all i \in [0, n]
        for _ in 0..sc.n {
            r_i.push(r_i.last().unwrap().mul(&r));
            r_i_negated.push(-*r_i.last().unwrap());
        }
        debug_assert_eq!(r_i.len(), sc.n + 1);

        // The vector of left-hand-side inputs to each pairing in the multi-pairing.
        let lhs = (0..sc.n)
            .map(|i| self.A[i].mul(r_i[i]))
            .chain(
                [
                    *pp.get_commitment_base(),
                    self.A[sc.n].mul(r_i[sc.n]),
                    g1_inverse.mul(r_i[sc.n]),
                ]
                .into_iter(),
            )
            .collect::<Vec<G1Projective>>();

        // The vector of right-hand-side inputs to each pairing in the multi-pairing.
        let (exps, _) = r_i_negated.split_at(sc.n);
        let Y_hat = g2_multi_exp(self.Y_hat.as_slice(), exps);
        let rhs = eks
            .iter()
            .map(|ek| Into::<G2Projective>::into(ek))
            .chain([Y_hat, *pp.get_public_key_base(), self.u2_hat].into_iter())
            .collect::<Vec<G2Projective>>();

        let res = multi_pairing(lhs.iter(), rhs.iter());
        if res != Gt::identity() {
            bail!("Expected zero, but got {} during multi-pairing check", {
                res
            });
        }

        return Ok(());
    }

    fn aggregate_with(&mut self, sc: &ThresholdConfig, other: &Transcript) {
        debug_assert_eq!(self.A.len(), sc.n + 1);
        debug_assert_eq!(self.Y_hat.len(), sc.n);

        self.u2_hat += other.u2_hat;

        for i in 0..sc.n {
            self.A[i] += other.A[i];
            self.Y_hat[i] += other.Y_hat[i];
        }
        self.A[sc.n] += other.A[sc.n];

        for sok in &other.soks {
            self.soks.push(sok.clone());
        }

        debug_assert_eq!(self.A.len(), other.A.len());
        debug_assert_eq!(self.Y_hat.len(), other.Y_hat.len());
    }

    fn get_public_key_share(
        &self,
        _sc: &Self::SecretSharingConfig,
        player: &Player,
    ) -> Self::DealtPubKeyShare {
        Self::DealtPubKeyShare::new(Self::DealtPubKey::new(self.A[player.id]))
    }

    fn get_dealt_public_key(&self) -> Self::DealtPubKey {
        Self::DealtPubKey::new(*self.A.last().unwrap())
    }

    fn decrypt_own_share(
        &self,
        _sc: &ThresholdConfig,
        player: &Player,
        dk: &Self::DecryptPrivKey,
    ) -> (Self::DealtSecretKeyShare, Self::DealtPubKeyShare) {
        // TODO(Security): Could get an out-of-bounds error here if the wrong Player is passed in. Maybe improve the design.
        let ctxt = self.Y_hat[player.id]; // \hat{Y}_i = \ek_i^{f(\omega^i)}
        let dealt_secret_key_share = ctxt.mul(dk.dk); // Y_i^{\dk_i} = \hat{h}_1^{f(\omega^i)} (because \ek_i = \hat{h}_1^{\dk_i^{-1}})
        let dealt_pub_key_share = self.A[player.id]; // g_1^{f(\omega^i})

        (
            Self::DealtSecretKeyShare::new(Self::DealtSecretKey::new(dealt_secret_key_share)),
            Self::DealtPubKeyShare::new(Self::DealtPubKey::new(dealt_pub_key_share)),
        )
    }

    fn generate<R>(sc: &ThresholdConfig, rng: &mut R) -> Self
    where
        R: rand_core::RngCore + rand_core::CryptoRng,
    {
        //
        // TODO(rand_core_hell): Since our random_g1_point and random_g2_point functions are
        // slower than we want, we do not pick everything randomly. Instead, we generate a
        // kind-of-random-looking transcript from a few random elliptic curve points by doubling them.
        //
        let g2 = random_g2_point(rng);
        let mut acc_g2 = g2;
        let g2_vec = (0..sc.n)
            .map(|_| {
                acc_g2 = acc_g2.double();
                acc_g2
            })
            .collect::<Vec<G2Projective>>();

        let mut acc_g1 = random_g1_point(rng);
        let g1_vec = (0..sc.n + 1)
            .map(|_| {
                acc_g1 = acc_g1.double();
                acc_g1
            })
            .collect::<Vec<G1Projective>>();

        let r1 = random_g1_point(rng);
        let r2 = random_g2_point(rng);

        Transcript {
            soks: vec![sc.get_player(0)],
            u2_hat: g2,
            A: g1_vec.iter().map(|p| p + r1).collect(),
            Y_hat: g2_vec.iter().map(|p| p + r2).collect(),
        }
    }

    fn get_dealers(&self) -> Vec<Player> {
        self.soks.clone()
    }
}
