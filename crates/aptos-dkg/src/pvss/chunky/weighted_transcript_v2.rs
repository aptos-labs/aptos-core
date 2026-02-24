// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Weighted chunky PVSS transcript (v2): SCRAPE LDT + PoK with G2-side MSM merge (no pairing in verify).

use crate::{
    delegate_transcript_core_to_subtrs,
    pcs::univariate_hiding_kzg,
    pvss::{
        chunky::{
            chunked_elgamal::{self, num_chunks_per_scalar},
            chunked_scalar_mul, chunks,
            hkzg_chunked_elgamal::HkzgWeightedElgamalWitness,
            hkzg_chunked_elgamal_commit,
            input_secret::InputSecret,
            keys,
            public_parameters::PublicParameters,
            subtranscript::Subtranscript,
            verify_common::{verify_weighted_preamble, SokContext},
        },
        traits::{
            self,
            transcript::{HasAggregatableSubtranscript, MalleableTranscript},
        },
        Player,
    },
    range_proofs::{dekart_univariate_v2, traits::BatchedRangeProof},
    sigma_protocol::{
        self, check_msm_eval_zero,
        homomorphism::{
            fixed_base_msms::Trait as MsmTrait, tuple::TupleCodomainShape, Trait as HomTrait,
            TrivialShape,
        },
        verifier_challenges_with_length, CurveGroupTrait, Trait as _,
    },
    Scalar,
};
use anyhow::bail;
use aptos_crypto::{
    arkworks::{
        msm,
        random::{
            sample_field_element, sample_field_elements, unsafe_random_point,
            unsafe_random_point_group, unsafe_random_points, UniformRand,
        },
        scrape::LowDegreeTest,
        serialization::{ark_de, ark_se},
        srs::SrsBasis,
    },
    bls12381::{self},
    weighted_config::WeightedConfigArkworks,
    CryptoMaterialError, TSecretSharingConfig, ValidCryptoMaterial,
};
use ark_ec::{pairing::Pairing, CurveGroup, VariableBaseMSM};
use ark_ff::{AdditiveGroup, Field, Fp, FpConfig};
use ark_poly::EvaluationDomain;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use rand_core::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};

/// Domain-separation tag (DST) used to ensure that all cryptographic hashes and
/// transcript operations within the protocol are uniquely namespaced
pub const DST: &[u8; 42] = b"APTOS_WEIGHTED_CHUNKY_FIELD_PVSS_v2_FS_DST";

/// Weighted chunky PVSS transcript, does not use pairings in the verifier (only indirectly in the range proof).
#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Transcript<E: Pairing> {
    dealer: Player,
    /// This is the aggregatable subtranscript
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub subtrs: Subtranscript<E>,
    /// Proof (of knowledge) showing that the s_{i,j}'s in C are base-B representations (of the s_i's in V, but this is not part of the proof), and that the r_j's in R are used in C
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub sharing_proof: SharingProof<E>,
}

/// This is the secret sharing config that will be used for weighted `chunky`
/// TODO: remove after merging subtranscript trait
#[allow(type_alias_bounds)]
type SecretSharingConfig<E: Pairing> = WeightedConfigArkworks<E::ScalarField>;

impl<const N: usize, P: FpConfig<N>, E: Pairing<ScalarField = Fp<P, N>>>
    HasAggregatableSubtranscript for Transcript<E>
{
    type Subtranscript = Subtranscript<E>;

    fn get_subtranscript(&self) -> Self::Subtranscript {
        self.subtrs.clone()
    }

    #[allow(non_snake_case)]
    fn verify<A: Serialize + Clone, R: RngCore + CryptoRng>(
        &self,
        sc: &Self::SecretSharingConfig,
        pp: &Self::PublicParameters,
        spks: &[Self::SigningPubKey],
        eks: &[Self::EncryptPubKey],
        sid: &A,
        rng: &mut R,
    ) -> anyhow::Result<()> {
        let sok_cntxt = verify_weighted_preamble(
            sc,
            &self.subtrs,
            &self.dealer,
            spks,
            eks,
            sid,
            <Self as traits::Transcript>::dst(),
        )?;

        {
            // Verify the range proof
            if let Err(err) = self.sharing_proof.range_proof.verify(
                &pp.pk_range_proof.vk,
                sc.get_total_weight() * num_chunks_per_scalar::<E::ScalarField>(pp.ell) as usize,
                pp.ell,
                &self.sharing_proof.range_proof_commitment,
                rng,
            ) {
                bail!("Range proof batch verification failed: {:?}", err);
            }
        }

        // Do the SCRAPE LDT
        let ldt = LowDegreeTest::random(
            rng,
            sc.get_threshold_weight(),
            sc.get_total_weight() + 1,
            true,
            &sc.get_threshold_config().domain,
        ); // includes_zero is true here means it includes a commitment to f(0), which is in V[n]
           // Collect affine elements for LDT (which expects affine)
        let mut Vs_flat: Vec<E::G2Affine> = self.subtrs.Vs.iter().flatten().copied().collect();
        Vs_flat.push(self.subtrs.V0);
        // could add an assert_eq here with sc.get_total_weight()
        let ldt_msm_terms = ldt.ldt_msm_input::<E::G2>(&Vs_flat)?;

        let eks_inner: Vec<_> = eks.iter().map(|ek| ek.ek).collect();
        let lagr_g1: &[E::G1Affine] = match &pp.pk_range_proof.ck_S.msm_basis {
            SrsBasis::Lagrange { lagr: lagr_g1 } => lagr_g1,
            SrsBasis::PowersOfTau { .. } => {
                bail!("Expected a Lagrange basis, received powers of tau basis instead")
            },
        };
        let hom = hkzg_chunked_elgamal_commit::Homomorphism::<E>::new(
            lagr_g1,
            pp.pk_range_proof.ck_S.xi_1,
            &pp.pp_elgamal,
            &pp.G2_table,
            &eks_inner,
            pp.get_commitment_base(),
            pp.ell,
        );

        let num_chunks = num_chunks_per_scalar::<E::ScalarField>(pp.ell) as usize;
        let total_weight = sc.get_total_weight();
        // First component length: 1 (TrivialShape) + chunks (total_weight*num_chunks) + randomness (max_weight*num_chunks), matching WeightedCodomainShape::into_iter
        let first_len = 1 + total_weight * num_chunks + sc.get_max_weight() * num_chunks;
        let public_statement = TupleCodomainShape(
            TupleCodomainShape(
                sigma_protocol::homomorphism::TrivialShape(
                    self.sharing_proof.range_proof_commitment.0.into_affine(), // Because it's not affine by default. Should probably change that
                ),
                chunked_elgamal::WeightedCodomainShape {
                    chunks: self.subtrs.Cs.clone(),
                    randomness: self.subtrs.Rs.clone(),
                },
            ),
            chunked_scalar_mul::CodomainShape(self.subtrs.Vs.iter().flatten().cloned().collect()),
        );
        let prover_first_message = self
            .sharing_proof
            .SoK
            .prover_commitment()
            .expect("SoK must contain commitment for Fiat–Shamir");
        let (c, powers_of_beta) = verifier_challenges_with_length::<_, E::ScalarField, _, _>(
            &sok_cntxt,
            &hom,
            &public_statement,
            prover_first_message,
            &sigma_protocol::Trait::dst(&hom),
            first_len + total_weight,
            rng,
        );

        let first_terms = hom.hom1.msm_terms(&self.sharing_proof.SoK.z);
        let first_msm_terms =
            hkzg_chunked_elgamal_commit::HkzgElgamalHomomorphism::<E>::merge_msm_terms(
                first_terms.into_iter().collect(),
                &prover_first_message.0,
                &public_statement.0,
                &powers_of_beta[..first_len],
                c,
            );
        check_msm_eval_zero(&hom.hom1, first_msm_terms)?;

        let second_terms = hom.hom2.msm_terms(&self.sharing_proof.SoK.z);
        let second_msm_terms = hkzg_chunked_elgamal_commit::LiftedCommitHomomorphism::<
            'static,
            E::G2,
        >::merge_msm_terms(
            second_terms.into_iter().collect(),
            &prover_first_message.1,
            &public_statement.1,
            &powers_of_beta[first_len..],
            c,
        );

        let beta = sample_field_element(rng);
        let merged_g2 =
            msm::merge_scaled_msm_terms::<E::G2>(&[&second_msm_terms, &ldt_msm_terms], &[
                E::ScalarField::ONE,
                beta,
            ]);
        let g2_msm = E::G2::msm(merged_g2.bases(), merged_g2.scalars())
            .expect("Failed to compute merged G2 MSM in chunky v2");
        if g2_msm != E::G2::ZERO {
            bail!("G2 MSM check failed (expected zero)");
        }

        Ok(())
    }
}

/// Proof that chunked ciphertexts and commitments are consistent (SoK + batched range proof).
#[allow(non_snake_case)]
#[derive(CanonicalSerialize, CanonicalDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct SharingProof<E: Pairing> {
    /// SoK: the SK is knowledge of `witnesses` s_{i,j} yielding the commitment and the C and the R, their image is the PK, and the signed message is a certain context `cntxt`
    pub SoK: hkzg_chunked_elgamal_commit::Proof<'static, E>, // static because we don't want the lifetime of the Proof to depend on the Homomorphism TODO: try removing it?
    /// A batched range proof showing that all committed values s_{i,j} lie in some range
    pub range_proof: dekart_univariate_v2::Proof<E>, // TODO: make an affine version of this
    /// A KZG-style commitment to the values s_{i,j} going into the range proof
    pub range_proof_commitment:
        <dekart_univariate_v2::Proof<E> as BatchedRangeProof<E>>::Commitment,
}

impl<E: Pairing> ValidCryptoMaterial for Transcript<E> {
    const AIP_80_PREFIX: &'static str = "";

    fn to_bytes(&self) -> Vec<u8> {
        // TODO: using `Result<Vec<u8>>` and `.map_err(|_| CryptoMaterialError::DeserializationError)` would be more consistent here?
        bcs::to_bytes(&self).expect("Unexpected error during PVSS transcript serialization")
    }
}

impl<E: Pairing> TryFrom<&[u8]> for Transcript<E> {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        bcs::from_bytes::<Transcript<E>>(bytes)
            .map_err(|_| CryptoMaterialError::DeserializationError)
    }
}

delegate_transcript_core_to_subtrs!(Transcript<E>, subtrs);

impl<const N: usize, P: FpConfig<N>, E: Pairing<ScalarField = Fp<P, N>>> traits::Transcript
    for Transcript<E>
{
    type InputSecret = InputSecret<E::ScalarField>;
    type SigningPubKey = bls12381::PublicKey;
    type SigningSecretKey = bls12381::PrivateKey;

    fn scheme_name() -> String {
        "chunky_v2".to_string()
    }

    /// Fetches the domain-separation tag (DST)
    fn dst() -> Vec<u8> {
        DST.to_vec()
    }

    #[allow(non_snake_case)]
    fn deal<A: Serialize + Clone, R: RngCore + CryptoRng>(
        sc: &Self::SecretSharingConfig,
        pp: &Self::PublicParameters,
        _ssk: &Self::SigningSecretKey,
        spk: &Self::SigningPubKey,
        eks: &[Self::EncryptPubKey],
        s: &Self::InputSecret,
        session_id: &A,
        dealer: &Player,
        rng: &mut R,
    ) -> Self {
        debug_assert_eq!(
            eks.len(),
            sc.get_total_num_players(),
            "Number of encryption keys must equal total weight"
        );

        // Initialize the PVSS SoK context
        let sok_cntxt = SokContext::new(spk.clone(), session_id, dealer.id, Self::dst());

        // Generate the Shamir secret sharing polynomial
        let mut f = vec![*s.get_secret_a()]; // constant term of polynomial
        f.extend(sample_field_elements::<E::ScalarField, _>(
            sc.get_threshold_weight() - 1,
            rng,
        )); // these are the remaining coefficients; total degree is `t - 1`, so the reconstruction threshold is `t`

        // Generate its `n` evaluations (shares) by doing an FFT over the whole domain, then truncating
        let mut f_evals = sc.get_threshold_config().domain.fft(&f);
        f_evals.truncate(sc.get_total_weight());
        debug_assert_eq!(f_evals.len(), sc.get_total_weight());

        // Encrypt the chunked shares and generate the sharing proof
        let (Cs, Rs, Vs, sharing_proof) =
            Self::encrypt_chunked_shares(&f_evals, eks, pp, sc, sok_cntxt, rng);

        // Compute V0 (projective)
        let V0_proj = pp.get_commitment_base() * f[0];

        Transcript {
            dealer: *dealer,
            subtrs: Subtranscript {
                V0: V0_proj.into_affine(),
                Vs,
                Cs,
                Rs,
            },
            sharing_proof,
        }
    }

    fn get_dealers(&self) -> Vec<Player> {
        vec![self.dealer]
    }

    #[allow(non_snake_case)]
    fn generate<R: RngCore + CryptoRng>(
        sc: &Self::SecretSharingConfig,
        pp: &Self::PublicParameters,
        rng: &mut R,
    ) -> Self {
        let num_chunks_per_share = num_chunks_per_scalar::<E::ScalarField>(pp.ell) as usize;

        let V0 = unsafe_random_point::<E::G2, _>(rng);
        let Vs: Vec<Vec<E::G2Affine>> = (0..sc.get_total_num_players())
            .map(|i| {
                let w = sc.get_player_weight(&sc.get_player(i));
                unsafe_random_points::<E::G2, _>(w, rng)
            })
            .collect();
        let Cs: Vec<Vec<Vec<E::G1Affine>>> = (0..sc.get_total_num_players())
            .map(|i| {
                let w = sc.get_player_weight(&sc.get_player(i));
                (0..w)
                    .map(|_| unsafe_random_points::<E::G1, _>(num_chunks_per_share, rng))
                    .collect()
            })
            .collect();
        let Rs: Vec<Vec<E::G1Affine>> = (0..sc.get_max_weight())
            .map(|_| unsafe_random_points::<E::G1, _>(num_chunks_per_share, rng))
            .collect();

        Transcript {
            dealer: sc.get_player(0),
            subtrs: Subtranscript { V0, Vs, Cs, Rs },
            sharing_proof: SharingProof {
                range_proof_commitment: sigma_protocol::homomorphism::TrivialShape(
                    unsafe_random_point_group::<E::G1, _>(rng),
                ),
                SoK: hkzg_chunked_elgamal_commit::Proof::generate(sc, num_chunks_per_share, rng),
                range_proof: dekart_univariate_v2::Proof::generate(pp.ell, rng),
            },
        }
    }
}

impl<const N: usize, P: FpConfig<N>, E: Pairing<ScalarField = Fp<P, N>>> Transcript<E> {
    /// Encrypts chunked shares and builds the sharing proof (SoK + range proof).
    /// Panics if `pp.pk_range_proof.ck_S` is not a Lagrange SRS basis (same requirement as verify).
    #[allow(non_snake_case)]
    pub fn encrypt_chunked_shares<'a, A: Serialize + Clone, R: RngCore + CryptoRng>(
        f_evals: &[E::ScalarField],
        eks: &[keys::EncryptPubKey<E>],
        pp: &PublicParameters<E>,
        sc: &<Self as traits::TranscriptCore>::SecretSharingConfig, // only for debugging purposes?
        sok_cntxt: SokContext<'a, A>,
        rng: &mut R,
    ) -> (
        Vec<Vec<Vec<E::G1Affine>>>,
        Vec<Vec<E::G1Affine>>,
        Vec<Vec<E::G2Affine>>,
        SharingProof<E>,
    ) {
        // Generate the required randomness
        let hkzg_randomness = univariate_hiding_kzg::CommitmentRandomness::rand(rng);
        let elgamal_randomness = Scalar::vecvec_from_inner(
            (0..sc.get_max_weight())
                .map(|_| {
                    chunked_elgamal::correlated_randomness(
                        rng,
                        1 << pp.ell as u64,
                        num_chunks_per_scalar::<E::ScalarField>(pp.ell),
                        &E::ScalarField::ZERO,
                    )
                })
                .collect(),
        );

        // Chunk and flatten the shares
        let f_evals_chunked: Vec<Vec<E::ScalarField>> = f_evals
            .iter()
            .map(|f_eval| chunks::scalar_to_le_chunks(pp.ell, f_eval))
            .collect();
        // Flatten it now (for use in the range proof) before `f_evals_chunked` is consumed in the next step
        let f_evals_chunked_flat: Vec<E::ScalarField> =
            f_evals_chunked.iter().flatten().copied().collect();
        // Separately, gather the chunks by weight
        let f_evals_weighted = sc.group_by_player(&f_evals_chunked);

        // Now generate the encrypted shares and range proof commitment, together with its SoK, so:
        // (1) Set up the witness
        let witness = HkzgWeightedElgamalWitness {
            hkzg_randomness,
            chunked_plaintexts: Scalar::vecvecvec_from_inner(f_evals_weighted),
            elgamal_randomness,
        };
        // (2) Compute its image under the corresponding homomorphism, and produce an SoK
        //   (2a) Set up the tuple homomorphism
        let eks_inner: Vec<_> = eks.iter().map(|ek| ek.ek).collect(); // TODO: this is a bit ugly
        let lagr_g1: &[E::G1Affine] = match &pp.pk_range_proof.ck_S.msm_basis {
            SrsBasis::Lagrange { lagr: lagr_g1 } => lagr_g1,
            SrsBasis::PowersOfTau { .. } => {
                panic!("Expected a Lagrange basis, received powers of tau basis instead")
            },
        };
        let hom = hkzg_chunked_elgamal_commit::Homomorphism::<E>::new(
            lagr_g1,
            pp.pk_range_proof.ck_S.xi_1,
            &pp.pp_elgamal,
            &pp.G2_table,
            &eks_inner,
            pp.get_commitment_base(),
            pp.ell,
        );
        //   (2b) Compute its image (the public statement), so the range proof commitment and chunked_elgamal encryptions
        let statement = hom.apply(&witness); // hmm slightly inefficient that we're unchunking here, so might be better to set up a "small" hom just for this part
                                             //   (2c) Produce the SoK
        let (SoK, normalized_statement) = hom.prove(&witness, statement, &sok_cntxt, rng);
        let SoK = SoK.change_lifetime(); // Make sure the lifetime of the proof is not coupled to `hom` which has references

        // Destructure the "public statement" of the above sigma protocol
        let TupleCodomainShape(
            TupleCodomainShape(
                range_proof_commitment,
                chunked_elgamal::WeightedCodomainShape {
                    chunks: Cs,
                    randomness: Rs,
                },
            ),
            chunked_scalar_mul::CodomainShape(Vs_flat),
        ) = normalized_statement;

        // Group Vs by player (convert flat Vec<E::G2> to Vec<Vec<E::G2>>)
        // Vs_flat is the inner Vec<E::G2> from CodomainShape
        let Vs: Vec<Vec<E::G2Affine>> = sc.group_by_player(&Vs_flat);

        // debug_assert_eq!(
        //     Cs.len(),
        //     sc.get_total_weight(),
        //     "Number of encrypted chunks must equal number of players"
        // );

        // Generate the batch range proof, given the `range_proof_commitment` produced in the PoK
        let range_proof = dekart_univariate_v2::Proof::prove(
            &pp.pk_range_proof,
            &f_evals_chunked_flat,
            pp.ell,
            &TrivialShape(range_proof_commitment.0.into()), // TODO: fix this
            &hkzg_randomness,
            rng,
        );

        // Assemble the sharing proof
        let sharing_proof = SharingProof {
            SoK,
            range_proof,
            range_proof_commitment: TrivialShape(range_proof_commitment.0.into()), // TODO: fix this
        };

        // Vs_flat from homomorphism codomain was grouped by player into Vs above.

        (Cs, Rs, Vs, sharing_proof)
    }
}

impl<const N: usize, P: FpConfig<N>, E: Pairing<ScalarField = Fp<P, N>>> MalleableTranscript
    for Transcript<E>
{
    fn maul_signature<A: Serialize + Clone>(
        &mut self,
        _ssk: &Self::SigningSecretKey,
        _aux: &A,
        _player: &Player,
    ) {
        // TODO: We're not using this but it probably fails if we don't; but that would probably mean recomputing almost the entire transcript... but then that would require eks and pp
        panic!("Doesn't work for this PVSS, at least for now");
        // self.dealer = *player;

        // let sgn = ssk
        //     .sign(&self.utrs)
        //     .expect("signing of `chunky` PVSS transcript failed");
        // self.sgn = sgn;
    }
}
