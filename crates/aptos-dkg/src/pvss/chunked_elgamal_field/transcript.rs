// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::bail;

use serde::{Deserialize, Serialize};

use std::ops::{Mul, Sub};

use ff::Field;
use group::Group;
use crate::utils;

use ark_serialize::{CanonicalDeserialize, CanonicalSerialize, Compress, Validate};

use blstrs::{G1Projective, G2Projective, Gt, Scalar as ScalarOLD};
use aptos_crypto::{bls12381, CryptoMaterialError, ValidCryptoMaterial};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use ark_ec::pairing::Pairing;
use crate::pvss::chunked_elgamal_field;
use crate::pvss::chunked_elgamal_field::chunked_elgamal;
use crate::pvss::chunked_elgamal_field::chunks;
use crate::pvss::chunked_elgamal_field::consistency_proof::HkzgElgamalWitness;
use crate::sigma_protocol;
use crate::Scalar;
use crate::fiat_shamir;
use crate::utils::serialization::ark_se;
use crate::utils::serialization::ark_de;


use ark_std::UniformRand;
use crate::pvss::{chunked_elgamal_field::chunked_elgamal::PublicParameters as PublicParametersElgamal};
use crate::sigma_protocol::homomorphism::Trait;

use crate::sigma_protocol::homomorphism::tuple::TupleCodomainShape;


use crate::{
    algebra::polynomials::shamir_secret_share_ark, pvss::{
        self, chunked_elgamal_field::{
            consistency_proof,
            public_parameters::PublicParameters,
        }, encryption_dlog, traits::{transcript::MalleableTranscript, HasEncryptionPublicParams, SecretSharingConfig}, LowDegreeTest, Player, ThresholdConfig
    }, range_proofs::dekart_univariate_v2, utils::{powers, random
    }
};

/// Domain-separator tag (DST) for the Fiat-Shamir hashing used to derive randomness from the transcript.
pub const DST: &[u8; 32] = b"APTOS_CHUNK_EG_FIELD_PVSS_FS_DST";



#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)] // Removed BCSCryptoHash, CryptoHasher? Not compatible with <E: Pairing>
#[allow(non_snake_case)]
pub struct Transcript<E: Pairing> {
    dealers: Vec<Player>,
    /// Public key shares from 0 to n-1: V[i] = g_2^{s_i}; public key is in V[n]
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub V: Vec<E::G2>,
    /// First chunked ElGamal component: C[i][j] = g^{s_{i,j}} ek_i^{r_j}. Here s_i = \sum_j s_{i,j} B^j
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub C: Vec<Vec<E::G1>>,
    /// Second chunked ElGamal component: R[j] = h^{r_j}
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub R: Vec<E::G1>,
    /// Proof (of knowledge) showing that the s_{i,j}'s in C are base-B representations (of the s_i's in V, but this is not part of the proof), and that the r_j's in R are used in C
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub sharing_proof: Option<SharingProof<E>>, // Option because these proofs don't aggregate
}

#[derive(CanonicalSerialize, CanonicalDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct SharingProof<E: Pairing> {
    /// Sharing proof showing that the values in the range proof commitment and C and R come from the same s_{i,j} `witnesses`
    pub consistency_proof: sigma_protocol::Proof<E, consistency_proof::HkzgChunkedElgamalHomomorphism<'static, E>>,
    /// A batched range proof showing that all committed values s_{i,j} lie in some range
    pub range_proof: dekart_univariate_v2::Proof<E>,
    /// A KZG-style commitment to the values s_{i,j} going into the range proof
    pub range_proof_commitment: dekart_univariate_v2::Commitment<E>,
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
        bcs::from_bytes::<Transcript<E>>(bytes).map_err(|_| CryptoMaterialError::DeserializationError)
    }
}

impl<E: Pairing> pvss::Transcript for Transcript<E> 
    {
    type DealtPubKey = pvss::dealt_pub_key::g2::DealtPubKey;
    type DealtPubKeyShare = pvss::dealt_pub_key_share::g2::DealtPubKeyShare;
    type DealtSecretKey = Scalar<E>;
    type DealtSecretKeyShare = Scalar<E>;
    type DecryptPrivKey = chunked_elgamal::DecryptPrivKey<E>;
    type EncryptPubKey = chunked_elgamal::EncryptPubKey<E>;
    type InputSecret = chunked_elgamal_field::InputSecret<E::ScalarField>;
    type PublicParameters = PublicParameters<E>;
    type SecretSharingConfig = ThresholdConfig;
    type SigningPubKey = bls12381::PublicKey;
    type SigningSecretKey = bls12381::PrivateKey;

    fn scheme_name() -> String {
        "chunked_elgamal_field_pvss".to_string()
    }

    fn dst() -> Vec<u8> { b"APTOS_CHUNK_EG_FIELD_PVSS_FS_DST".to_vec() }

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
        let mut fs_transcript = fiat_shamir::initialize_pvss_transcript::<Self>(
            sc,
            pp,
            eks,
            DST,
        );

        // Generate Shamir secret sharing polynomial and evaluations (shares)
        let (f, mut f_evals) = shamir_secret_share_ark(sc, s, rng);
        debug_assert_eq!(f_evals.len(), sc.n);
        // Add constant term for commitment
        f_evals.push(f[0]);

        // Commit to polynomial evaluations + constant term
        let g_2 = pp.get_commitment_base();
        let V = utils::commit_to_scalars(g_2, &f_evals);
        debug_assert_eq!(V.len(), sc.n + 1);

        // Encrypt the chunked shares and generate the sharing proof
        let (C, R, sharing_proof) =
            encrypt_chunked_shares(pp, sc, eks, f_evals, &mut fs_transcript, rng);

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
        spks: &Vec<Self::SigningPubKey>,
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

        let mut fs_transcript = fiat_shamir::initialize_pvss_transcript::<Self>(sc, pp, eks, DST);

        if let Some(proof) = &self.sharing_proof {

            if let Err(err) = range_proof::batch_verify(
                &pp.pp_range_proof,
                &proof.range_proof_commitment,
                &proof.range_proof,
                &mut fs_transcript,
            ) {
                bail!("Range proof batch verification failed: {:?}", err);
            }

            // Need to have the exact same FS transcript here as the dealer
            consistency_proof::append_claim(&mut fs_transcript, &proof.range_proof_commitment.0, &self.C, &self.R);

            if let Err(err) = consistency_proof::verify(
                &consistency_proof::Homomorphism::new(pp, eks),
                &(proof.range_proof_commitment.0, self.C.clone(), self.R.clone()), // TODO: can probably get rid of clone with a struct + methods
                &proof.consistency_proof,
                &mut fs_transcript,
                (spks, &proof.consistency_proof.z),
                true,
            ) {
                bail!("Commitment consistency verification failed: {:?}", err);
            }

        } else {
            println!("There is no consistency proof");
        }

        <merlin::Transcript as fiat_shamir::PVSS<Transcript>>::append_share_commitments(
            &mut fs_transcript,
            &self.V,
        );

        let f = <merlin::Transcript as fiat_shamir::PVSS<Transcript>>::challenge_dual_code_word_polynomial(
            &mut fs_transcript,
            sc.t,
            sc.n + 1,
        );
        let ldt = LowDegreeTest::new(f, sc.t, sc.n + 1, true, sc.get_batch_evaluation_domain())?; // includes_zero here means it includes a commitment to f(0), which is in V[n]
        ldt.low_degree_test_on_g2(&self.V)?;

        {
            let mut base_vec = Vec::new();
            let mut exp_vec = Vec::new();

            let beta = 
                <merlin::Transcript as fiat_shamir::PVSS<Transcript>>::challenge_linear_combination_scalar(
                &mut fs_transcript,
                );
            let powers_of_beta = powers(beta, self.C.len() + 1);
        
            let weighted_Vs = g2_multi_exp(&self.V[..self.C.len()], &powers_of_beta[..self.C.len()]);
            // TODO: merge this multi_exp with the consistency proof computation as in YOLO YOSO?

            for i in 0..self.C.len() {
                for j in 0..self.C[0].len() {
                    let base = self.C[i][j];
                    let exp = pp.powers_of_radix[j] * powers_of_beta[i];
                    base_vec.push(base);
                    exp_vec.push(exp);
                }
            }

            let weighted_Cs = g1_multi_exp(&base_vec, &exp_vec);

            let res = commitment_pairing(
                weighted_Cs,
                weighted_Vs,
                pp.get_encryption_public_params().message_base(),
                pp.get_commitment_base(),
            );

            if res != Gt::identity() {
                bail!("Expected zero, but got {} during multi-pairing check", res);
            }
        }

        Ok(())
    }

    fn get_dealers(&self) -> Vec<Player> {
        self.dealers.clone()
    }

    fn aggregate_with(&mut self, sc: &Self::SecretSharingConfig, other: &Transcript) {
        debug_assert_eq!(self.C.len(), sc.n);
        debug_assert_eq!(self.V.len(), sc.n + 1);
        debug_assert_eq!(self.C.len(), other.C.len());
        debug_assert_eq!(self.V.len(), other.V.len());

        for i in 0..sc.n {
            self.V[i] += other.V[i];
            self.R[i] += other.R[i];
            for j in 0..self.C[i].len() {
                self.C[i][j] += other.C[i][j];
            }
        }
        self.V[sc.n] += other.V[sc.n];
        self.dealers.extend_from_slice(other.dealers.as_slice());

        self.sharing_proof = None; // the proofs don't aggregate
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

    #[allow(non_snake_case)]
    fn decrypt_own_share(
        &self,
        _sc: &Self::SecretSharingConfig,
        player: &Player,
        dk: &Self::DecryptPrivKey,
        pp: &Self::PublicParameters,
    ) -> (Self::DealtSecretKeyShare, Self::DealtPubKeyShare) {
        let chunk_size = pp.pp_range_proof.ell; // TODO: Not important but could also use build_constants::CHUNK_SIZE here

        let ctxts = &self.C[player.id];
        let ephemeral_keys: Vec<_> = self.R.iter().map(|Ri| Ri.mul(dk.dk)).collect();
        let dealt_encrypted_secret_key_share_chunks: Vec<_> = ctxts
            .iter()
            .zip(ephemeral_keys.iter())
            .map(|(Cij, Rj)| Cij.sub(Rj))
            .collect();

        #[cfg(feature = "kangaroo")]
        let kangaroo = match pp.table.as_ref() {
            Some(k) => k,
            None => panic!("sdfsdfsdfsdf"),
        };
        #[cfg(feature = "kangaroo")]
        let dealt_chunked_secret_key_share = kangaroo_dlog_vec(dealt_encrypted_secret_key_share_chunks, &kangaroo);
        
        #[cfg(not(any(feature = "bsgs", feature = "bsgs_with_phf", feature = "kangaroo")))]
        let dealt_chunked_secret_key_share = table_dlog::dlog_vec(dealt_encrypted_secret_key_share_chunks)
            .expect("discrete phf-only log failed");

        #[cfg(any(feature = "bsgs", feature = "bsgs_with_phf"))]
        let dealt_chunked_secret_key_share = bsgs::dlog_vec(dealt_encrypted_secret_key_share_chunks)
            .expect("discrete bsgs log failed");

        let dealt_secret_key_share = chunks::chunks_to_scalar(chunk_size, &dealt_chunked_secret_key_share)
            .expect("Failed to reconstruct secret key share from chunks");

        let dealt_pub_key_share = self.V[player.id]; // g_2^{f(\omega^i})

        (
            dealt_secret_key_share,
            Self::DealtPubKeyShare::new(Self::DealtPubKey::new(dealt_pub_key_share)),
        )
    }

    #[allow(non_snake_case)]
    fn generate<R>(sc: &Self::SecretSharingConfig, rng: &mut R) -> Self
    where
        R: rand_core::RngCore + rand_core::CryptoRng,
    {
        const SOME_NUMBER: usize = 16;

        Transcript {
            dealers: vec![sc.get_player(0)],
            V: random::insecure_random_g2_points(sc.n + 1, rng),
            C: (0..sc.n)
                .map(|_| random::insecure_random_g1_points(SOME_NUMBER, rng))
                .collect::<Vec<_>>(),
            R: random::insecure_random_g1_points(sc.n, rng),
            sharing_proof: Some(SharingProof {
                range_proof_commitment: range_proof::Commitment(random::random_g1_point(rng)),
                consistency_proof: consistency_proof::Proof {
                    first_stored_message: sigma_proof::FirstStoredMessage::Commitment((
                        random::random_g1_point(rng),
                        vec![vec![random::random_g1_point(rng); SOME_NUMBER]; sc.n],
                        vec![random::random_g1_point(rng); sc.n],
                    )),
                    z: (
                        random::random_scalar(rng),
                        vec![vec![random::random_scalar(rng); SOME_NUMBER]; sc.n],
                        vec![random::random_scalar(rng); sc.n],
                    ),
                },
                range_proof: range_proof::Proof {
                    d: random::random_g1_point(rng),
                    c: random::insecure_random_g1_points(SOME_NUMBER, rng),
                    c_hat: random::insecure_random_g2_points(SOME_NUMBER, rng),
                },
            }),
        }
    }
}

#[allow(non_snake_case)]
pub fn encrypt_chunked_shares<E: Pairing, R: rand_core::RngCore + rand_core::CryptoRng>(
    pp: &PublicParameters<E>,
    sc: &ThresholdConfig,
    eks: &[encryption_dlog::g1::EncryptPubKey],
    f_evals: Vec<E::ScalarField>,
    fs_transcript: &mut merlin::Transcript,
    rng: &mut R,
) -> (Vec<Vec<E::G1>>, Vec<E::G1>, SharingProof<E>) {
    let radix_exponent = pp.pk_range_proof.prover_precomputed.powers_of_two.len(); // TODO: not sure this is ideal, this is actually max_ell
    let number_of_chunks = E::ScalarField::MODULUS_BIT_SIZE.div_ceil(radix_exponent);

    let f_evals_chunked: Vec<Vec<E::ScalarField>> = f_evals
        .iter()
        .map(|f_eval| chunks::chunk_field_elt(radix_exponent, f_eval))
        .collect();

    let rs: Vec<Scalar<E>> = (0..number_of_chunks)
        .map(|_| Scalar::<E>::rand(rng))
        .collect();

    let hkzg_randomness = Scalar::<E>::rand(rng);

    // Do this now before f_evals_chunked is consumed
    let f_evals_chunked_flat: Vec<Scalar> = f_evals_chunked
        .iter()
        .flatten()
        .copied()
        .collect();

    let witness = HkzgElgamalWitness{ hkzg_randomness, chunked_plaintexts: Scalar::vecvec_from_inner(f_evals_chunked), elgamal_randomness: rs };
    let PublicParametersElgamal{g: g_1, h: h_1} = pp.pp_elgamal;
    let hom = consistency_proof::HkzgChunkedElgamalHomomorphism::new(&pp.pk_range_proof.ck_S.lagr_g1, pp.pk_range_proof.ck_S.xi_1, &g_1, &h_1, eks);

    let statement = hom.apply(&witness);
    let TupleCodomainShape(range_proof_commitment, chunked_elgamal::CodomainShape{chunks: Cs, randomness: Rs}) = statement;

    debug_assert_eq!(
        Cs.len(),
        sc.n,
        "Number of encryption keys must equal number of players"
    );

    // Generate the batch range proof
    let range_proof = dekart_univariate_v2::Proof::prove(pp.pk_range_proof, f_evals_chunked_flat, radix_exponent, dekart_univariate_v2::Commitment(range_proof_commitment), hkzg_randomness, fs_transcript);

    <merlin::Transcript as fiat_shamir::SigmaProtocol<E, consistency_proof::HkzgChunkedElgamalHomomorphism>>::append_sigma_protocol_public_statement(
        fs_transcript,
        statement,
    );

    let consistency_proof = hom.prove(
        witness,
        fs_transcript,
        rng,
    );

    // Assemble sharing proof
    let sharing_proof = SharingProof {
        consistency_proof,
        range_proof,
        range_proof_commitment,
    };

    (Cs, Rs, sharing_proof)
}

// #[allow(non_snake_case)]
// pub fn generate_chunked_shamir_shares_and_commitments<E: Pairing, R: rand_core::RngCore + rand_core::CryptoRng>(
//     pp: &PublicParameters<E>,
//     sc: &ThresholdConfig,
//     s: &pvss::input_secret::InputSecret,
//     rng: &mut R,
// ) -> (
//     Vec<E::ScalarField>,            // f_evals_with_const
//     Vec<Vec<E::ScalarField>>,       // f_evals_chunked
//     Vec<E::G2>,      // V (commitments to polynomial + constant term)
// ) {
//     // Retrieve parameters
//     let radix_exponent = pp.pk_range_proof.prover_precomputed.powers_of_two.len();

//     // Generate Shamir secret sharing polynomial and evaluations
//     let (f, mut f_evals) = shamir_secret_share_ark(sc, s, rng);
//     debug_assert_eq!(f_evals.len(), sc.n);

//     // Chunk polynomial evaluations (excluding constant term)
//     let f_evals_chunked: Vec<Vec<E::ScalarField>> = f_evals
//         .iter()
//         .map(|f_eval| chunks::chunk_field_elt(radix_exponent, f_eval))
//         .collect();

//     // Add constant term for commitment
//     f_evals.push(f[0]);

//     // Commit to polynomial evaluations + constant term
//     let g_2 = pp.get_commitment_base();
//     let V = utils::commit_to_scalars(g_2, &f_evals);
//     debug_assert_eq!(V.len(), sc.n + 1);

//     (f_evals, f_evals_chunked, V)
// }

// #[allow(non_snake_case)]
// pub fn generate_randomness_and_commitments<E: Pairing, R: rand_core::RngCore + rand_core::CryptoRng>(
//     rng: &mut R,
//     pp: &PublicParameters<E>,
// ) -> (Vec<ScalarOLD>, Vec<G1Projective>) {
//     let radix_exponent = pp.pp_range_proof.ell;
//     let number_of_chunks = 255_usize.div_ceil(radix_exponent);

//     // Generate correlated randomness for ElGamal encryption
//     let rs = range_proof::correlated_randomness(rng, 1 << radix_exponent, number_of_chunks, &ScalarOLD::ZERO);

//     // Compute the R commitments using public key base and randomness
//     let h_1 = pp.get_encryption_public_params().pubkey_base();
//     let Rs: Vec<G1Projective> = commit_to_scalars(h_1, &rs);
//     debug_assert_eq!(Rs.len(), number_of_chunks, "Number of randomness commitments must equal number of chunks");

//     (rs, Rs)
// }

// pub fn generate_batch_range_proof<R: rand_core::RngCore + rand_core::CryptoRng>(
//     sc: &ThresholdConfig,
//     f_evals_chunked: &[Vec<ScalarOLD>],
//     rng: &mut R,
//     pp: &PublicParameters,
//     fs_transcript: &mut merlin::Transcript,
// ) -> (
//     range_proof::Commitment,
//     ScalarOLD,
//     range_proof::Proof,
// ) {
//     // Setup range proof parameters
//     let b = pp.pp_range_proof.ell; // the radix exponent
//     let m = 255_usize.div_ceil(b); // number of chunks per scalar

//     // Flatten chunked polynomial evaluations for range proofs
//     let f_evals_chunked_flat: Vec<ScalarOLD> = f_evals_chunked
//         .iter()
//         .flatten()
//         .copied()
//         .collect();
//     debug_assert_eq!(
//         f_evals_chunked_flat.len(),
//         sc.n * m,
//         "Number of f_evals_chunked_flat must equal number of players times number of chunks"
//     );

//     // Create range proof commitment and hint randomness
//     let (range_proof_commitment, r_hint) =
//         range_proof::commit(&pp.pp_range_proof, &f_evals_chunked_flat, rng);

//     // Generate the batch range proof
//     let batch_range_proof = range_proof::batch_prove(
//         rng,
//         &pp.pp_range_proof,
//         &f_evals_chunked_flat,
//         &range_proof_commitment,
//         &r_hint,
//         fs_transcript,
//     );

//     (
//         range_proof_commitment,
//         r_hint,
//         batch_range_proof,
//     )
// }

#[cfg(feature = "kangaroo")]
fn kangaroo_dlog_vec(xs: Vec<G1Projective>, kangaroo: &Kangaroo<ActiveCurve>) -> Vec<ScalarOLD> {
    xs.into_iter()
        .map(|x| kangaroo_dlog(x, kangaroo))
        .collect()
}

#[cfg(feature = "kangaroo")]
fn kangaroo_dlog(x: G1Projective, kangaroo: &Kangaroo<ActiveCurve>) -> ScalarOLD {
    ScalarOLD::from(kangaroo.solve_dlp(&x, None).unwrap().unwrap())
}

impl MalleableTranscript for Transcript {
    fn maul_signature<A: Serialize + Clone>(
        &mut self,
        _ssk: &Self::SigningSecretKey,
        _aux: &A,
        player: &Player,
    ) {
        self.dealers = vec![*player];
    }
}

#[cfg(test)]
mod tests { // TODO: Update this stuff
    use super::*;
    use blstrs::ScalarOLD;
    use ff::Field;
    use rand::{rngs::StdRng, RngCore};
    use rand_core::{CryptoRng, SeedableRng};

    fn dummy_scalar_vec<R>(len: usize, mut rng: R) -> Vec<ScalarOLD>
    where
        R: RngCore + CryptoRng,
    {
        (0..len).map(|_| random::random_scalar(&mut rng)).collect()
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_reconstruct_ciphertexts_basic() {
        let mut rng = StdRng::from_seed([0u8; 32]);

        let num_secrets = 4;
        let num_chunks = 16;
        let b = 16;
        let B = 1usize << b;
        let m = num_chunks;

        let f_evals = dummy_scalar_vec(num_secrets, &mut rng);

        for (i, scalar) in f_evals.iter().enumerate() {
            let chunks = chunks::scalar_to_chunks(b, scalar);
            let reconstructed =
                chunks::chunks_to_scalar(b, &chunks).expect("chunks_to_scalar should not fail");

            assert_eq!(
                scalar, &reconstructed,
                "Reconstructed scalar does not match original at index {}",
                i
            );
        }

        let f_evals_chunked: Vec<Vec<ScalarOLD>> = f_evals
            .iter()
            .map(|scalar| chunks::scalar_to_chunks(b, scalar))
            .collect();

        let g_1 = random::random_g1_point(&mut rng);

        let rs =
            range_proof::correlated_randomness(&mut rng, B.try_into().unwrap(), m, &ScalarOLD::ZERO);

        assert_eq!(
            {
                let B_scalar = ScalarOLD::from(B as u64);
                let mut power = ScalarOLD::ONE;
                let mut sum = ScalarOLD::ZERO;

                for r in &rs {
                    sum += *r * power;
                    power *= B_scalar;
                }

                sum
            },
            ScalarOLD::ZERO,
            "Correlated randomness sum check failed"
        );

        let eks: Vec<encryption_dlog::g1::EncryptPubKey> = (0..num_secrets)
            .map(|_| encryption_dlog::g1::EncryptPubKey {
                ek: random::random_g1_point(&mut rng),
            })
            .collect();

        let Cs = chunks::compute_chunked_ciphertexts(&g_1, &rs, &eks, &f_evals_chunked, pvss::encryption_elgamal::g1::el_gamal_encrypt);

        let powers_of_B: Vec<ScalarOLD> = powers(ScalarOLD::from(B as u64), num_chunks);

        let Ci_vec = chunks::reconstruct_ciphertexts_from_chunks(&Cs, &powers_of_B, g1_multi_exp);

        for i in 0..num_secrets {
            assert_eq!(Ci_vec[i], g_1.mul(f_evals[i]));
        }
    }
}
