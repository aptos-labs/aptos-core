// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    dlog::bsgs,
    fiat_shamir,
    pcs::univariate_hiding_kzg,
    pvss::{
        chunky::{
            chunked_elgamal, chunked_elgamal::num_chunks_per_scalar, chunks, hkzg_chunked_elgamal,
            hkzg_chunked_elgamal::HkzgElgamalWitness, input_secret::InputSecret, keys,
            public_parameters::PublicParameters,
        },
        traits,
        traits::{transcript::MalleableTranscript, HasEncryptionPublicParams},
        Player,
    },
    range_proofs::{dekart_univariate_v2, traits::BatchedRangeProof},
    sigma_protocol,
    sigma_protocol::{
        homomorphism::{tuple::TupleCodomainShape, Trait as _},
        traits::Trait as _,
    },
    Scalar,
};
use anyhow::bail;
use aptos_crypto::{
    arkworks::{
        self,
        random::{
            sample_field_element, sample_field_elements, unsafe_random_point, unsafe_random_points,
            UniformRand,
        },
        scrape::LowDegreeTest,
        serialization::{ark_de, ark_se},
        shamir::ShamirThresholdConfig,
    },
    bls12381, utils, CryptoMaterialError, SecretSharingConfig, ValidCryptoMaterial,
};
use ark_ec::{
    pairing::{Pairing, PairingOutput},
    AffineRepr, CurveGroup, VariableBaseMSM,
};
use ark_ff::AdditiveGroup;
use ark_poly::EvaluationDomain;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use serde::{Deserialize, Serialize};
use std::ops::{Mul, Sub};

/// Domain-separator tag (DST) for the Fiat-Shamir hashing used to derive randomness from the transcript.
pub const DST: &[u8; 32] = b"APTOS_CHUNK_EG_FIELD_PVSS_FS_DST";

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)] // Removed CryptoHasher - not compatible with <E: Pairing> and doesn't seem to be used?
pub struct Transcript<E: Pairing> {
    dealers: Vec<Player>,
    /// Public key shares from 0 to n-1: V[i] = s_i * G_2; public key is in V[n]
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub V: Vec<E::G2>,
    /// First chunked ElGamal component: C[i][j] = s_{i,j} * G + r_j * ek_i. Here s_i = \sum_j s_{i,j} * B^j // TODO: change notation because B is not a group element?
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub C: Vec<Vec<E::G1>>, // TODO: maybe make this and the other fields affine? The verifier will have to do it anyway
    /// Second chunked ElGamal component: R[j] = r_j * H
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub R: Vec<E::G1>,
    /// Proof (of knowledge) showing that the s_{i,j}'s in C are base-B representations (of the s_i's in V, but this is not part of the proof), and that the r_j's in R are used in C
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub sharing_proof: Option<SharingProof<E>>, // Option because these proofs don't aggregate
}

#[allow(non_snake_case)]
#[derive(CanonicalSerialize, CanonicalDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct SharingProof<E: Pairing> {
    /// Proof showing knowledge of `witnesses` s_{i,j} yielding the commitment and the C and the R
    pub PoK: sigma_protocol::Proof<E, hkzg_chunked_elgamal::Homomorphism<'static, E>>, // static because we don't want the lifetime of the Proof to depend on the Homomorphism TODO: try removing it?
    /// A batched range proof showing that all committed values s_{i,j} lie in some range
    pub range_proof: dekart_univariate_v2::Proof<E>,
    /// A KZG-style commitment to the values s_{i,j} going into the range proof
    pub range_proof_commitment:
        <dekart_univariate_v2::Proof<E> as BatchedRangeProof<E>>::Commitment,
}

impl<E: Pairing> ValidCryptoMaterial for Transcript<E> {
    const AIP_80_PREFIX: &'static str = "";

    fn to_bytes(&self) -> Vec<u8> {
        bcs::to_bytes(&self).expect("unexpected error during PVSS transcript serialization")
    }
}

impl<E: Pairing> TryFrom<&[u8]> for Transcript<E> {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        bcs::from_bytes::<Transcript<E>>(bytes)
            .map_err(|_| CryptoMaterialError::DeserializationError)
    }
}

impl<E: Pairing> traits::Transcript for Transcript<E> {
    type DealtPubKey = keys::DealtPubKey<E>;
    type DealtPubKeyShare = keys::DealtPubKeyShare<E>;
    type DealtSecretKey = Scalar<E>;
    type DealtSecretKeyShare = Scalar<E>;
    type DecryptPrivKey = keys::DecryptPrivKey<E>;
    type EncryptPubKey = keys::EncryptPubKey<E>;
    type InputSecret = InputSecret<E::ScalarField>;
    type PublicParameters = PublicParameters<E>;
    type SecretSharingConfig = ShamirThresholdConfig<E::ScalarField>;
    type SigningPubKey = bls12381::PublicKey;
    // TODO!!!!!!!!!!!!!!!!!! NEXT PR
    type SigningSecretKey = bls12381::PrivateKey;

    // TODO!!!!!!!!!!!!!!!!!! NEXT PR

    fn scheme_name() -> String {
        "chunky_pvss".to_string()
    }

    fn dst() -> Vec<u8> {
        b"APTOS_CHUNK_EG_FIELD_PVSS_FS_DST".to_vec()
    }

    #[allow(non_snake_case)]
    fn deal<A: Serialize + Clone, R: rand_core::RngCore + rand_core::CryptoRng>(
        sc: &Self::SecretSharingConfig,
        pp: &Self::PublicParameters,
        _ssk: &Self::SigningSecretKey,
        eks: &Vec<Self::EncryptPubKey>,
        s: &Self::InputSecret,
        _aux: &A,
        dealer: &Player,
        rng: &mut R,
    ) -> Self {
        debug_assert_eq!(
            eks.len(),
            sc.n,
            "Number of encryption keys must equal number of players"
        );

        // Initialize the PVSS Fiat-Shamir transcript
        let mut fs_transcript =
            fiat_shamir::initialize_pvss_transcript::<E, Self>(sc, pp, eks, DST);

        // Generate the Shamir secret sharing polynomial
        let mut f = vec![*s.get_secret_a()]; // constant term of polynomial
        f.extend(sample_field_elements::<E::ScalarField, _>(
            sc.get_threshold() - 1,
            rng,
        )); // remaining coefficients; total degree is `t - 1`

        // Generate its `n` evaluations (shares)
        let mut f_evals = sc.domain.fft(&f);
        f_evals.truncate(sc.n);
        debug_assert_eq!(f_evals.len(), sc.n);

        // Encrypt the chunked shares and generate the sharing proof
        let (C, R, sharing_proof) =
            encrypt_chunked_shares(&f_evals, eks, sc, pp, &mut fs_transcript, rng);

        // Add constant term for the `\mathbb{G}_2` commitment below (we're doing this **after** the previous step because we're now mutating `f_evals` by enlarging it)
        f_evals.push(f[0]); // or *s.get_secret_a()
                            // Commit to polynomial evaluations + constant term
        let G_2 = pp.get_commitment_base();
        let V = arkworks::commit_to_scalars(&G_2, &f_evals);
        debug_assert_eq!(V.len(), sc.n + 1);

        // Return the transcript struct with all computed values
        Transcript {
            dealers: vec![*dealer],
            V,
            C,
            R,
            sharing_proof: Some(sharing_proof),
        }
    }

    #[allow(non_snake_case)]
    fn verify<A: Serialize + Clone>(
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
        if self.C.len() != sc.n {
            bail!(
                "Expected {} arrays of chunked ciphertexts, but got {}",
                sc.n,
                self.C.len()
            );
        }
        if self.V.len() != sc.n + 1 {
            bail!(
                "Expected {} commitment elements, but got {}",
                sc.n + 1,
                self.V.len()
            );
        }

        let mut fs_t = fiat_shamir::initialize_pvss_transcript::<E, Self>(sc, pp, eks, DST);

        if let Some(proof) = &self.sharing_proof {
            // Verify the PoK
            let eks_inner: Vec<_> = eks.iter().map(|ek| ek.ek).collect();
            let hom = hkzg_chunked_elgamal::Homomorphism::new(
                &pp.pk_range_proof.ck_S.lagr_g1,
                pp.pk_range_proof.ck_S.xi_1,
                &pp.pp_elgamal,
                &eks_inner,
            );
            if let Err(err) = hom.verify(
                &TupleCodomainShape(
                    proof.range_proof_commitment.clone(),
                    chunked_elgamal::CodomainShape {
                        chunks: self.C.clone(),
                        randomness: self.R.clone(),
                    },
                ),
                &proof.PoK,
                &mut fs_t,
            ) {
                bail!("PoK verification failed: {:?}", err);
            }

            // Verify the range proof
            if let Err(err) = proof.range_proof.verify(
                &pp.pk_range_proof.vk,
                sc.n * num_chunks_per_scalar::<E>(pp.ell) as usize,
                pp.ell as usize,
                &proof.range_proof_commitment,
                &mut fs_t,
            ) {
                bail!("Range proof batch verification failed: {:?}", err);
            }
        } else {
            println!("There is no sharing proof");
        }

        let mut rng = rand::thread_rng(); // TODO: make rng a parameter of fn verify()?

        // Do the SCRAPE LDT
        let ldt = LowDegreeTest::random(&mut rng, sc.t, sc.n + 1, true, &sc.domain); // includes_zero is true here means it includes a commitment to f(0), which is in V[n]
        ldt.low_degree_test_group(&self.V)?;

        // Now compute the final MSM // TODO: merge this multi_exp with the PoK verification, as in YOLO YOSO?
        let mut base_vec = Vec::new();
        let mut exp_vec = Vec::new();

        let beta = sample_field_element(&mut rng);
        let powers_of_beta = utils::powers(beta, self.C.len() + 1);

        for i in 0..self.C.len() {
            for j in 0..self.C[i].len() {
                let base = self.C[i][j];
                let exp = pp.powers_of_radix[j] * powers_of_beta[i];
                base_vec.push(base);
                exp_vec.push(exp);
            }
        }

        let weighted_Cs = E::G1::msm(&E::G1::normalize_batch(&base_vec), &exp_vec)
            .expect("Failed to compute MSM of Cs in chunky");

        let weighted_Vs = E::G2::msm(
            &E::G2::normalize_batch(&self.V[..self.C.len()]),
            &powers_of_beta[..self.C.len()],
        )
        .expect("Failed to compute MSM of Vs in chunky");

        let res = E::multi_pairing(
            [
                weighted_Cs.into_affine(),
                *pp.get_encryption_public_params().message_base(),
            ],
            [pp.get_commitment_base(), (-weighted_Vs).into_affine()],
        ); // Making things affine here rather than converting the two bases to group elements, since that's probably what they would be converted to anyway: https://github.com/arkworks-rs/algebra/blob/c1f4f5665504154a9de2345f464b0b3da72c28ec/ec/src/models/bls12/g1.rs#L14

        assert_eq!(
            PairingOutput::<E>::ZERO,
            res,
            "Expected zero during multi-pairing check",
        );

        Ok(())
    }

    fn get_dealers(&self) -> Vec<Player> {
        self.dealers.clone()
    }

    fn aggregate_with(&mut self, sc: &Self::SecretSharingConfig, other: &Transcript<E>) {
        debug_assert_eq!(self.C.len(), sc.n);
        debug_assert_eq!(self.V.len(), sc.n + 1);
        debug_assert_eq!(self.C.len(), other.C.len());
        debug_assert_eq!(self.R.len(), other.R.len());
        debug_assert_eq!(self.V.len(), other.V.len());

        for i in 0..sc.n {
            self.V[i] += other.V[i];
            self.R[i] += other.R[i];
            for j in 0..self.C[i].len() {
                self.C[i][j] += other.C[i][j];
            }
        }
        self.V[sc.n] += other.V[sc.n];

        for (r_self, r_other) in self.R.iter_mut().zip(&other.R) {
            *r_self += r_other;
        }

        self.dealers.extend_from_slice(other.dealers.as_slice());

        self.sharing_proof = None; // the proofs don't aggregate
    }

    fn get_public_key_share(
        &self,
        _sc: &Self::SecretSharingConfig,
        player: &Player,
    ) -> Self::DealtPubKeyShare {
        Self::DealtPubKeyShare::new(Self::DealtPubKey::new(self.V[player.id].into_affine()))
    }

    fn get_dealt_public_key(&self) -> Self::DealtPubKey {
        Self::DealtPubKey::new(self.V.last().expect("V is empty somehow").into_affine())
    }

    #[allow(non_snake_case)]
    fn decrypt_own_share(
        &self,
        _sc: &Self::SecretSharingConfig,
        player: &Player,
        dk: &Self::DecryptPrivKey,
        pp: &Self::PublicParameters,
    ) -> (Self::DealtSecretKeyShare, Self::DealtPubKeyShare) {
        let C_i = &self.C[player.id]; // where i = player.id

        let ephemeral_keys: Vec<_> = self.R.iter().map(|Ri| Ri.mul(dk.dk)).collect();
        assert_eq!(
            ephemeral_keys.len(),
            C_i.len(),
            "Number of ephemeral keys does not match the number of ciphertext chunks"
        );
        let dealt_encrypted_secret_key_share_chunks: Vec<_> = C_i
            .iter()
            .zip(ephemeral_keys.iter())
            .map(|(C_ij, ephemeral_key)| C_ij.sub(ephemeral_key))
            .collect();

        let dealt_chunked_secret_key_share = bsgs::dlog_vec(
            pp.pp_elgamal.G.into_group(),
            &dealt_encrypted_secret_key_share_chunks,
            &pp.table,
            1 << pp.ell as u32,
        )
        .expect("BSGS dlog failed");

        let dealt_chunked_secret_key_share_fr: Vec<E::ScalarField> = dealt_chunked_secret_key_share
            .iter()
            .map(|&x| E::ScalarField::from(x))
            .collect();

        let dealt_secret_key_share =
            chunks::le_chunks_to_scalar(pp.ell, &dealt_chunked_secret_key_share_fr);

        let dealt_pub_key_share = self.V[player.id].into_affine(); // G_2^{f(\omega^i})

        (
            Scalar(dealt_secret_key_share),
            Self::DealtPubKeyShare::new(Self::DealtPubKey::new(dealt_pub_key_share)), // TODO: review this formalism
        )
    }

    #[allow(non_snake_case)]
    fn generate<R>(sc: &Self::SecretSharingConfig, pp: &Self::PublicParameters, rng: &mut R) -> Self
    where
        R: rand_core::RngCore + rand_core::CryptoRng,
    {
        let num_chunks_per_share = num_chunks_per_scalar::<E>(pp.ell) as usize;
        Transcript {
            dealers: vec![sc.get_player(0)],
            V: unsafe_random_points::<E::G2, _>(sc.n + 1, rng),
            C: (0..sc.n)
                .map(|_| unsafe_random_points(num_chunks_per_share, rng))
                .collect::<Vec<_>>(),
            R: unsafe_random_points(num_chunks_per_share, rng),
            sharing_proof: Some(SharingProof {
                range_proof_commitment: sigma_protocol::homomorphism::TrivialShape(
                    unsafe_random_point(rng),
                ),
                PoK: hkzg_chunked_elgamal::Proof::generate(
                    pp.pk_range_proof.max_n,
                    num_chunks_per_share,
                    rng,
                ),
                range_proof: dekart_univariate_v2::Proof::generate(pp.ell, rng),
            }),
        }
    }
}

#[allow(non_snake_case)]
pub fn encrypt_chunked_shares<E: Pairing, R: rand_core::RngCore + rand_core::CryptoRng>(
    f_evals: &[E::ScalarField],
    eks: &[keys::EncryptPubKey<E>],
    sc: &ShamirThresholdConfig<E::ScalarField>,
    pp: &PublicParameters<E>,
    fs_transcript: &mut merlin::Transcript,
    rng: &mut R,
) -> (Vec<Vec<E::G1>>, Vec<E::G1>, SharingProof<E>) {
    // Generate the required randomness
    let hkzg_randomness = univariate_hiding_kzg::CommitmentRandomness::rand(rng);

    let elgamal_randomness = Scalar::vec_from_inner(chunked_elgamal::correlated_randomness(
        rng,
        1 << pp.ell as u64,
        num_chunks_per_scalar::<E>(pp.ell),
    ));

    // Chunk and flatten the shares
    let f_evals_chunked: Vec<Vec<E::ScalarField>> = f_evals
        .iter()
        .map(|f_eval| chunks::scalar_to_le_chunks(pp.ell, f_eval))
        .collect();
    // Flatten it now before `f_evals_chunked` is consumed in the next step
    let f_evals_chunked_flat: Vec<E::ScalarField> =
        f_evals_chunked.iter().flatten().copied().collect();

    // Now generate the encrypted shares and range proof commitment, together with its PoK, so:
    // (1) Set up the witness
    let witness = HkzgElgamalWitness {
        hkzg_randomness,
        chunked_plaintexts: Scalar::vecvec_from_inner(f_evals_chunked),
        elgamal_randomness,
    };
    // (2) Compute its image under the corresponding homomorphism, and prove knowledge of an inverse
    //   (2a) Set up the tuple homomorphism
    let eks_inner: Vec<_> = eks.iter().map(|ek| ek.ek).collect(); // TODO: this is a bit ugly
    let hom = hkzg_chunked_elgamal::Homomorphism::new(
        &pp.pk_range_proof.ck_S.lagr_g1,
        pp.pk_range_proof.ck_S.xi_1,
        &pp.pp_elgamal,
        &eks_inner,
    );
    //   (2b) Compute its image, so the range proof commitment and chunked_elgamal encryptions
    let statement = hom.apply(&witness);
    //   (2c) Prove knowledge of its inverse
    let PoK = hom
        .prove(&witness, &statement, fs_transcript, rng)
        .change_lifetime(); // Make sure the lifetime of the proof is not coupled to `hom`

    // Destructure the "public statement" of the above sigma protocol
    let TupleCodomainShape(
        range_proof_commitment,
        chunked_elgamal::CodomainShape {
            chunks: Cs,
            randomness: Rs,
        },
    ) = statement;

    debug_assert_eq!(
        Cs.len(),
        sc.n,
        "Number of encrypted chunks must equal number of players"
    );

    // Generate the batch range proof, given the `range_proof_commitment` produced during the PoK
    let range_proof = dekart_univariate_v2::Proof::prove(
        &pp.pk_range_proof,
        &f_evals_chunked_flat,
        pp.ell as usize,
        &range_proof_commitment,
        &hkzg_randomness,
        fs_transcript,
        rng,
    );

    // Assemble the sharing proof
    let sharing_proof = SharingProof {
        PoK,
        range_proof,
        range_proof_commitment,
    };

    (Cs, Rs, sharing_proof)
}

impl<E: Pairing> MalleableTranscript for Transcript<E> {
    fn maul_signature<A: Serialize + Clone>(
        &mut self,
        _ssk: &Self::SigningSecretKey,
        _aux: &A,
        player: &Player,
    ) {
        self.dealers = vec![*player]; // TODO!!!!!!!!!!!!!!!!!! NEXT PR
    }
}
