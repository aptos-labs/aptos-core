// Copyright © Aptos Foundation

use crate::algebra::polynomials::{get_powers_of_tau, shamir_secret_share};
use crate::pvss;
use crate::pvss::das::DAS_SK_IN_G1;
use crate::pvss::scrape::LowDegreeTest;
use crate::pvss::traits::{HasEncryptionPublicParams, SecretSharingConfig};
use crate::pvss::{das, encryption_dlog, fiat_shamir, schnorr, traits, Player, ThresholdConfig};
use crate::utils::random::{random_g1_point, random_g2_point, random_scalar};
use crate::utils::{g1_multi_exp, g2_multi_exp, multi_pairing};
use anyhow::bail;
use aptos_crypto::{bls12381, CryptoMaterialError, Genesis, SigningKey, ValidCryptoMaterial};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use blstrs::{G1Projective, G2Projective, Gt};
use group::Group;
use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Mul, Neg, Sub};

/// Domain-separator tag (DST) for the Fiat-Shamir hashing used to derive randomness from the transcript.
const DAS_PVSS_FIAT_SHAMIR_DST: &[u8; 30] = b"APTOS_DAS_PVSS_FIAT_SHAMIR_DST";

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
    /// ElGamal encryption randomness $g_2^r \in G_2$
    hat_w: G2Projective,
    /// First $n$ elements are commitments to the evaluations of $p(X)$: $g_2^{p(\omega^i)}$,
    /// where $i \in [n]$. Last element is $g_2^{p(0)}$ (i.e., the dealt public key).
    V: Vec<G2Projective>,
    /// ElGamal encryptions of the shares $h_1^{p(\omega^i)} ek^r$.
    C: Vec<G1Projective>,
    /// Ciphertext randomness commitment $g_1^r$.
    C_0: G1Projective,
}

/// A PVSS contribution, which is signed as part of the PVSS transcript
/// TODO(TechDebt): CryptoHasher doesn't seem to work with lifetimes. So I cannot make this struct store references, which causes unnecesary cloning in the code.
#[derive(Serialize, Deserialize, BCSCryptoHash, CryptoHasher)]
pub struct Contribution<A> {
    comm: G2Projective,
    player: Player,
    aux: A,
}

// TODO(Optimization): For verification, can we get any speed-ups when a lot of the PKs are the same? Assuming the PVSS remains secure.

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
    type PublicParameters = das::PublicParameters;
    type SigningSecretKey = bls12381::PrivateKey;
    type SigningPubKey = bls12381::PublicKey;
    type DealtSecretKeyShare = pvss::dealt_secret_key_share::g1::DealtSecretKeyShare;
    type DealtPubKeyShare = pvss::dealt_pub_key_share::g2::DealtPubKeyShare;
    type DealtSecretKey = pvss::dealt_secret_key::g1::DealtSecretKey;
    type DealtPubKey = pvss::dealt_pub_key::g2::DealtPubKey;
    type InputSecret = pvss::input_secret::InputSecret;
    type EncryptPubKey = encryption_dlog::g1::EncryptPubKey;
    type DecryptPrivKey = encryption_dlog::g1::DecryptPrivKey;

    fn scheme_name() -> String {
        DAS_SK_IN_G1.to_string()
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
        assert_eq!(eks.len(), sc.n);

        let (f, f_evals) = shamir_secret_share(sc, s, rng);

        // Pick ElGamal randomness
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

        // Compute PoK of input secret committed in V[n]
        let pok = schnorr::pok_prove(&f[0], g_2, &V[sc.n], rng);

        debug_assert_eq!(V.len(), sc.n + 1);
        debug_assert_eq!(C.len(), sc.n);

        // Sign the secret commitment, player ID and `aux`
        let sig = ssk
            .sign(&Contribution {
                comm: V[sc.n],
                player: *dealer,
                aux: aux.clone(),
            })
            .expect("signing of PVSS contribution should have succeeded");

        Transcript {
            soks: vec![(*dealer, V[sc.n], sig, pok)],
            hat_w: g_2.mul(r),
            V,
            C,
            C_0: g_1.mul(r),
        }
    }

    fn verify<A: Serialize + Clone>(
        &self,
        sc: &Self::SecretSharingConfig,
        pp: &Self::PublicParameters,
        spks: &Vec<Self::SigningPubKey>,
        eks: &Vec<Self::EncryptPubKey>,
        aux: &Vec<A>,
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

        if self.soks.len() != spks.len() {
            bail!(
                "Expected {} signing PKs, but got {}",
                self.soks.len(),
                spks.len()
            );
        }

        if self.soks.len() != aux.len() {
            bail!(
                "Expected {} auxiliary infos, but got {}",
                self.soks.len(),
                aux.len()
            );
        }

        // Verify signature(s) on the secret commitment, player ID and `aux`
        let msgs = self
            .soks
            .iter()
            .zip(aux)
            .map(|((player, comm, _, _), aux)| Contribution::<A> {
                comm: *comm,
                player: *player,
                aux: aux.clone(),
            })
            .collect::<Vec<Contribution<A>>>();
        let msgs_refs = msgs.iter().map(|c| c).collect::<Vec<&Contribution<A>>>();
        let pks = spks
            .iter()
            .map(|pk| pk)
            .collect::<Vec<&Self::SigningPubKey>>();
        let sig = bls12381::Signature::aggregate(
            self.soks
                .iter()
                .map(|(_, _, sig, _)| sig.clone())
                .collect::<Vec<bls12381::Signature>>(),
        )?;

        sig.verify_aggregate(&msgs_refs[..], &pks[..])?;

        // Verify the PoK(s) of the dealt secret
        let mut c = G2Projective::identity();
        for (_, c_i, _, _) in &self.soks {
            c.add_assign(c_i)
        }

        if c != self.V[sc.n] {
            bail!(
                "The PoK does not correspond to the dealt secret. Expected {} but got {}",
                self.V[sc.n],
                c
            );
        }

        // Derive challenges deterministically via Fiat-Shamir; easier to debug for distributed systems
        let (f, extra) =
            fiat_shamir::fiat_shamir(self, sc, pp, eks, &DAS_PVSS_FIAT_SHAMIR_DST[..], 3);

        let gamma = extra[2];
        let g_2 = pp.get_commitment_base();
        schnorr::pok_batch_verify(
            &self
                .soks
                .iter()
                .map(|(_, c, _, pok)| (*c, *pok))
                .collect::<Vec<(G2Projective, schnorr::PoK<G2Projective>)>>(),
            g_2,
            &gamma,
        )?;

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
        let r_i = get_powers_of_tau(&extra[0], sc.n);

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
        let g_1_inverse = pp.get_encryption_public_params().pubkey_base().neg();
        let alpha = extra[1];

        // The vector of left-hand-side ($\mathbb{G}_1$) inputs to each pairing in the multi-pairing.
        let lhs = vec![h_1.mul(alpha), ek.add(g_1_inverse), self.C_0.add(c.neg())];
        // The vector of right-hand-side ($\mathbb{G}_2$) inputs to each pairing in the multi-pairing.
        let rhs = vec![v, self.hat_w.mul(alpha), g_2.mul(alpha)];

        let res = multi_pairing(lhs.iter(), rhs.iter());
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

        for sok in &other.soks {
            self.soks.push(sok.clone());
        }

        debug_assert_eq!(self.C.len(), other.C.len());
        debug_assert_eq!(self.V.len(), other.V.len());
    }

    fn get_dealt_public_key(&self) -> Self::DealtPubKey {
        Self::DealtPubKey::new(*self.V.last().unwrap())
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
            Self::DealtSecretKeyShare::new(Self::DealtSecretKey::new(dealt_secret_key_share)),
            Self::DealtPubKeyShare::new(Self::DealtPubKey::new(dealt_pub_key_share)),
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
        let sk = bls12381::PrivateKey::genesis();
        Transcript {
            soks: vec![(
                sc.get_player(0),
                r2,
                sk.sign(&Contribution::<usize> {
                    comm: r2,
                    player: sc.get_player(0),
                    aux: 0,
                })
                .unwrap(),
                (acc_g2, random_scalar(rng)),
            )],
            hat_w: g2,
            V: V.iter().map(|p2| p2 + r2).collect(),
            C: C.iter().map(|p1| p1 + r1).collect(),
            C_0: random_g1_point(rng),
        }
    }
}
