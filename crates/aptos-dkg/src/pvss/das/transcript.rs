// Copyright © Aptos Foundation

use crate::algebra::polynomials::shamir_secret_share;
use crate::pvss::das::DAS_SK_IN_G1;
use crate::pvss::scrape::LowDegreeTest;
use crate::pvss::traits::HasEncryptionPublicParams;
use crate::pvss::{das, encryption_dlog, fiat_shamir, traits, Player, ThresholdConfig};
use crate::utils::random::{random_g1_point, random_g2_point, random_scalar};
use crate::utils::{g1_multi_exp, g2_multi_exp, multi_pairing};
use anyhow::bail;
use aptos_crypto::{CryptoMaterialError, ValidCryptoMaterial};
use blstrs::{G1Projective, G2Projective, Gt, Scalar};
use ff::Field;
use group::Group;
use serde::{Deserialize, Serialize};
use std::ops::{Add, Mul, Neg, Sub};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[allow(non_snake_case)]
pub struct Transcript {
    /// ElGamal encryption randomness $g_2^r \in G_2$
    hat_w: G2Projective,
    /// First $n$ elements are commitments to the evaluations of $p(X)$: $g_2^{p(\omega^i)}$. Last
    /// element is $g_2^{p(0)}$ (i.e., the dealt public key).
    V: Vec<G2Projective>,
    /// ElGamal encryptions of the shares $h_1^{p(\omega^i)} ek^r$.
    C: Vec<G1Projective>,
    /// Ciphertext randomness commitment $g_1^r$.
    C_0: G1Projective,
}

// TODO(Performance): for verification, can we get any speed-ups when a lot of the PKs are the same?

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
    type SecretSharingConfig = ThresholdConfig;
    type PvssPublicParameters = das::PublicParameters;
    type DealtSecretKeyShare = das::DealtSecretKeyShare;
    type DealtPubKeyShare = das::DealtPubKeyShare;
    type DealtSecretKey = das::DealtSecretKey;
    type DealtPubKey = das::DealtPubKey;
    type InputSecret = das::InputSecret;
    type EncryptPubKey = encryption_dlog::g1::EncryptPubKey;
    type DecryptPrivKey = encryption_dlog::g1::DecryptPrivKey;

    fn scheme_name() -> String {
        DAS_SK_IN_G1.to_string()
    }

    #[allow(non_snake_case)]
    fn deal<R: rand_core::RngCore + rand_core::CryptoRng>(
        sc: &ThresholdConfig,
        pp: &das::PublicParameters,
        eks: &Vec<Self::EncryptPubKey>,
        s: &Self::InputSecret,
        _dst: &'static [u8], // DSTs are only needed in `Transcript::verify` for computing random challenges via Fiat-Shamir
        mut rng: &mut R,
    ) -> Self {
        assert_eq!(eks.len(), sc.n);

        let (f, f_evals) = shamir_secret_share(sc, s, rng);

        // ElGamal randomness
        let r = random_scalar(&mut rng);
        let g_1 = pp.get_encryption_public_params().pubkey_base();
        let g_2 = pp.get_commitment_base();
        let h_1 = *pp.get_encryption_public_params().message_base();

        let V = (0..sc.n)
            .map(|i| g_2.mul(f_evals[i]))
            .chain([g_2.mul(f[0])])
            .collect::<Vec<G2Projective>>();

        let C = (0..sc.n)
            .map(|i| {
                g1_multi_exp(
                    [h_1, Into::<G1Projective>::into(&eks[i])].as_slice(),
                    [f_evals[i], r].as_slice(),
                )
            })
            .collect::<Vec<G1Projective>>();

        debug_assert_eq!(V.len(), sc.n + 1);
        debug_assert_eq!(C.len(), sc.n);

        Transcript {
            hat_w: g_2.mul(r),
            V,
            C,
            C_0: g_1.mul(r),
        }
    }

    fn verify(
        &self,
        sc: &ThresholdConfig,
        pp: &das::PublicParameters,
        eks: &Vec<Self::EncryptPubKey>,
        dst: &'static [u8],
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

        // Derive challenges deterministically via Fiat-Shamir; it's easier to debug for distributed systems
        let (f, r_and_alpha) = fiat_shamir::fiat_shamir(self, sc, pp, eks, dst, 2);
        let r = r_and_alpha[0];
        let alpha = r_and_alpha[1];

        let ldt = LowDegreeTest::new(f, sc.t, sc.n + 1, true, sc.get_batch_evaluation_domain())?;
        ldt.low_degree_test_on_g2(&self.V)?;

        //
        // Correctness of encryptions check
        //

        // We need to check the following equations hold:
        //
        // v  = \prod_{i=0}^{n-1} V[i] ^{r^i}
        // ek = \prod_{i=0}^{n-1} ek[i]^{r^i}
        // c  = \prod_{i=0}^{n-1} C[i] ^{r^i}
        //
        //   e(C[0], g_2) = e(g_1, \hat{w})
        //   e(h_1 , v) e(ek, \hat{w}) = e(c, g_2)
        //
        // Next: combine these using a linear-combination:
        //   e(h_1^\alpha , v) e(ek, \hat{w}^\alpha) e(C[0], g_2^\alpha) =
        //   e(g_1, \hat{w}^\alpha) e(c, g_2^\alpha)
        //
        // Move the RHS into the LHS:
        //   e(h_1^\alpha, v)
        //   e(ek,         \hat{w}^\alpha)
        //   e(C[0],       g_2^\alpha)
        //   e(g_1^{-1},   \hat{w}^\alpha)
        //   e(c^{-1},     g_2^\alpha) = 1
        //
        // Group pairings:
        //   e(h_1^\alpha,  v)
        //   e(ek g_1^{-1}, \hat{w}^\alpha)
        //   e(C[0] c^{-1}, g_2^\alpha) = 1

        // TODO(Performance): Would storing elements in affine representation after deserializing help?
        let mut r_i = Vec::with_capacity(sc.n);
        r_i.push(Scalar::ONE);

        // First, compute r_i = r^i, for all i \in [0, n]
        for _ in 0..sc.n - 1 {
            r_i.push(r_i.last().unwrap().mul(&r));
        }
        debug_assert_eq!(r_i.len(), sc.n);

        // Compute the multiexps from above.
        // Note: |V| = |r_i| + 1, so the multiexp will be of size |r_i|.
        let v = g2_multi_exp(&self.V[..self.V.len() - 1], r_i.as_slice());
        let ek = g1_multi_exp(
            eks.iter()
                .map(|ek| Into::<G1Projective>::into(ek))
                .collect::<Vec<G1Projective>>()
                .as_slice(),
            r_i.as_slice(),
        );
        let c = g1_multi_exp(self.C.as_slice(), r_i.as_slice());

        // Fetch some public parameters
        let h_1 = pp.get_encryption_public_params().message_base();
        let g_2 = pp.get_commitment_base();
        let g_1_inverse = pp.get_encryption_public_params().pubkey_base().neg();

        // The vector of left-hand-side ($\mathbb{G}_1$) inputs to each pairing in the multi-pairing.
        let lhs = vec![h_1.mul(alpha), ek.add(g_1_inverse), self.C_0.add(c.neg())];
        // The vector of right-hand-side ($\mathbb{G}_2$) inputs to each pairing in the multi-pairing.
        let rhs = vec![v, self.hat_w.mul(alpha), g_2.mul(alpha)];

        let res = multi_pairing(lhs.iter(), rhs.iter());
        if res != Gt::identity() {
            bail!("Expected zero, but got {} during multi-pairing check", {
                res
            });
        }

        return Ok(());
    }

    fn aggregate_with(&mut self, sc: &ThresholdConfig, other: &Transcript) {
        debug_assert_eq!(self.C.len(), sc.n);
        debug_assert_eq!(self.V.len(), sc.n + 1);

        self.hat_w += other.hat_w;
        self.C_0 += other.C_0;

        for i in 0..sc.n {
            self.C[i] += other.C[i];
            self.V[i] += other.V[i];
        }
        self.V[sc.n] += other.V[sc.n];

        debug_assert_eq!(self.C.len(), other.C.len());
        debug_assert_eq!(self.V.len(), other.V.len());
    }

    fn get_dealt_public_key(&self) -> das::DealtPubKey {
        das::DealtPubKey::new(*self.V.last().unwrap())
    }

    fn decrypt_own_share(
        &self,
        _sc: &ThresholdConfig,
        player: &Player,
        dk: &Self::DecryptPrivKey,
    ) -> (Self::DealtSecretKeyShare, Self::DealtPubKeyShare) {
        // TODO(Security): Could get an out-of-bounds error here if the wrong Player is passed in. Maybe improve the design.
        let ctxt = self.C[player.id]; // C_i = h_1^m \ek_i^r = h_1^m g_1^{r sk_i}
        let ephemeral_key = self.C_0.mul(dk.dk); // (g_1^r)^{sk_i} = ek_i^r
        let dealt_secret_key_share = ctxt.sub(ephemeral_key);
        let dealt_pub_key_share = self.V[player.id]; // g_2^{f(\omega^i})

        (
            das::DealtSecretKeyShare(Self::DealtSecretKey::new(dealt_secret_key_share)),
            das::DealtPubKeyShare(Self::DealtPubKey::new(dealt_pub_key_share)),
        )
    }

    #[allow(non_snake_case)]
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
        let V = (0..sc.n + 1)
            .map(|_| {
                acc_g2 = acc_g2.double();
                acc_g2
            })
            .collect::<Vec<G2Projective>>();

        let mut acc_g1 = random_g1_point(rng);
        let C = (0..sc.n)
            .map(|_| {
                acc_g1 = acc_g1.double();
                acc_g1
            })
            .collect::<Vec<G1Projective>>();

        let r1 = random_g1_point(rng);
        let r2 = random_g2_point(rng);

        Transcript {
            hat_w: g2,
            V: V.iter().map(|p2| p2 + r2).collect(),
            C: C.iter().map(|p1| p1 + r1).collect(),
            C_0: random_g1_point(rng),
        }
    }
}
