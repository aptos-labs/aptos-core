// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Weighted Chunky PVSS transcript:
//! - SCRAPE LDT
//! - SoK (knowledge-of-exponent)
//! - Share consistency check
//! Batched into:
//!   * one MSM in G1
//!   * one MSM in G2
//! Verified with a single pairing equation.
//!
//! Largely following: <https://alinush.github.io/chunky#chunky-a-weighted-non-malleable-pvss>

use crate::{
    delegate_transcript_core_to_subtrs,
    pcs::univariate_hiding_kzg,
    pvss::{
        chunky::{
            chunked_elgamal::{self, num_chunks_per_scalar},
            hkzg_chunked_elgamal,
            hkzg_chunked_elgamal::WitnessData,
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
        homomorphism::{tuple::TupleCodomainShape, Trait as _},
        traits::{CurveGroupTrait as _, Trait},
    },
};
use anyhow::bail;
use aptos_crypto::{
    arkworks::{
        self,
        msm::{self, MsmInput},
        random::{sample_field_element_with_powers, unsafe_random_point},
        scrape::LowDegreeTest,
        serialization::{ark_de, ark_se},
        srs::SrsBasis,
    },
    bls12381::{self},
    weighted_config::WeightedConfigArkworks as SecretSharingConfig,
    CryptoMaterialError, TSecretSharingConfig as _, ValidCryptoMaterial,
};
use ark_ec::{
    pairing::{Pairing, PairingOutput},
    CurveGroup, VariableBaseMSM,
};
use ark_ff::{AdditiveGroup, Field, Fp, FpConfig};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use rand_core::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};

/// Domain-separation tag (DST) used to ensure that all cryptographic hashes and
/// transcript operations within the protocol are uniquely namespaced
pub const DST: &[u8; 39] = b"APTOS_WEIGHTED_CHUNKY_FIELD_PVSS_FS_DST";

/// Weighted chunky PVSS transcript.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Transcript<P: Pairing> {
    dealer: Player,
    /// This is the aggregatable subtranscript
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    // Even though `Subtranscript` implements serde, we need this attribute macro because `Pairing` does not implement serde
    pub subtrs: Subtranscript<P>,
    /// Proof (of knowledge) showing that the s_{i,j}'s in C are base-B representations (of the s_i's in V, but this is not part of the proof), and that the r_j's in R are used in C
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub sharing_proof: SharingProof<P>,
}

/// Proof that chunked ciphertexts and commitments are consistent (SoK + batched range proof).
#[allow(non_snake_case)]
#[derive(CanonicalSerialize, CanonicalDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct SharingProof<E: Pairing> {
    /// SoK: the SK is knowledge of `witnesses` s_{i,j} yielding the commitment and the C and the R, their image is the PK, and the signed message is a certain context `cntxt`
    pub SoK: sigma_protocol::Proof<E::ScalarField, hkzg_chunked_elgamal::Homomorphism<'static, E>>, // static because we don't want the lifetime of the Proof to depend on the Homomorphism
    /// A batched range proof showing that all committed values s_{i,j} lie in some range
    pub range_proof: dekart_univariate_v2::Proof<E>,
    /// A KZG-style commitment to the values s_{i,j} going into the range proof
    pub range_proof_commitment:
        <dekart_univariate_v2::Proof<E> as BatchedRangeProof<E>>::CommitmentNormalised,
}

impl<E: Pairing> ValidCryptoMaterial for Transcript<E> {
    const AIP_80_PREFIX: &'static str = "";

    fn to_bytes(&self) -> Vec<u8> {
        bcs::to_bytes(&self)
            .expect("Unexpected error during chunky field PVSS transcript serialization")
    }
}

impl<E: Pairing> TryFrom<&[u8]> for Transcript<E> {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        bcs::from_bytes::<Transcript<E>>(bytes)
            .map_err(|_| CryptoMaterialError::DeserializationError)
    }
}

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
            "Number of encryption keys must equal total number of players"
        );

        // Initialize the PVSS SoK context
        let sok_cntxt = SokContext::new(spk.clone(), session_id, dealer.id, Self::dst());

        // Step 1: sample the polynomial and compute the Shamir shares
        let (f, mut f_evals) = sc
            .get_threshold_config()
            .sample_polynomial_and_compute_shares(*s.get_secret_a(), rng);

        // Step 3-6: Encrypt the chunked shares and generate the sharing proof
        let (Cs, Rs, sharing_proof) =
            Self::encrypt_chunked_shares(&f_evals, eks, pp, sc, sok_cntxt, rng);

        // Step 2 (which comes after 3-6 here for efficiency, because we modify `f_evals`):
        // Commit to polynomial evaluations + constant term using `batch_mul`
        f_evals.push(f[0]);
        let flattened_Vs_proj = arkworks::batch_mul::<E::G2>(&pp.G2_table, &f_evals);

        debug_assert_eq!(flattened_Vs_proj.len(), sc.get_total_weight() + 1);

        // Remainder of this function is just batch-normalising the G2 elements and re-splitting into V0 and per-player Vs.
        let g2_affine = E::G2::normalize_batch(&flattened_Vs_proj);
        let Vs = sc.group_by_player(&g2_affine); // Doesn't use the last value
        let V0 = *g2_affine.last().unwrap();

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
        let num_chunks_per_share = num_chunks_per_scalar::<E::ScalarField>(pp.ell);

        Transcript {
            dealer: sc.get_player(0),
            subtrs: Subtranscript::generate(sc, num_chunks_per_share, rng),
            sharing_proof: SharingProof {
                range_proof_commitment: univariate_hiding_kzg::CommitmentNormalised(
                    unsafe_random_point(rng),
                ),
                SoK: hkzg_chunked_elgamal::Proof::generate(sc, num_chunks_per_share, rng),
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
        sc: &<Self as traits::TranscriptCore>::SecretSharingConfig,
        sok_cntxt: SokContext<'a, A>,
        rng: &mut R,
    ) -> (
        Vec<Vec<Vec<E::G1Affine>>>,
        Vec<Vec<E::G1Affine>>,
        SharingProof<E>,
    ) {
        // Step 3-4a: prepare the SoK witness data
        let WitnessData {
            witness,
            f_evals_chunked_flat,
        } = hkzg_chunked_elgamal::prepare_chunked_witness(f_evals, pp, sc, rng);

        // Step 4b and 5a: compute the encryptions and the KZG commitment
        let ek_g1_affines: Vec<E::G1Affine> = eks.iter().map(|ek| ek.ek).collect();
        let lagr_g1: &[E::G1Affine] = match &pp.pk_range_proof.ck_S.msm_basis {
            SrsBasis::Lagrange { lagr } => lagr,
            SrsBasis::PowersOfTau { .. } => {
                panic!("Expected a Lagrange basis, received powers of tau basis instead")
            },
        };
        let hom = hkzg_chunked_elgamal::Homomorphism::<E>::new(
            lagr_g1,
            pp.pk_range_proof.ck_S.xi_1,
            &pp.pp_elgamal,
            &ek_g1_affines,
        );
        let statement = hom.apply(&witness).expect("hom.apply");

        // Step 6: produce the SoK; this is done before step 5b because it naturally normalises the statement
        let (SoK, normalized_statement) = hom
            .prove(&witness, statement, &sok_cntxt, rng)
            .expect("hom.prove");
        let SoK = SoK.change_lifetime(); // Make sure the lifetime of the proof is not coupled to `hom` which has references

        // Step 5b: compute the range proof
        // Destructure the "public statement" of the above sigma protocol
        let TupleCodomainShape(
            range_proof_commitment,
            chunked_elgamal::CodomainShape {
                chunks: Cs,
                randomness: Rs,
            },
        ) = normalized_statement;
        let range_proof_projective = dekart_univariate_v2::Proof::prove(
            &pp.pk_range_proof,
            &f_evals_chunked_flat,
            pp.ell,
            &univariate_hiding_kzg::CommitmentNormalised(range_proof_commitment.0.clone()),
            &witness.hkzg_randomness,
            rng,
        );

        // Assemble the sharing proof
        let sharing_proof = SharingProof {
            SoK,
            range_proof: range_proof_projective.into(), // Doing G1 normalisation here
            range_proof_commitment: univariate_hiding_kzg::CommitmentNormalised(
                range_proof_commitment.0.clone(),
            ),
        };

        (Cs, Rs, sharing_proof)
    }
}

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
            pp,
            &self.subtrs,
            &self.dealer,
            spks,
            eks,
            sid,
            <Self as traits::Transcript>::dst(),
        )?;
        let Vs_flat = self.subtrs.all_Vs_flat(); // Also has the public key V[0]

        // Step 1: Do the SCRAPE LDT (G_2)
        let ldt = LowDegreeTest::random(
            rng,
            sc.get_threshold_weight(),
            sc.get_total_weight() + 1,
            true, // Because we're using the public key V[0]
            &sc.get_threshold_config().domain,
        );
        let ldt_msm_terms = ldt.ldt_msm_input(&Vs_flat)?;

        // Step 2: Verify the range proof
        let (g1_terms, g2_terms) = self.sharing_proof.range_proof.pairing_for_verify(
            &pp.pk_range_proof.vk,
            sc.get_total_weight() * num_chunks_per_scalar::<E::ScalarField>(pp.ell),
            pp.ell,
            &self.sharing_proof.range_proof_commitment,
            rng,
        )?;

        // Step 3: Check that ciphertexts encrypt the committed shares
        let n = sc.get_total_weight();
        let (_, powers_of_beta) = sample_field_element_with_powers(n, rng);

        let Cs_flat: Vec<_> = self.subtrs.Cs.iter().flatten().cloned().collect();

        let (weighted_Cs_base, weighted_Cs_scalar): (Vec<_>, Vec<_>) = Cs_flat
            .iter()
            .enumerate()
            .flat_map(|(i, row)| {
                let beta_power = &powers_of_beta[i];
                let radix = &pp.powers_of_radix;
                row.iter()
                    .enumerate()
                    .map(move |(j, base)| (*base, radix[j] * beta_power))
            })
            .unzip();

        let weighted_Cs_msm = MsmInput::new(weighted_Cs_base, weighted_Cs_scalar)?;
        let weighted_Vs_msm = MsmInput::new(Vs_flat[..n].to_vec(), powers_of_beta.to_vec())?;
        // An alternative way to get the same MSMs would be:
        // Consider Cs_flat as a list of MSM bases, with each MSM paired with the scalars pp.powers_of_radix
        // These are merged using powers_of_beta
        // On the other hand, each Vs_flat is a list of "single" MSM bases, and these are mixed with the same powers_of_beta

        // Step 4: Verify the SoK (G_1)
        let ek_g1_affines: Vec<E::G1Affine> = eks.iter().map(|ek| ek.ek).collect();
        let lagr_g1: &[E::G1Affine] = match &pp.pk_range_proof.ck_S.msm_basis {
            SrsBasis::Lagrange { lagr: lagr_g1 } => lagr_g1,
            SrsBasis::PowersOfTau { .. } => {
                bail!("Expected a Lagrange basis, received powers of tau basis instead")
            },
        };
        let hom = hkzg_chunked_elgamal::Homomorphism::<E>::new(
            lagr_g1,
            pp.pk_range_proof.ck_S.xi_1,
            &pp.pp_elgamal,
            &ek_g1_affines,
        );
        let sok_statement = TupleCodomainShape(
            sigma_protocol::homomorphism::TrivialShape(
                self.sharing_proof.range_proof_commitment.0.clone(),
            ),
            chunked_elgamal::CodomainShape {
                chunks: self.subtrs.Cs.clone(),
                randomness: self.subtrs.Rs.clone(),
            },
        );
        let sok_msm_terms =
            hom.msm_terms_for_verify(&sok_statement, &self.sharing_proof.SoK, &sok_cntxt)?;

        // Final step: Combine the MSM terms and check the multi-pairing
        let (_, powers_of_gamma) = sample_field_element_with_powers::<E::ScalarField, _>(3, rng);

        let pok_merged = msm::merge_msm_inputs::<E::G1Affine, _>(&sok_msm_terms, rng)?;
        let g1_inputs = vec![pok_merged, weighted_Cs_msm];
        let merged_g1 = msm::merge_msm_inputs_with_scales::<E::G1Affine>(&g1_inputs, &[
            powers_of_gamma[1],
            E::ScalarField::ONE,
        ])?;
        let combined_G1 =
            E::G1::msm(merged_g1.bases(), merged_g1.scalars()).map_err(|min_len| {
                anyhow::anyhow!(
                    "Failed to compute merged G1 MSM in chunky (bases/scalars min length: {})",
                    min_len
                )
            })?;

        let g2_inputs = vec![ldt_msm_terms, weighted_Vs_msm];
        let merged_g2 = msm::merge_msm_inputs_with_scales(&g2_inputs, &[
            powers_of_gamma[2],
            E::ScalarField::ONE,
        ])?;
        let combined_G2 =
            E::G2::msm(merged_g2.bases(), merged_g2.scalars()).map_err(|min_len| {
                anyhow::anyhow!(
                    "Failed to compute merged G2 MSM in chunky (bases/scalars min length: {})",
                    min_len
                )
            })?;

        let res = E::multi_pairing(
            g1_terms.iter().copied().chain([
                combined_G1.into_affine(),
                *pp.get_encryption_public_params().message_base(),
            ]),
            g2_terms
                .iter()
                .copied()
                .chain([pp.get_commitment_base(), (-combined_G2).into_affine()]),
        );
        if PairingOutput::<E>::ZERO != res {
            bail!("Expected zero during multi-pairing check");
        }

        Ok(())
    }
}

delegate_transcript_core_to_subtrs!(Transcript<E>, subtrs);

impl<const N: usize, P: FpConfig<N>, E: Pairing<ScalarField = Fp<P, N>>> MalleableTranscript
    for Transcript<E>
{
    fn maul_signature<A: Serialize + Clone>(
        &mut self,
        _ssk: &Self::SigningSecretKey,
        _aux: &A,
        _player: &Player,
    ) {
        // TODO: We're not using this; it would probably mean recomputing almost the entire transcript... but then that would require eks and pp
        panic!("Doesn't work for this PVSS, at least for now");
    }
}
