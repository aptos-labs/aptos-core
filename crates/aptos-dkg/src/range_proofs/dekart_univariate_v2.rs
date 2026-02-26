// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// This file implements the range proof described here: https://alinush.github.io/dekart

use ark_ec::pairing::PairingOutput;
use crate::{
    algebra::polynomials,
    pcs::univariate_hiding_kzg,
    range_proofs::traits,
    sigma_protocol::{
        self,
        homomorphism::{self, Trait as _},
        traits::Trait as _,
        CurveGroupTrait,
    },
    utils, Scalar,
};
use aptos_crypto::arkworks::{
    self,
    msm::MsmInput,
    random::{
        sample_field_element, sample_field_elements, unsafe_random_point,
        unsafe_random_points_group,
    },
    srs::{SrsBasis, SrsType},
    GroupGenerators,
};
use ark_ec::{pairing::Pairing, CurveGroup, PrimeGroup, VariableBaseMSM};
use ark_ff::{AdditiveGroup, Field, PrimeField, Zero};
use ark_poly::{self, EvaluationDomain, Polynomial};
use ark_serialize::{
    CanonicalDeserialize, CanonicalSerialize, Compress, Read, SerializationError, Valid, Validate,
};
use num_integer::Roots;
use rand::{CryptoRng, RngCore};
#[cfg(feature = "range_proof_timing_univariate_v2")]
use std::time::{Duration, Instant};
use std::{fmt::Debug, io::Write};

// TODO: make an affine version of this
#[allow(non_snake_case)]
#[derive(CanonicalSerialize, Debug, PartialEq, Eq, Clone, CanonicalDeserialize)]
pub struct Proof<E: Pairing> {
    hatC: E::G1,
    pi_PoK: sigma_protocol::Proof<E::ScalarField, two_term_msm::Homomorphism<E::G1>>,
    Cs: Vec<E::G1>, // has length ell
    D: E::G1,
    a: E::ScalarField,
    a_h: E::ScalarField,
    a_js: Vec<E::ScalarField>, // has length ell
    pi_gamma: univariate_hiding_kzg::OpeningProof<E>,
}

impl<E: Pairing> Proof<E> {
    /// Generates a random looking proof (but not a valid one).
    /// Useful for testing and benchmarking. TODO: might be able to derive this through macros etc
    pub fn generate<R: rand::Rng + rand::CryptoRng>(ell: u8, rng: &mut R) -> Self {
        Self {
            hatC: unsafe_random_point::<E::G1, _>(rng).into(),
            pi_PoK: two_term_msm::Proof::generate(rng),
            Cs: unsafe_random_points_group(ell as usize, rng),
            D: unsafe_random_point::<E::G1, _>(rng).into(),
            a: sample_field_element(rng),
            a_h: sample_field_element(rng),
            a_js: sample_field_elements(ell as usize, rng),
            pi_gamma: univariate_hiding_kzg::OpeningProof::generate(rng),
        }
    }
}

#[allow(non_snake_case)]
#[derive(CanonicalSerialize, CanonicalDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct ProverKey<E: Pairing> {
    pub(crate) vk: VerificationKey<E>,
    pub(crate) ck_S: univariate_hiding_kzg::CommitmentKey<E>,
    pub(crate) max_n: usize,
    pub(crate) prover_precomputed: ProverPrecomputed<E>,
}

#[derive(CanonicalSerialize)]
pub struct PublicStatement<E: Pairing> {
    n: usize,
    ell: u8,
    comm: univariate_hiding_kzg::Commitment<E>,
}

#[derive(CanonicalSerialize, CanonicalDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct VerificationKey<E: Pairing> {
    xi_1: E::G1Affine,
    lagr_0: E::G1Affine,
    vk_hkzg: univariate_hiding_kzg::VerificationKey<E>,
    verifier_precomputed: VerifierPrecomputed<E>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProverPrecomputed<E: Pairing> {
    pub(crate) powers_of_two: Vec<E::ScalarField>,
    h_denom_eval: Vec<E::ScalarField>,
}

// Custom `CanonicalSerialize/CanonicalDeserialize` for `VerifierPrecomputed` because most of it can be recomputed
impl<E: Pairing> CanonicalSerialize for ProverPrecomputed<E> {
    fn serialize_with_mode<W: Write>(
        &self,
        mut writer: W,
        compress: Compress,
    ) -> Result<(), SerializationError> {
        self.powers_of_two
            .len()
            .serialize_with_mode(&mut writer, compress)?;
        let triangular_number = self.h_denom_eval[0]
            .inverse()
            .expect("Could not invert h_denom_eval[0]");
        let num_omegas = floored_triangular_root(
            arkworks::scalar_to_u32(&triangular_number)
                .expect("triangular number did not fit in u32") as usize,
        ) + 1;
        num_omegas.serialize_with_mode(&mut writer, compress)?;

        Ok(())
    }

    fn serialized_size(&self, compress: Compress) -> usize {
        let mut size = 0;
        size += 2 * self.powers_of_two.len().serialized_size(compress); // `num_omegas` is also a usize
        size
    }
}

impl<E: Pairing> CanonicalDeserialize for ProverPrecomputed<E> {
    fn deserialize_with_mode<R: Read>(
        mut reader: R,
        compress: Compress,
        validate: Validate,
    ) -> Result<Self, SerializationError> {
        let powers_len = usize::deserialize_with_mode(&mut reader, compress, validate)?;
        let num_omegas = usize::deserialize_with_mode(&mut reader, compress, validate)?;

        let powers_of_two = arkworks::powers_of_two::<E::ScalarField>(powers_len);

        let roots_of_unity = arkworks::compute_roots_of_unity::<E::ScalarField>(num_omegas);
        let h_denom_eval = compute_h_denom_eval::<E>(&roots_of_unity);

        Ok(Self {
            powers_of_two,
            h_denom_eval,
        })
    }
}

// Required by `CanonicalDeserialize`
impl<E: Pairing> Valid for ProverPrecomputed<E> {
    #[inline]
    fn check(&self) -> Result<(), SerializationError> {
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifierPrecomputed<E: Pairing> {
    powers_of_two: Vec<E::ScalarField>,
    roots_of_unity: Vec<E::ScalarField>,
}

// Custom `CanonicalSerialize/CanonicalDeserialize` for `VerifierPrecomputed` because most of it can be recomputed
impl<E: Pairing> CanonicalSerialize for VerifierPrecomputed<E> {
    fn serialize_with_mode<W: Write>(
        &self,
        mut writer: W,
        compress: Compress,
    ) -> Result<(), SerializationError> {
        self.roots_of_unity
            .len()
            .serialize_with_mode(&mut writer, compress)?;
        self.powers_of_two
            .len()
            .serialize_with_mode(&mut writer, compress)?;

        Ok(())
    }

    fn serialized_size(&self, compress: Compress) -> usize {
        let mut size = 0;
        size += self.roots_of_unity.len().serialized_size(compress);
        size += self.roots_of_unity[1].serialized_size(compress);
        size += self.powers_of_two.len().serialized_size(compress);

        size
    }
}

impl<E: Pairing> CanonicalDeserialize for VerifierPrecomputed<E> {
    fn deserialize_with_mode<R: Read>(
        mut reader: R,
        compress: Compress,
        validate: Validate,
    ) -> Result<Self, SerializationError> {
        let num_omegas = usize::deserialize_with_mode(&mut reader, compress, validate)?;
        let max_ell = usize::deserialize_with_mode(&mut reader, compress, validate)?;

        let roots_of_unity = arkworks::compute_roots_of_unity::<E::ScalarField>(num_omegas);
        let powers_of_two = arkworks::powers_of_two::<E::ScalarField>(max_ell);

        // Reconstruct the VerificationKey
        Ok(Self {
            roots_of_unity,
            powers_of_two,
        })
    }
}

// Required by CanonicalDeserialize
impl<E: Pairing> Valid for VerifierPrecomputed<E> {
    #[inline]
    fn check(&self) -> Result<(), SerializationError> {
        Ok(())
    }
}

fn compute_h_denom_eval<E: Pairing>(
    roots_of_unity_in_eval_dom: &Vec<E::ScalarField>,
) -> Vec<E::ScalarField> {
    let num_omegas = roots_of_unity_in_eval_dom.len();
    let mut h_denom_eval = Vec::with_capacity(num_omegas);

    // First element: inverse of (max_n * (max_n + 1) / 2)
    h_denom_eval.push(
        E::ScalarField::from(((num_omegas - 1) * num_omegas / 2) as u64)
            .inverse()
            .expect("Value should be invertible"),
    );

    // Remaining elements: h_denom_eval[i] is the inverse of (max_n + 1) / (ω^i (ω^i - 1) )
    h_denom_eval.extend(roots_of_unity_in_eval_dom.iter().skip(1).map(|&root| {
        (root * (root - E::ScalarField::ONE)) / E::ScalarField::from(num_omegas as u64)
    }));

    h_denom_eval
}

impl<E: Pairing> traits::BatchedRangeProof<E> for Proof<E> {
    type Commitment = univariate_hiding_kzg::Commitment<E>;
    type CommitmentKey = univariate_hiding_kzg::CommitmentKey<E>;
    type CommitmentRandomness = univariate_hiding_kzg::CommitmentRandomness<E::ScalarField>;
    type Input = E::ScalarField;
    type ProverKey = ProverKey<E>;
    type PublicStatement = PublicStatement<E>;
    type VerificationKey = VerificationKey<E>;

    /// Domain-separation tag (DST) used to ensure that all cryptographic hashes and
    /// transcript operations within the protocol are uniquely namespaced
    const DST: &[u8] = b"APTOS_UNIVARIATE_DEKART_V2_RANGE_PROOF_DST";

    fn commitment_key_from_prover_key(pk: &Self::ProverKey) -> Self::CommitmentKey {
        pk.ck_S.clone()
    }

    #[allow(non_snake_case)]
    fn setup<R: RngCore + CryptoRng>(
        max_n: usize,
        max_ell: u8,
        group_generators: GroupGenerators<E>,
        rng: &mut R,
    ) -> (ProverKey<E>, VerificationKey<E>) {
        #[cfg(feature = "range_proof_timing_univariate_v2")]
        let setup_start = Instant::now();
        #[cfg(feature = "range_proof_timing_univariate_v2")]
        let mut cumulative = Duration::ZERO;
        #[cfg(feature = "range_proof_timing_univariate_v2")]
        let mut print_cumulative = |name: &str, duration: Duration| {
            cumulative += duration;
            println!(
                "  {:>10.2} ms  ({:>10.2} ms cum.)  [dekart_univariate_v2 setup] {}",
                duration.as_secs_f64() * 1000.0,
                cumulative.as_secs_f64() * 1000.0,
                name
            );
        };

        let num_omegas = max_n + 1;
        assert!(num_omegas.is_power_of_two());

        #[cfg(feature = "range_proof_timing_univariate_v2")]
        let start = Instant::now();
        // Generate trapdoor elements
        let trapdoor = univariate_hiding_kzg::Trapdoor::<E>::rand(rng);
        let xi_1_proj: E::G1 = group_generators.g1 * trapdoor.xi;
        #[cfg(feature = "range_proof_timing_univariate_v2")]
        print_cumulative("trapdoor + xi_1_proj", start.elapsed());

        #[cfg(feature = "range_proof_timing_univariate_v2")]
        let start = Instant::now();
        let (vk_hkzg, ck_S) = univariate_hiding_kzg::setup(
            max_n + 1,
            SrsType::Lagrange,
            group_generators.clone(),
            trapdoor,
        );
        #[cfg(feature = "range_proof_timing_univariate_v2")]
        print_cumulative("univariate_hiding_kzg::setup", start.elapsed());

        #[cfg(feature = "range_proof_timing_univariate_v2")]
        let start = Instant::now();
        let h_denom_eval = compute_h_denom_eval::<E>(&ck_S.roots_of_unity_in_eval_dom);
        #[cfg(feature = "range_proof_timing_univariate_v2")]
        print_cumulative("compute_h_denom_eval", start.elapsed());

        #[cfg(feature = "range_proof_timing_univariate_v2")]
        let start = Instant::now();
        let powers_of_two = arkworks::powers_of_two::<E::ScalarField>(max_ell.into());

        let prover_precomputed = ProverPrecomputed {
            powers_of_two: powers_of_two.clone(),
            h_denom_eval,
        };

        let verifier_precomputed = VerifierPrecomputed {
            powers_of_two,
            roots_of_unity: ck_S.roots_of_unity_in_eval_dom.clone(),
        };

        let lagr_0: E::G1Affine = match &ck_S.msm_basis {
            SrsBasis::Lagrange { lagr: lagr_g1 } => lagr_g1[0],
            SrsBasis::PowersOfTau { .. } => panic!("Wrong basis, this should not happen"),
        };

        let vk = VerificationKey {
            xi_1: xi_1_proj.into_affine(),
            lagr_0,
            vk_hkzg,
            verifier_precomputed,
        };
        let prk = ProverKey {
            vk: vk.clone(),
            ck_S,
            max_n,
            prover_precomputed,
        };
        #[cfg(feature = "range_proof_timing_univariate_v2")]
        print_cumulative("powers_of_two + precomputed + vk/prk", start.elapsed());

        #[cfg(feature = "range_proof_timing_univariate_v2")]
        println!(
            "  [dekart_univariate_v2 setup] TOTAL: {:.2} ms",
            setup_start.elapsed().as_secs_f64() * 1000.0
        );

        (prk, vk)
    }

    #[allow(non_snake_case)]
    fn commit_with_randomness(
        ck_S: &Self::CommitmentKey,
        values: &[Self::Input],
        rho: &Self::CommitmentRandomness,
    ) -> Self::Commitment {
        let mut values_shifted = vec![E::ScalarField::ZERO]; // start with 0,
        values_shifted.extend(values); // then append all values from the original vector

        univariate_hiding_kzg::commit_with_randomness(ck_S, &values_shifted, rho)
    }

    #[allow(non_snake_case)]
    fn prove<R>(
        pk: &ProverKey<E>,
        values: &[Self::Input],
        ell: u8,
        comm: &Self::Commitment,
        rho: &Self::CommitmentRandomness,
        rng: &mut R,
    ) -> Proof<E>
    where
        R: rand_core::RngCore + rand_core::CryptoRng,
    {
        #[cfg(feature = "range_proof_timing_univariate_v2")]
        let prove_start = Instant::now();
        #[cfg(feature = "range_proof_timing_univariate_v2")]
        let mut cumulative = Duration::ZERO;
        #[cfg(feature = "range_proof_timing_univariate_v2")]
        let mut print_cumulative = |name: &str, duration: Duration| {
            cumulative += duration;
            println!(
                "  {:>10.2} ms  ({:>10.2} ms cum.)  [dekart_univariate_v2 prove] {}",
                duration.as_secs_f64() * 1000.0,
                cumulative.as_secs_f64() * 1000.0,
                name
            );
        };

        let mut fs_t = merlin::Transcript::new(Self::DST);

        #[cfg(feature = "range_proof_timing_univariate_v2")]
        let start = Instant::now();
        // Step 1a
        let ProverKey {
            vk,
            ck_S,
            max_n,
            prover_precomputed,
        } = pk;

        let n = values.len();
        let max_ell: u8 = prover_precomputed.powers_of_two.len().try_into().unwrap();

        assert!(
            n <= *max_n,
            "n (got {}) must be ≤ max_n (which is {})",
            n,
            max_n
        );
        // TODO: Use a subdomain to make the FFTs smaller, when n is much smaller than max_n
        assert!(
            ell <= max_ell,
            "ell (got {}) must be ≤ max_ell (which is {})",
            ell,
            max_ell
        );

        let num_omegas = max_n + 1;

        let univariate_hiding_kzg::CommitmentKey {
            xi_1,
            msm_basis,
            eval_dom,
            m_inv: num_omegas_inv,
            ..
        } = ck_S;

        let lagr_g1: &[E::G1Affine] = match msm_basis {
            SrsBasis::Lagrange { lagr: lagr_g1 } => lagr_g1,
            SrsBasis::PowersOfTau { .. } => {
                panic!("Expected Lagrange basis, somehow got PowersOfTau basis instead")
            },
        };

        debug_assert_eq!(
            *num_omegas_inv,
            E::ScalarField::from(num_omegas as u64).inverse().unwrap()
        );

        // Step 1b
        fiat_shamir::append_initial_data(&mut fs_t, Self::DST, vk, PublicStatement {
            n,
            ell,
            comm: comm.clone(),
        });
        #[cfg(feature = "range_proof_timing_univariate_v2")]
        print_cumulative("unpack pk + append_initial_data", start.elapsed());

        #[cfg(feature = "range_proof_timing_univariate_v2")]
        let start = Instant::now();
        // Step 2a
        let r = sample_field_element(rng);
        let delta_rho = sample_field_element(rng);
        let hatC = *xi_1 * delta_rho + lagr_g1[0] * r + comm.0;

        // Step 2b
        fiat_shamir::append_hat_f_commitment::<E>(&mut fs_t, &hatC);
        #[cfg(feature = "range_proof_timing_univariate_v2")]
        print_cumulative("hatC (r, delta_rho, hatC) + append_hat_f", start.elapsed());

        #[cfg(feature = "range_proof_timing_univariate_v2")]
        let start = Instant::now();
        // Step 3a
        let pi_PoK = two_term_msm::Homomorphism {
            base_1: lagr_g1[0],
            base_2: *xi_1,
        }
        .prove(
            &two_term_msm::Witness {
                poly_randomness: Scalar(r),
                hiding_kzg_randomness: Scalar(delta_rho),
            },
            two_term_msm::CodomainShape(hatC - comm.0),
            &Self::DST,
            rng,
        )
        .0; // TODO: we're throwing away the normalised statment here, fix it

        // Step 3b
        fiat_shamir::append_sigma_proof::<E>(&mut fs_t, &pi_PoK);
        #[cfg(feature = "range_proof_timing_univariate_v2")]
        print_cumulative("pi_PoK (two_term_msm prove) + append_sigma", start.elapsed());

        #[cfg(feature = "range_proof_timing_univariate_v2")]
        let start = Instant::now();
        // Step 4a
        let rs: Vec<E::ScalarField> = (0..ell).map(|_| sample_field_element(rng)).collect();

        let f_js_evals: Vec<Vec<E::ScalarField>> = {
            let mut f_js_evals = vec![Vec::with_capacity(num_omegas); ell as usize];

            for j in 0..ell as usize {
                f_js_evals[j].push(rs[j]);
            }

            for &val in values.iter() {
                let bits = utils::scalar_to_bits_le::<E>(&val);
                for j in 0..ell as usize {
                    f_js_evals[j].push(E::ScalarField::from(bits[j]));
                }
            }

            for f_j in &mut f_js_evals {
                f_j.resize(num_omegas, E::ScalarField::ZERO);
            }

            f_js_evals
        };

        let rhos: Vec<E::ScalarField> = std::iter::repeat_with(|| sample_field_element(rng))
            .take(ell as usize)
            .collect();

        let hkzg_commitment_hom = univariate_hiding_kzg::CommitmentHomomorphism::<E> {
            msm_basis: lagr_g1,
            xi_1: *xi_1,
        };
        // f_j_evals[0] is blinding rs[j]; f_j_evals[1..] are 0/1. Compute commitment as xi_1*rho + lagr_g1[0]*f_j_evals[0] + msm_bool(lagr_g1[1..], bits).
        let Cs: Vec<E::G1> = f_js_evals
            .iter()
            .zip(rhos.iter())
            .map(|(f_j_evals, &rho)| {
                let bits: Vec<bool> = f_j_evals[1..]
                    .iter()
                    .map(|e| !e.is_zero())
                    .collect();
                let sum = lagr_g1[0] * f_j_evals[0]
                    + utils::msm_bool(&lagr_g1[1..], &bits);
                *xi_1 * rho + sum
            })
            .collect();

        // Step 4b
        fiat_shamir::append_f_j_commitments::<E>(&mut fs_t, &Cs);
        #[cfg(feature = "range_proof_timing_univariate_v2")]
        print_cumulative("f_js_evals + rhos + Cs (hkzg commits) + append_f_j", start.elapsed());

        #[cfg(feature = "range_proof_timing_univariate_v2")]
        let start = Instant::now();
        // Step 6
        let (beta, betas) = fiat_shamir::get_beta_challenges::<E>(&mut fs_t, ell as usize);
        #[cfg(feature = "range_proof_timing_univariate_v2")]
        print_cumulative("get_beta_challenges", start.elapsed());

        #[cfg(feature = "range_proof_timing_univariate_v2")]
        let start = Instant::now();
        let hat_f_evals: Vec<E::ScalarField> = {
            let mut v = Vec::with_capacity(num_omegas);
            v.push(r);
            v.extend_from_slice(values);
            v.resize(num_omegas, E::ScalarField::ZERO);
            v
        };
        #[cfg(feature = "range_proof_timing_univariate_v2")]
        print_cumulative("hat_f_evals", start.elapsed());

        #[cfg(feature = "range_proof_timing_univariate_v2")]
        let start = Instant::now();
        let hat_f_coeffs = eval_dom.ifft(&hat_f_evals);
        debug_assert_eq!(hat_f_coeffs.len(), pk.max_n + 1);
        #[cfg(feature = "range_proof_timing_univariate_v2")]
        print_cumulative("hat_f_coeffs (ifft)", start.elapsed());

        #[cfg(feature = "range_proof_timing_univariate_v2")]
        let start = Instant::now();
        let diff_hat_f_evals: Vec<E::ScalarField> = {
            let mut result = polynomials::differentiate(&hat_f_coeffs);
            eval_dom.fft_in_place(&mut result);
            result
        };
        #[cfg(feature = "range_proof_timing_univariate_v2")]
        print_cumulative("diff_hat_f_evals (fft)", start.elapsed());

        #[cfg(feature = "range_proof_timing_univariate_v2")]
        let start = Instant::now();
        let f_j_coeffs: Vec<Vec<E::ScalarField>> = (0..ell as usize)
            .map(|j| {
                let mut f_j = f_js_evals[j].clone();
                debug_assert_eq!(f_j.len(), pk.max_n + 1);
                eval_dom.ifft_in_place(&mut f_j);
                debug_assert_eq!(f_j.len(), pk.max_n + 1);
                f_j
            })
            .collect();
        #[cfg(feature = "range_proof_timing_univariate_v2")]
        print_cumulative("f_j_coeffs (ifft)", start.elapsed());

        #[cfg(feature = "range_proof_timing_univariate_v2")]
        let start = Instant::now();
        let diff_f_js_evals: Vec<Vec<E::ScalarField>> = f_js_evals
            .iter()
            .map(|f_j_eval| {
                let mut result = eval_dom.ifft(f_j_eval); // Convert to coefficients
                polynomials::differentiate_in_place(&mut result); // Differentiate
                eval_dom.fft_in_place(&mut result); // Convert back to evaluations
                result
            })
            .collect();
        #[cfg(feature = "range_proof_timing_univariate_v2")]
        print_cumulative("diff_f_js_evals (ifft)", start.elapsed());

        #[cfg(feature = "range_proof_timing_univariate_v2")]
        let start = Instant::now();
        let h_evals: Vec<E::ScalarField> = {
            let mut result = Vec::with_capacity(num_omegas);

            let first_h_eval = {
                let mut pow2 = E::ScalarField::ONE;
                let mut sum_pow2_rs = E::ScalarField::ZERO;
                for r_j in &rs {
                    sum_pow2_rs += pow2 * r_j;
                    pow2 = pow2.double();
                }

                let sum_betas_term: E::ScalarField = betas
                    .iter()
                    .zip(&rs)
                    .map(|(&beta_j, r_j)| beta_j * r_j * (*r_j - E::ScalarField::ONE))
                    .sum();

                let numerator = beta * (r - sum_pow2_rs) + sum_betas_term;
                numerator * num_omegas_inv
            };
            result.push(first_h_eval);

            for i in 1..num_omegas {
                // First term: beta * diff_hat_f_evals[i]
                let mut val = diff_hat_f_evals[i];

                // Second term: -beta * sum_j 2^j * f_j_evals[j][i]
                let sum1: E::ScalarField = diff_f_js_evals
                    .iter()
                    .enumerate()
                    .map(|(j, diff_f_j)| E::ScalarField::from(1u64 << j) * diff_f_j[i])
                    .sum();
                val = (val - sum1) * beta;

                // Third term: sum_j betas[j] * diff_f_j * (2*f_j - 1)
                let sum2: E::ScalarField = diff_f_js_evals
                    .iter()
                    .zip(f_js_evals.iter())
                    .enumerate()
                    .map(|(j, (diff_f_j, f_j))| {
                        betas[j]
                            * diff_f_j[i]
                            * (E::ScalarField::from(2u64) * f_j[i] - E::ScalarField::ONE)
                    })
                    .sum();
                val += sum2;

                // Divide by precomputed denominator
                val *= prover_precomputed.h_denom_eval[i];

                result.push(val);
            }

            result
        };
        #[cfg(feature = "range_proof_timing_univariate_v2")]
        print_cumulative("h_evals", start.elapsed());

        #[cfg(feature = "range_proof_timing_univariate_v2")]
        let start = Instant::now();
        let rho_h = sample_field_element(rng);
        let D = hkzg_commitment_hom
            .apply(&univariate_hiding_kzg::Witness {
                hiding_randomness: Scalar(rho_h),
                values: Scalar::vec_from_inner_slice(&h_evals),
            })
            .0;
        // Step 7b
        fiat_shamir::append_h_commitment::<E>(&mut fs_t, &D);
        #[cfg(feature = "range_proof_timing_univariate_v2")]
        print_cumulative("D (hkzg commit to h_evals) + append_h", start.elapsed());

        #[cfg(feature = "range_proof_timing_univariate_v2")]
        let start = Instant::now();
        // Step 8
        let (mu, mu_h, mus) = fiat_shamir::get_mu_challenges::<E>(&mut fs_t, ell as usize);

        let u_values: Vec<_> = (0..num_omegas)
            .map(|i| {
                mu * hat_f_evals[i]
                    + mu_h * h_evals[i]
                    + mus
                        .iter()
                        .zip(&f_js_evals)
                        .map(|(&mu_j, f_j)| mu_j * f_j[i])
                        .sum::<E::ScalarField>()
            })
            .collect();
        #[cfg(feature = "range_proof_timing_univariate_v2")]
        print_cumulative("get_mu_challenges + u_values", start.elapsed());

        #[cfg(feature = "range_proof_timing_univariate_v2")]
        let start = Instant::now();
        // Step 9
        let gamma =
            fiat_shamir::get_gamma_challenge::<E>(&mut fs_t, &ck_S.roots_of_unity_in_eval_dom);

        let a: E::ScalarField = {
            let poly = ark_poly::univariate::DensePolynomial {
                coeffs: hat_f_coeffs,
            };
            poly.evaluate(&gamma)
        }; // This algorithm should be Horner's, hence a bit faster than barycentric interpolation

        let a_h = polynomials::barycentric_eval(
            &h_evals,
            &ck_S.roots_of_unity_in_eval_dom,
            gamma,
            *num_omegas_inv,
        );

        let a_js: Vec<E::ScalarField> = (0..ell as usize)
            .map(|i| {
                let poly = ark_poly::univariate::DensePolynomial {
                    coeffs: f_j_coeffs[i].clone(),
                };
                poly.evaluate(&gamma)
            }) // Again, using Horner's here
            .collect();
        #[cfg(feature = "range_proof_timing_univariate_v2")]
        print_cumulative("gamma + a + a_h + a_js (evals at gamma)", start.elapsed());

        #[cfg(feature = "range_proof_timing_univariate_v2")]
        let start = Instant::now();
        // Step 10
        let s = sample_field_element(rng);

        let rho_u = mu * (rho.0 + delta_rho)
            + mu_h * rho_h
            + mus
                .iter()
                .zip(&rhos)
                .map(|(&mu_j, &rho_j)| mu_j * rho_j)
                .sum::<E::ScalarField>();

        let u_val = polynomials::barycentric_eval(
            &u_values,
            &ck_S.roots_of_unity_in_eval_dom,
            gamma,
            *num_omegas_inv,
        );
        let pi_gamma = univariate_hiding_kzg::CommitmentHomomorphism::open(
            ck_S,
            u_values,
            rho_u,
            gamma,
            u_val,
            &Scalar(s),
            0, // the `offset`
        );
        #[cfg(feature = "range_proof_timing_univariate_v2")]
        print_cumulative("rho_u + u_val + pi_gamma (hkzg open)", start.elapsed());

        #[cfg(feature = "range_proof_timing_univariate_v2")]
        println!(
            "  [dekart_univariate_v2 prove] TOTAL: {:.2} ms",
            prove_start.elapsed().as_secs_f64() * 1000.0
        );

        Proof {
            hatC,
            pi_PoK,
            Cs,
            D,
            a,
            a_h,
            a_js,
            pi_gamma,
        }
    }

    #[allow(non_snake_case)]
    fn pairing_for_verify<R: RngCore + CryptoRng>(
        &self,
        vk: &Self::VerificationKey,
        n: usize,
        ell: u8,
        comm: &Self::Commitment,
        rng: &mut R,
    ) -> anyhow::Result<(Vec<E::G1Affine>, Vec<E::G2Affine>)> {
        #[cfg(feature = "range_proof_timing_univariate_v2")]
        let verify_start = Instant::now();
        #[cfg(feature = "range_proof_timing_univariate_v2")]
        let mut cumulative = Duration::ZERO;
        #[cfg(feature = "range_proof_timing_univariate_v2")]
        let mut print_cumulative = |name: &str, duration: Duration| {
            cumulative += duration;
            println!(
                "  {:>10.2} ms  ({:>10.2} ms cum.)  [dekart_univariate_v2 verify] {}",
                duration.as_secs_f64() * 1000.0,
                cumulative.as_secs_f64() * 1000.0,
                name
            );
        };

        let mut fs_t = merlin::Transcript::new(Self::DST);

        #[cfg(feature = "range_proof_timing_univariate_v2")]
        let start = Instant::now();
        // Step 1
        let VerificationKey {
            xi_1,
            lagr_0,
            vk_hkzg,
            verifier_precomputed,
        } = vk;

        assert!(
            ell as usize <= verifier_precomputed.powers_of_two.len(),
            "ell (got {}) must be ≤ max_ell (which is {})",
            ell,
            verifier_precomputed.powers_of_two.len()
        ); // Easy to work around this if it fails...

        let Proof {
            hatC,
            pi_PoK,
            Cs,
            D,
            a,
            a_h,
            a_js,
            pi_gamma,
        } = self;

        // Step 2a
        fiat_shamir::append_initial_data(&mut fs_t, Self::DST, vk, PublicStatement {
            n,
            ell,
            comm: comm.clone(),
        });

        // Step 2b
        fiat_shamir::append_hat_f_commitment::<E>(&mut fs_t, &hatC);
        #[cfg(feature = "range_proof_timing_univariate_v2")]
        print_cumulative("unpack + append_initial_data + append_hat_f", start.elapsed());

        #[cfg(feature = "range_proof_timing_univariate_v2")]
        let start = Instant::now();
        // Step 3
        let hom = two_term_msm::Homomorphism::<E::G1> {
            base_1: *lagr_0,
            base_2: *xi_1,
        };
        <two_term_msm::Homomorphism<E::G1> as CurveGroupTrait>::verify(
            &hom,
            &(two_term_msm::CodomainShape((*hatC - comm.0).into_affine())),
            pi_PoK,
            &Self::DST,
            Some(1), // TrivialShape has one element
            rng,
        )?;
        #[cfg(feature = "range_proof_timing_univariate_v2")]
        print_cumulative("two_term_msm verify", start.elapsed());

        #[cfg(feature = "range_proof_timing_univariate_v2")]
        let start = Instant::now();
        // Step 4a
        fiat_shamir::append_sigma_proof::<E>(&mut fs_t, &pi_PoK);

        // Step 4b
        fiat_shamir::append_f_j_commitments::<E>(&mut fs_t, &Cs);

        // Step 5
        let (beta, beta_js) = fiat_shamir::get_beta_challenges::<E>(&mut fs_t, ell as usize);

        // Step 6
        fiat_shamir::append_h_commitment::<E>(&mut fs_t, &D);

        // Step 7
        let (mu, mu_h, mu_js) = fiat_shamir::get_mu_challenges::<E>(&mut fs_t, ell as usize);
        #[cfg(feature = "range_proof_timing_univariate_v2")]
        print_cumulative("append_sigma + append_f_j + get_beta + append_h + get_mu", start.elapsed());

        #[cfg(feature = "range_proof_timing_univariate_v2")]
        let start = Instant::now();
        // Step 8
        let U_bases: Vec<E::G1Affine> = {
            let mut v = Vec::with_capacity(2 + Cs.len());
            v.push(*hatC);
            v.push(*D);
            v.extend_from_slice(&Cs);
            E::G1::normalize_batch(&v)
        };

        let U_scalars: Vec<E::ScalarField> = {
            let mut v = Vec::with_capacity(2 + mu_js.len());
            v.push(mu);
            v.push(mu_h);
            v.extend_from_slice(&mu_js);
            v
        };

        let U = E::G1::msm(&U_bases, &U_scalars).expect("Failed to compute MSM in DeKARTv2");
        #[cfg(feature = "range_proof_timing_univariate_v2")]
        print_cumulative("U_bases + U_scalars + MSM", start.elapsed());

        #[cfg(feature = "range_proof_timing_univariate_v2")]
        let start = Instant::now();
        // Step 9
        let gamma =
            fiat_shamir::get_gamma_challenge::<E>(&mut fs_t, &verifier_precomputed.roots_of_unity);

        // Step 10
        let a_u = *a * mu
            + *a_h * mu_h
            + a_js
                .iter()
                .zip(&mu_js)
                .map(|(&a_j, &mu_j)| a_j * mu_j)
                .sum::<E::ScalarField>();

        #[cfg(feature = "range_proof_timing_univariate_v2")]
        print_cumulative("gamma + a_u + hkzg verify", start.elapsed());

        #[cfg(feature = "range_proof_timing_univariate_v2")]
        let start = Instant::now();
        // Step 11
        let num_omegas = verifier_precomputed.roots_of_unity.len();

        let LHS = {
            // First compute V_SS^*(gamma), where V_SS^*(X) is the polynomial (X^{max_n + 1} - 1) / (X - 1)
            let V_eval_gamma = {
                let gamma_pow = gamma.pow([num_omegas as u64]);
                (gamma_pow - E::ScalarField::ONE) * (gamma - E::ScalarField::ONE).inverse().unwrap()
            };

            *a_h * V_eval_gamma
        };

        let RHS = {
            // Compute sum_j 2^j a_j
            let sum1: E::ScalarField = verifier_precomputed
                .powers_of_two
                .iter()
                .zip(a_js.iter())
                .map(|(&power_of_two, aj)| power_of_two * aj)
                .sum();

            // Compute sum_j beta_j a_j (a_j - 1)
            let sum2: E::ScalarField = beta_js
                .iter()
                .zip(a_js.iter())
                .map(|(beta, &a)| a * (a - E::ScalarField::ONE) * beta) // TODO: submit PR to change arkworks so beta can be on the left...
                .sum();

            beta * (*a - sum1) + sum2
        };

        anyhow::ensure!(LHS == RHS);
        #[cfg(feature = "range_proof_timing_univariate_v2")]
        print_cumulative("LHS/RHS (V_eval_gamma + sum1 + sum2) + ensure", start.elapsed());

        #[cfg(feature = "range_proof_timing_univariate_v2")]
        println!(
            "  [dekart_univariate_v2 verify] TOTAL: {:.2} ms",
            verify_start.elapsed().as_secs_f64() * 1000.0
        );


        use sigma_protocol::homomorphism::TrivialShape as HkzgCommitment;
        Ok(univariate_hiding_kzg::CommitmentHomomorphism::pairing_for_verify(
            *vk_hkzg,
            HkzgCommitment(U), // TODO: Ugh univariate_hiding_kzg::Commitment(U) does not work because it's a tuple struct, see https://github.com/rust-lang/rust/issues/17422; So make it a struct with one named field?
            gamma,
            a_u,
            pi_gamma.clone(),
        ))
    }

    fn maul(&mut self) {
        self.D += E::G1::generator();
    }
}

mod fiat_shamir {
    use super::*;
    use crate::fiat_shamir::RangeProof;
    use merlin::Transcript;

    pub(crate) fn append_initial_data<E: Pairing>(
        fs_t: &mut Transcript,
        dst: &[u8],
        vk: &VerificationKey<E>,
        ps: PublicStatement<E>,
    ) {
        <Transcript as RangeProof<E, Proof<E>>>::append_sep(fs_t, dst);
        <Transcript as RangeProof<E, Proof<E>>>::append_vk(fs_t, vk);
        <Transcript as RangeProof<E, Proof<E>>>::append_public_statement(fs_t, ps);
    }

    #[allow(non_snake_case)]
    pub(crate) fn append_hat_f_commitment<E: Pairing>(
        fs_transcript: &mut Transcript,
        hatC: &E::G1,
    ) {
        <Transcript as RangeProof<E, Proof<E>>>::append_hat_f_commitment(fs_transcript, hatC);
    }

    #[allow(non_snake_case)]
    pub(crate) fn append_sigma_proof<E: Pairing>(
        fs_transcript: &mut Transcript,
        pi_PoK: &sigma_protocol::Proof<E::ScalarField, two_term_msm::Homomorphism<E::G1>>,
    ) {
        <Transcript as RangeProof<E, Proof<E>>>::append_sigma_proof(fs_transcript, pi_PoK);
    }

    #[allow(non_snake_case)]
    pub(crate) fn append_f_j_commitments<E: Pairing>(
        fs_transcript: &mut Transcript,
        Cs: &Vec<E::G1>,
    ) {
        <Transcript as RangeProof<E, Proof<E>>>::append_f_j_commitments(fs_transcript, Cs);
    }

    pub(crate) fn get_beta_challenges<E: Pairing>(
        fs_transcript: &mut Transcript,
        ell: usize,
    ) -> (E::ScalarField, Vec<E::ScalarField>) {
        let mut betas =
            <Transcript as RangeProof<E, Proof<E>>>::challenges_for_quotient_polynomials(
                fs_transcript,
                ell,
            );
        let beta = betas
            .pop()
            .expect("The betas must have at least one element");
        (beta, betas)
    }

    #[allow(non_snake_case)]
    pub(crate) fn append_h_commitment<E: Pairing>(fs_transcript: &mut Transcript, D: &E::G1) {
        <Transcript as RangeProof<E, Proof<E>>>::append_h_commitment(fs_transcript, D);
    }

    pub(crate) fn get_mu_challenges<E: Pairing>(
        fs_transcript: &mut Transcript,
        ell: usize,
    ) -> (E::ScalarField, E::ScalarField, Vec<E::ScalarField>) {
        let mut mus = <Transcript as RangeProof<E, Proof<E>>>::challenges_for_linear_combination(
            fs_transcript,
            ell + 2,
        );

        let mu = mus.pop().expect("The mus must have at least one element");
        let mu_h = mus.pop().expect("The mus must have at least two elements");

        (mu, mu_h, mus)
    }

    #[allow(non_snake_case)]
    pub(crate) fn get_gamma_challenge<E: Pairing>(
        fs_transcript: &mut Transcript,
        roots_of_unity: &Vec<E::ScalarField>,
    ) -> E::ScalarField {
        loop {
            let gamma =
                <Transcript as RangeProof<E, Proof<E>>>::challenge_from_verifier(fs_transcript);
            if !roots_of_unity.contains(&gamma) {
                return gamma;
            }
        }
    }
}

/// This module defines a homomorphism that takes two scalar inputs and
/// maps them to a single group element output using two fixed base points.
/// Conceptually, this behaves similarly to a Pedersen commitment:
///
/// `output = base_1 * scalar_1 + base_2 * scalar_2`
///
/// The resulting sigma protocol is also known as Okamoto's protocol (see 19.5.1 in the book of Boneh-Shoup)
pub mod two_term_msm {
    // TODO: maybe fixed_base_msms should become a folder and put its code inside mod.rs? Then put this mod inside of that folder?
    use super::*;
    use crate::sigma_protocol::{homomorphism::fixed_base_msms, FirstProofItem};
    use aptos_crypto::arkworks::random::UniformRand;
    use aptos_crypto_derive::SigmaProtocolWitness;
    use ark_ec::AffineRepr;
    pub use sigma_protocol::homomorphism::TrivialShape as CodomainShape;
    pub type Proof<C> = sigma_protocol::Proof<
        <<C as CurveGroup>::Affine as AffineRepr>::ScalarField,
        Homomorphism<C>,
    >;

    impl<C: CurveGroup> Proof<C> {
        /// Generates a random looking proof (but not a valid one).
        /// Useful for testing and benchmarking. TODO: might be able to derive this through macros etc
        pub fn generate<R: rand::Rng + rand::CryptoRng>(rng: &mut R) -> Self {
            Self {
                first_proof_item: FirstProofItem::Commitment(CodomainShape(unsafe_random_point::<
                    C,
                    _,
                >(rng))),
                z: Witness {
                    poly_randomness: Scalar::rand(rng),
                    hiding_kzg_randomness: Scalar::rand(rng),
                },
            }
        }
    }

    /// Represents a homomorphism with two base points over an elliptic curve group.
    ///
    /// This structure defines a map from two scalars to one group element:
    /// `f(x1, x2) = base_1 * x1 + base_2 * x2`.
    #[derive(CanonicalSerialize, Clone, Debug, PartialEq, Eq)]
    pub struct Homomorphism<C: CurveGroup> {
        pub base_1: C::Affine,
        pub base_2: C::Affine,
    }

    #[derive(
        SigmaProtocolWitness, CanonicalSerialize, CanonicalDeserialize, Clone, Debug, PartialEq, Eq,
    )]
    pub struct Witness<F: PrimeField> {
        pub poly_randomness: Scalar<F>,
        pub hiding_kzg_randomness: Scalar<F>,
    }

    impl<C: CurveGroup> homomorphism::Trait for Homomorphism<C> {
        type Codomain = CodomainShape<C>;
        type CodomainNormalized = CodomainShape<C::Affine>;
        type Domain = Witness<C::ScalarField>;

        fn apply(&self, input: &Self::Domain) -> Self::Codomain {
            // Not doing `self.apply_msm(self.msm_terms(input))` because E::G1::msm is slower!
            // `msm_terms()` is still useful for verification though: there the code will use it to produce an MSM
            //  of size 2+2 (the latter two are for the first prover message A and the statement P)
            CodomainShape(
                self.base_1 * input.poly_randomness.0 + self.base_2 * input.hiding_kzg_randomness.0,
            )
        }

        fn normalize(&self, value: Self::Codomain) -> Self::CodomainNormalized {
            <Homomorphism<C> as fixed_base_msms::Trait>::normalize_output(value)
        }
    }

    impl<C: CurveGroup> fixed_base_msms::Trait for Homomorphism<C> {
        type Base = C::Affine;
        type CodomainShape<T>
            = CodomainShape<T>
        where
            T: CanonicalSerialize + CanonicalDeserialize + Clone + Eq + Debug;
        type MsmOutput = C;
        type Scalar = C::ScalarField;

        fn msm_terms(
            &self,
            input: &Self::Domain,
        ) -> Self::CodomainShape<MsmInput<Self::Base, Self::Scalar>> {
            let mut scalars = Vec::with_capacity(2);
            scalars.push(input.poly_randomness.0);
            scalars.push(input.hiding_kzg_randomness.0);

            let mut bases = Vec::with_capacity(2);
            bases.push(self.base_1);
            bases.push(self.base_2);

            CodomainShape(MsmInput { bases, scalars })
        }

        fn msm_eval(input: MsmInput<Self::Base, Self::Scalar>) -> Self::MsmOutput {
            C::msm(input.bases(), input.scalars()).expect("MSM failed in TwoTermMSM")
        }

        fn batch_normalize(msm_output: Vec<Self::MsmOutput>) -> Vec<Self::Base> {
            C::normalize_batch(&msm_output)
        }
    }

    impl<C: CurveGroup> sigma_protocol::CurveGroupTrait for Homomorphism<C> {
        type Group = C;

        fn dst(&self) -> Vec<u8> {
            b"DEKART_V2_SIGMA_PROTOCOL".to_vec()
        }
    }
}

/// The `n`th triangular number is the sum of the `n` natural numbers from 1 to `n`.
/// Here we compute the maximum `n` such that `1 + 2 + ... + n <= a`, using integer
/// arithmetic and the num_integer crate.
fn floored_triangular_root(a: usize) -> usize {
    // Solve `n*(n+1)/2 <= a`, or equivalently `n^2 + n - 2a <= 0`
    let discriminant = 1 + 8 * a;
    let sqrt_disc = discriminant.sqrt(); // integer sqrt
    (sqrt_disc - 1) / 2
}

#[cfg(test)]
mod test_floored_triangular_root {
    use super::floored_triangular_root;

    #[test]
    fn test_invert_triangular_number_small_values() {
        assert_eq!(floored_triangular_root(0), 0);
        assert_eq!(floored_triangular_root(1), 1); // 1 <= 1
        assert_eq!(floored_triangular_root(2), 1); // 1+2 > 2
        assert_eq!(floored_triangular_root(3), 2); // 1+2=3 <= 3
        assert_eq!(floored_triangular_root(5), 2); // 1+2+3=6 > 5
        assert_eq!(floored_triangular_root(6), 3);
    }
}
