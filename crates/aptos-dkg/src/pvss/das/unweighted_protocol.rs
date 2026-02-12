// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    algebra::polynomials::{get_nonzero_powers_of_tau, shamir_secret_share},
    pvss::{
        self,
        contribution::{batch_verify_soks, Contribution, SoK},
        das, encryption_dlog, schnorr,
        traits::{
            self, transcript::MalleableTranscript, AggregatableTranscript,
            HasEncryptionPublicParams,
        },
        LowDegreeTest, Player, ThresholdConfigBlstrs,
    },
    traits::transcript::{Aggregatable, Aggregated},
    utils::{
        g1_multi_exp, g2_multi_exp,
        random::{
            insecure_random_g1_points, insecure_random_g2_points, random_g1_point, random_g2_point,
            random_scalars,
        },
    },
};
use anyhow::bail;
use aptos_crypto::{
    bls12381,
    blstrs::{multi_pairing, random_scalar},
    traits::TSecretSharingConfig as _,
    CryptoMaterialError, Genesis, SigningKey, ValidCryptoMaterial,
};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use blstrs::{G1Projective, G2Projective, Gt};
use group::Group;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use std::ops::{Add, Mul, Neg, Sub};

pub const DAS_SK_IN_G1: &'static str = "das_sk_in_g1";

/// Domain-separator tag (DST) for the Fiat-Shamir hashing used to derive randomness from the transcript.

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
    soks: Vec<SoK<G2Projective>>,
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
    type DealtPubKeyShare = pvss::dealt_pub_key_share::g2::DealtPubKeyShare;
    type DealtSecretKey = pvss::dealt_secret_key::g1::DealtSecretKey;
    type DealtSecretKeyShare = pvss::dealt_secret_key_share::g1::DealtSecretKeyShare;
    type DecryptPrivKey = encryption_dlog::g1::DecryptPrivKey;
    type EncryptPubKey = encryption_dlog::g1::EncryptPubKey;
    type InputSecret = pvss::input_secret::InputSecret;
    type PublicParameters = das::PublicParameters;
    type SecretSharingConfig = ThresholdConfigBlstrs;
    type SigningPubKey = bls12381::PublicKey;
    type SigningSecretKey = bls12381::PrivateKey;

    fn dst() -> Vec<u8> {
        b"APTOS_DAS_PVSS_FIAT_SHAMIR_DST".to_vec()
    }

    fn scheme_name() -> String {
        DAS_SK_IN_G1.to_string()
    }

    #[allow(non_snake_case)]
    fn deal<A: Serialize + Clone, R: rand_core::RngCore + rand_core::CryptoRng>(
        sc: &Self::SecretSharingConfig,
        pp: &Self::PublicParameters,
        ssk: &Self::SigningSecretKey,
        _spk: &Self::SigningPubKey,
        eks: &[Self::EncryptPubKey],
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
        let sig = Transcript::sign_contribution(ssk, dealer, aux, &V[sc.n]);

        Transcript {
            soks: vec![(*dealer, V[sc.n], sig, pok)],
            hat_w: g_2.mul(r),
            V,
            C,
            C_0: g_1.mul(r),
        }
    }

    fn get_dealers(&self) -> Vec<Player> {
        self.soks
            .iter()
            .map(|(p, _, _, _)| *p)
            .collect::<Vec<Player>>()
    }

    fn get_public_key_share(
        &self,
        _sc: &Self::SecretSharingConfig,
        player: &Player,
    ) -> Self::DealtPubKeyShare {
        Self::DealtPubKeyShare::new(Self::DealtPubKey::new(self.V[player.id]))
    }

    fn get_dealt_public_key(&self) -> Self::DealtPubKey {
        Self::DealtPubKey::new(*self.V.last().unwrap())
    }

    fn decrypt_own_share(
        &self,
        _sc: &Self::SecretSharingConfig,
        player: &Player,
        dk: &Self::DecryptPrivKey,
        _pp: &Self::PublicParameters,
    ) -> (Self::DealtSecretKeyShare, Self::DealtPubKeyShare) {
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
    fn generate<R>(
        sc: &Self::SecretSharingConfig,
        _pp: &Self::PublicParameters,
        rng: &mut R,
    ) -> Self
    where
        R: rand_core::RngCore + rand_core::CryptoRng,
    {
        let sk = bls12381::PrivateKey::genesis();
        Transcript {
            soks: vec![(
                sc.get_player(0),
                random_g2_point(rng),
                sk.sign(&Contribution::<G2Projective, usize> {
                    comm: random_g2_point(rng),
                    player: sc.get_player(0),
                    aux: 0,
                })
                .unwrap(),
                (random_g2_point(rng), random_scalar(rng)),
            )],
            hat_w: random_g2_point(rng),
            V: insecure_random_g2_points(sc.n + 1, rng),
            C: insecure_random_g1_points(sc.n, rng),
            C_0: random_g1_point(rng),
        }
    }
}

impl AggregatableTranscript for Transcript {
    fn verify<A: Serialize + Clone>(
        &self,
        sc: &<Self as traits::Transcript>::SecretSharingConfig,
        pp: &Self::PublicParameters,
        spks: &[Self::SigningPubKey],
        eks: &[Self::EncryptPubKey],
        auxs: &[A],
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

        // Deriving challenges by flipping coins: less complex to implement & less likely to get wrong. Creates bad RNG risks but we deem that acceptable.
        let mut rng = thread_rng();
        let extra = random_scalars(2, &mut rng);

        // Verify signature(s) on the secret commitment, player ID and `aux`
        let g_2 = *pp.get_commitment_base();
        batch_verify_soks::<G2Projective, A>(
            self.soks.as_slice(),
            &g_2,
            &self.V[sc.n],
            spks,
            auxs,
            &extra[0],
        )?;

        // Verify the committed polynomial is of the right degree
        let ldt = LowDegreeTest::random(
            &mut rng,
            sc.t,
            sc.n + 1,
            true,
            sc.get_batch_evaluation_domain(),
        );
        ldt.low_degree_test_on_g2(&self.V)?;

        //
        // Correctness of encryptions check
        //
        // (see [WVUF Overleaf](https://www.overleaf.com/project/63a1c2c222be94ece7c4b862) for
        //  explanation of how batching works)
        //

        // TODO(Performance): Change the Fiat-Shamir transform to use 128-bit random exponents.
        // r_i = \tau^i, \forall i \in [n]
        // TODO: benchmark this
        let taus = get_nonzero_powers_of_tau(&extra[1], sc.n);

        // Compute the multiexps from above.
        let v = g2_multi_exp(&self.V[..self.V.len() - 1], taus.as_slice());
        let ek = g1_multi_exp(
            eks.iter()
                .map(|ek| Into::<G1Projective>::into(ek))
                .collect::<Vec<G1Projective>>()
                .as_slice(),
            taus.as_slice(),
        );
        let c = g1_multi_exp(self.C.as_slice(), taus.as_slice());

        // Fetch some public parameters
        let h_1 = *pp.get_encryption_public_params().message_base();
        let g_1_inverse = pp.get_encryption_public_params().pubkey_base().neg();

        // The vector of left-hand-side ($\mathbb{G}_1$) inputs to each pairing in the multi-pairing.
        let lhs = [h_1, ek.add(g_1_inverse), self.C_0.add(c.neg())];
        // The vector of right-hand-side ($\mathbb{G}_2$) inputs to each pairing in the multi-pairing.
        let rhs = [v, self.hat_w, g_2];

        let res = multi_pairing(lhs.iter(), rhs.iter());
        if res != Gt::identity() {
            bail!("Expected zero, but got {} during multi-pairing check", res);
        }

        return Ok(());
    }
}

impl Aggregatable for Transcript {
    type Aggregated = Self;
    type SecretSharingConfig = ThresholdConfigBlstrs;

    fn to_aggregated(&self) -> Self::Aggregated {
        self.clone()
    }
}

impl Aggregated<Transcript> for Transcript {
    fn aggregate_with(
        &mut self,
        sc: &ThresholdConfigBlstrs,
        other: &Transcript,
    ) -> anyhow::Result<()> {
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

        Ok(())
    }

    fn normalize(self) -> Transcript {
        self
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
}
