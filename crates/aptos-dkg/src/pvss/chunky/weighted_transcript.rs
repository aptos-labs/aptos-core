// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Weighted chunky PVSS transcript (v1): SCRAPE LDT + PoK + consistency check combined into two MSMs, then checked via a single pairing check.

use crate::{
    delegate_transcript_core_to_subtrs,
    pcs::univariate_hiding_kzg,
    pvss::{
        chunky::{
            chunked_elgamal::{self, num_chunks_per_scalar},
            chunks,
            hkzg_chunked_elgamal::{self, HkzgWeightedElgamalWitness},
            input_secret::InputSecret,
            keys,
            public_parameters::PublicParameters,
            subtranscript::Subtranscript,
            verify_common::{verify_weighted_preamble, SokContext},
        },
        traits::{
            self,
            transcript::{HasAggregatableSubtranscript, MalleableTranscript},
            HasEncryptionPublicParams,
        },
        Player,
    },
    range_proofs::{dekart_univariate_v2, traits::BatchedRangeProof},
    sigma_protocol::{
        self,
        homomorphism::{tuple::TupleCodomainShape, Trait as _, TrivialShape},
        traits::{CurveGroupTrait as _, Trait},
    },
    Scalar,
};
use anyhow::bail;
use aptos_crypto::{
    arkworks::{
        self,
        msm::{self, MsmInput},
        random::{
            sample_field_element, sample_field_elements, unsafe_random_point,
            unsafe_random_point_group, unsafe_random_points, UniformRand,
        },
        scrape::LowDegreeTest,
        serialization::{ark_de, ark_se},
        srs::SrsBasis,
    },
    bls12381::{self},
    utils,
    weighted_config::WeightedConfigArkworks,
    CryptoMaterialError, TSecretSharingConfig as _, ValidCryptoMaterial,
};
use ark_ec::{
    pairing::{Pairing, PairingOutput},
    CurveGroup, VariableBaseMSM,
};
use ark_ff::{AdditiveGroup, Field, Fp, FpConfig};
use ark_poly::EvaluationDomain;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use rand_core::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};

/// Domain-separation tag (DST) used to ensure that all cryptographic hashes and
/// transcript operations within the protocol are uniquely namespaced
pub const DST: &[u8; 39] = b"APTOS_WEIGHTED_CHUNKY_FIELD_PVSS_FS_DST";

/// Weighted chunky PVSS transcript.
#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Transcript<P: Pairing> {
    dealer: Player,
    /// This is the aggregatable subtranscript
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub subtrs: Subtranscript<P>,
    /// Proof (of knowledge) showing that the s_{i,j}'s in C are base-B representations (of the s_i's in V, but this is not part of the proof), and that the r_j's in R are used in C
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub sharing_proof: SharingProof<P>,
}

/// This is the secret sharing config that will be used for weighted `chunky`
#[allow(type_alias_bounds)]
type SecretSharingConfig<P: Pairing> = WeightedConfigArkworks<P::ScalarField>;

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
            // Verify the range proof (still separate)
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

        // PoK MSM terms (G1) and LDT MSM terms (G2) for merging into one pairing check
        let eks_inner: Vec<_> = eks.iter().map(|ek| ek.ek).collect();
        let lagr_g1: &[E::G1Affine] = match &pp.pk_range_proof.ck_S.msm_basis {
            SrsBasis::Lagrange { lagr: lagr_g1 } => lagr_g1,
            SrsBasis::PowersOfTau { .. } => {
                bail!("Expected a Lagrange basis, received powers of tau basis instead")
            },
        };
        let hom = hkzg_chunked_elgamal::WeightedHomomorphism::<E>::new(
            lagr_g1,
            pp.pk_range_proof.ck_S.xi_1,
            &pp.pp_elgamal,
            &eks_inner,
        );
        let pok_statement = TupleCodomainShape(
            sigma_protocol::homomorphism::TrivialShape(
                self.sharing_proof.range_proof_commitment.0.into_affine(),
            ),
            chunked_elgamal::WeightedCodomainShape {
                chunks: self.subtrs.Cs.clone(),
                randomness: self.subtrs.Rs.clone(),
            },
        );
        let num_chunks = num_chunks_per_scalar::<E::ScalarField>(pp.ell) as usize;
        let pok_num_beta_powers =
            1 + sc.get_total_weight() * num_chunks + sc.get_max_weight() * num_chunks;
        let pok_msm_terms = hom
            .msm_terms_for_verify::<_, hkzg_chunked_elgamal::WeightedHomomorphism<'static, E>, _>(
                &pok_statement,
                &self.sharing_proof.SoK,
                &sok_cntxt,
                Some(pok_num_beta_powers),
                rng,
            );

        let ldt = LowDegreeTest::random(
            rng,
            sc.get_threshold_weight(),
            sc.get_total_weight() + 1,
            true,
            &sc.get_threshold_config().domain,
        );
        let mut Vs_flat: Vec<E::G2Affine> = self.subtrs.Vs.iter().flatten().copied().collect();
        Vs_flat.push(self.subtrs.V0);

        let beta = sample_field_element(rng);
        let powers_of_beta = utils::powers(beta, sc.get_total_weight() + 1);

        let Cs_flat: Vec<_> = self.subtrs.Cs.iter().flatten().cloned().collect();
        assert_eq!(
            Cs_flat.len(),
            sc.get_total_weight(),
            "Number of ciphertexts does not equal number of weights"
        );

        let mut weighted_Cs_base = Vec::new();
        let mut weighted_Cs_scalar = Vec::new();
        for i in 0..Cs_flat.len() {
            for j in 0..Cs_flat[i].len() {
                weighted_Cs_base.push(Cs_flat[i][j]);
                weighted_Cs_scalar.push(pp.powers_of_radix[j] * powers_of_beta[i]);
            }
        }

        let gamma = sample_field_element(rng);
        let gamma_sq = gamma * gamma;

        let weighted_Cs_msm =
            MsmInput::new(weighted_Cs_base, weighted_Cs_scalar).expect("weighted_Cs MSM terms");
        let merged_g1 =
            msm::merge_scaled_msm_terms::<E::G1>(&[&pok_msm_terms, &weighted_Cs_msm], &[
                gamma,
                E::ScalarField::ONE,
            ]);
        let combined_G1 = E::G1::msm(merged_g1.bases(), merged_g1.scalars())
            .expect("Failed to compute merged G1 MSM in chunky");

        let ldt_msm_terms = ldt.ldt_msm_input::<E::G2>(&Vs_flat)?;
        let n = sc.get_total_weight();
        let weighted_Vs_msm = MsmInput::new(Vs_flat[..n].to_vec(), powers_of_beta[..n].to_vec())
            .expect("weighted_Vs MSM terms");
        let merged_g2 =
            msm::merge_scaled_msm_terms::<E::G2>(&[&ldt_msm_terms, &weighted_Vs_msm], &[
                gamma_sq,
                E::ScalarField::ONE,
            ]);
        let combined_G2 = E::G2::msm(merged_g2.bases(), merged_g2.scalars())
            .expect("Failed to compute merged G2 MSM in chunky");

        let res = E::multi_pairing(
            [
                combined_G1.into_affine(),
                *pp.get_encryption_public_params().message_base(),
            ],
            [pp.get_commitment_base(), (-combined_G2).into_affine()],
        );
        if PairingOutput::<E>::ZERO != res {
            return Err(anyhow::anyhow!("Expected zero during multi-pairing check"));
        }

        Ok(())
    }
}

/// Proof that chunked ciphertexts and commitments are consistent (SoK + batched range proof).
#[allow(non_snake_case)]
#[derive(CanonicalSerialize, CanonicalDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct SharingProof<P: Pairing> {
    /// SoK: the SK is knowledge of `witnesses` s_{i,j} yielding the commitment and the C and the R, their image is the PK, and the signed message is a certain context `cntxt`
    pub SoK: sigma_protocol::Proof<
        P::ScalarField,
        hkzg_chunked_elgamal::WeightedHomomorphism<'static, P>,
    >, // static because we don't want the lifetime of the Proof to depend on the Homomorphism TODO: try removing it?
    /// A batched range proof showing that all committed values s_{i,j} lie in some range
    pub range_proof: dekart_univariate_v2::Proof<P>, // TODO: make an affine version of this
    /// A KZG-style commitment to the values s_{i,j} going into the range proof
    pub range_proof_commitment:
        <dekart_univariate_v2::Proof<P> as BatchedRangeProof<P>>::Commitment,
}

impl<E: Pairing> ValidCryptoMaterial for Transcript<E> {
    const AIP_80_PREFIX: &'static str = "";

    fn to_bytes(&self) -> Vec<u8> {
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
        "chunky_v1".to_string()
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
        let (Cs, Rs, sharing_proof) =
            Self::encrypt_chunked_shares(&f_evals, eks, pp, sc, sok_cntxt, rng);

        // Add constant term for the `\mathbb{G}_2` commitment (we're doing this **after** the previous step
        // because we're now mutating `f_evals` by enlarging it; this is an unimportant technicality however,
        // it has no impact on computational complexity whatsoever as we could simply modify the `commit_to_scalars()`
        // function to take another input)
        f_evals.push(f[0]); // or *s.get_secret_a()

        // Commit to polynomial evaluations + constant term using batch_mul
        //let G_2 = pp.get_commitment_base();
        let flattened_Vs_proj = arkworks::batch_mul::<E::G2>(&pp.G2_table, &f_evals);

        debug_assert_eq!(flattened_Vs_proj.len(), sc.get_total_weight() + 1);

        let Vs_proj = sc.group_by_player(&flattened_Vs_proj); // last item of flattened_Vs_proj is f(0), won't appear in Vs_proj
        let V0_proj = *flattened_Vs_proj.last().unwrap();

        // Batch normalize G2 elements and re-split into V0 and per-player Vs (same layout as Vs_proj).
        let mut g2_elems = vec![V0_proj];
        for row in &Vs_proj {
            g2_elems.extend(row.iter().copied());
        }
        let g2_affine = E::G2::normalize_batch(&g2_elems);
        let mut g2_iter = g2_affine.into_iter();
        let V0 = g2_iter.next().unwrap();
        let Vs: Vec<Vec<E::G2Affine>> = Vs_proj
            .iter()
            .map(|row| row.iter().map(|_| g2_iter.next().unwrap()).collect())
            .collect();
        debug_assert!(g2_iter.next().is_none());

        Transcript {
            dealer: *dealer,
            subtrs: Subtranscript { V0, Vs, Cs, Rs },
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
        let Vs_flat = unsafe_random_points::<E::G2, _>(sc.get_total_weight(), rng);
        let Vs = sc.group_by_player(&Vs_flat);
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
                SoK: hkzg_chunked_elgamal::WeightedProof::generate(sc, num_chunks_per_share, rng),
                range_proof: dekart_univariate_v2::Proof::generate(pp.ell, rng),
            },
        }
    }
}

impl<const N: usize, P: FpConfig<N>, E: Pairing<ScalarField = Fp<P, N>>> Transcript<E> {
    /// Encrypts chunked shares and builds the sharing proof (SoK + range proof).
    /// Panics if `pp.pk_range_proof.ck_S` is not a Lagrange SRS basis (same requirement as verify).
    #[allow(non_snake_case)]
    pub fn encrypt_chunked_shares<
        'a,
        A: Serialize + Clone,
        R: rand_core::RngCore + rand_core::CryptoRng,
    >(
        f_evals: &[E::ScalarField],
        eks: &[keys::EncryptPubKey<E>],
        pp: &PublicParameters<E>,
        sc: &<Self as traits::TranscriptCore>::SecretSharingConfig, // only for debugging purposes?
        sok_cntxt: SokContext<'a, A>,
        rng: &mut R,
    ) -> (
        Vec<Vec<Vec<E::G1Affine>>>,
        Vec<Vec<E::G1Affine>>,
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
        let hom = hkzg_chunked_elgamal::WeightedHomomorphism::<E>::new(
            lagr_g1,
            pp.pk_range_proof.ck_S.xi_1,
            &pp.pp_elgamal,
            &eks_inner,
        );
        //   (2b) Compute its image (the public statement), so the range proof commitment and chunked_elgamal encryptions
        let statement = hom.apply(&witness);
        //   (2c) Produce the SoK
        let (SoK, normalized_statement) = hom.prove(&witness, statement, &sok_cntxt, rng);
        let SoK = SoK.change_lifetime(); // Make sure the lifetime of the proof is not coupled to `hom` which has references

        // Destructure the "public statement" of the above sigma protocol
        let TupleCodomainShape(
            range_proof_commitment,
            chunked_elgamal::WeightedCodomainShape {
                chunks: Cs,
                randomness: Rs,
            },
        ) = normalized_statement;

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

        (Cs, Rs, sharing_proof)
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
