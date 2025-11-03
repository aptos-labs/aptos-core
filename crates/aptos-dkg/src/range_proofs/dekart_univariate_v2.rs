// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file implements the range proof described here: https://alinush.github.io/dekart

use crate::{
    algebra::{polynomials, GroupGenerators},
    pcs::univariate_hiding_kzg,
    range_proofs::traits,
    sigma_protocol::{self, homomorphism, homomorphism::Trait as _, Trait as _},
    utils, Scalar,
};
use aptos_crypto::arkworks;
use ark_ec::{pairing::Pairing, CurveGroup, PrimeGroup, VariableBaseMSM};
use ark_ff::{AdditiveGroup, Field};
use ark_poly::{self, EvaluationDomain, Polynomial};
use ark_serialize::{
    CanonicalDeserialize, CanonicalSerialize, Compress, Read, SerializationError, Valid, Validate,
};
use ark_std::{
    rand::{CryptoRng, RngCore},
    UniformRand,
};
use num_integer::Roots;
use std::{fmt::Debug, io::Write};

#[allow(non_snake_case)]
#[derive(CanonicalSerialize, Debug, PartialEq, Eq, Clone, CanonicalDeserialize)]
pub struct Proof<E: Pairing> {
    hatC: E::G1,
    pi_PoK: sigma_protocol::Proof<E, two_term_msm::Homomorphism<E>>,
    Cs: Vec<E::G1>,
    D: E::G1,
    a: E::ScalarField,
    a_h: E::ScalarField,
    a_js: Vec<E::ScalarField>,
    pi_gamma: univariate_hiding_kzg::OpeningProof<E>,
}

#[derive(CanonicalSerialize, CanonicalDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct Commitment<E: Pairing>(pub(crate) E::G1);

#[allow(non_snake_case)]
#[derive(CanonicalSerialize, CanonicalDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct ProverKey<E: Pairing> {
    vk: VerificationKey<E>,
    pub(crate) ck_S: univariate_hiding_kzg::CommitmentKey<E>,
    max_n: usize,
    pub(crate) prover_precomputed: ProverPrecomputed<E>,
}

#[derive(CanonicalSerialize)]
pub struct PublicStatement<E: Pairing> {
    n: usize,
    ell: usize,
    comm: Commitment<E>,
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
        self.h_denom_eval[0].serialize_with_mode(&mut writer, compress)?;

        Ok(())
    }

    fn serialized_size(&self, compress: Compress) -> usize {
        let mut size = 0;
        size += self.powers_of_two.len().serialized_size(compress);
        size += self.powers_of_two[0].serialized_size(compress);
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
        let first_h_denom_eval =
            E::ScalarField::deserialize_with_mode(&mut reader, compress, validate)?;
        let first_h_denom_eval_as_u32 = arkworks::scalar_to_u32(&first_h_denom_eval)
            .expect("first_h_denom_eval did not fit in u32!");

        let powers_of_two = arkworks::powers_of_two::<E>(powers_len);

        let max_n = floored_triangular_root(first_h_denom_eval_as_u32 as usize);
        let roots_of_unity = arkworks::compute_roots_of_unity::<E>(max_n);
        let h_denom_eval = compute_h_denom_eval::<E>(&roots_of_unity);

        Ok(Self {
            powers_of_two,
            h_denom_eval,
        })
    }
}

// Required by CanonicalDeserialize
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

        let roots_of_unity = arkworks::compute_roots_of_unity::<E>(num_omegas);
        let powers_of_two = arkworks::powers_of_two::<E>(max_ell);

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
    type Commitment = Commitment<E>;
    type CommitmentKey = univariate_hiding_kzg::CommitmentKey<E>;
    type CommitmentRandomness = E::ScalarField;
    type Input = E::ScalarField;
    type ProverKey = ProverKey<E>;
    type PublicStatement = PublicStatement<E>;
    type VerificationKey = VerificationKey<E>;

    const DST: &[u8] = b"APTOS_UNIVARIATE_DEKART_V2_RANGE_PROOF_DST";

    fn commitment_key_from_prover_key(pk: &Self::ProverKey) -> Self::CommitmentKey {
        pk.ck_S.clone()
    }

    #[allow(non_snake_case)]
    fn setup<R: RngCore + CryptoRng>(
        max_n: usize,
        max_ell: usize,
        group_generators: GroupGenerators<E>,
        rng: &mut R,
    ) -> (ProverKey<E>, VerificationKey<E>) {
        let num_omegas = max_n + 1;
        assert!(num_omegas.is_power_of_two());

        // Generate trapdoor elements
        let trapdoor = univariate_hiding_kzg::Trapdoor::<E>::rand(rng);
        let xi_1_proj: E::G1 = group_generators.g1 * trapdoor.xi;

        let (vk_hkzg, ck_S) =
            univariate_hiding_kzg::setup(max_n + 1, group_generators.clone(), trapdoor, rng);

        let h_denom_eval = compute_h_denom_eval::<E>(&ck_S.roots_of_unity_in_eval_dom);

        let powers_of_two = arkworks::powers_of_two::<E>(max_ell);

        let prover_precomputed = ProverPrecomputed {
            powers_of_two: powers_of_two.clone(),
            h_denom_eval,
        };

        let verifier_precomputed = VerifierPrecomputed {
            powers_of_two,
            roots_of_unity: ck_S.roots_of_unity_in_eval_dom.clone(),
        };

        let vk = VerificationKey {
            xi_1: xi_1_proj.into_affine(),
            lagr_0: ck_S.lagr_g1[0],
            vk_hkzg,
            verifier_precomputed,
        };
        let prk = ProverKey {
            vk: vk.clone(),
            ck_S,
            max_n,
            prover_precomputed,
        };

        (prk, vk)
    }

    #[allow(non_snake_case)]
    fn commit_with_randomness(
        ck_S: &Self::CommitmentKey,
        values: &[Self::Input],
        rho: &Self::CommitmentRandomness,
    ) -> Commitment<E> {
        let mut values_shifted = vec![E::ScalarField::ZERO]; // start with 0,
        values_shifted.extend(values); // then append all values from the original vector

        let hiding_kzg_hom = univariate_hiding_kzg::CommitmentHomomorphism::<E> {
            lagr_g1: &ck_S.lagr_g1,
            xi_1: ck_S.xi_1,
        };

        let hiding_kzg_input = (*rho, values_shifted);

        Commitment(hiding_kzg_hom.apply(&hiding_kzg_input).0)
    }

    #[allow(non_snake_case)]
    fn prove<R>(
        pk: &ProverKey<E>,
        values: &[Self::Input],
        ell: usize,
        comm: &Self::Commitment,
        rho: &Self::CommitmentRandomness,
        fs_t: &mut merlin::Transcript,
        rng: &mut R,
    ) -> Proof<E>
    where
        R: RngCore + CryptoRng,
    {
        // Step 1a
        let ProverKey {
            vk,
            ck_S,
            max_n,
            prover_precomputed,
        } = pk;

        let n = values.len();
        let max_ell = prover_precomputed.powers_of_two.len();

        assert!(
            n <= *max_n,
            "n (got {}) must be ≤ max_n (which is {})",
            n,
            max_n
        );
        // TODO: Use a subdomain to make the FFTs smaller
        assert!(
            ell <= max_ell,
            "ell (got {}) must be ≤ max_ell (which is {})",
            ell,
            max_ell
        );

        let num_omegas = max_n + 1;

        let univariate_hiding_kzg::CommitmentKey {
            xi_1,
            lagr_g1,
            eval_dom,
            m_inv: num_omegas_inv,
            ..
        } = ck_S;

        debug_assert_eq!(
            *num_omegas_inv,
            E::ScalarField::from(num_omegas as u64).inverse().unwrap()
        );

        // Step 1b
        fiat_shamir::append_initial_data(fs_t, Self::DST, vk, PublicStatement {
            n,
            ell,
            comm: comm.clone(),
        });

        // Step 2a
        let r = E::ScalarField::rand(rng);
        let delta_rho = E::ScalarField::rand(rng);
        let hatC = *xi_1 * delta_rho + lagr_g1[0] * r + comm.0;

        // Step 2b
        fiat_shamir::append_hat_f_commitment::<E>(fs_t, &hatC);

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
            &two_term_msm::CodomainShape(hatC - comm.0),
            fs_t,
            rng,
        );

        // Step 3b
        fiat_shamir::append_sigma_proof(fs_t, &pi_PoK); // TODO: should be changed to "remainder of sigma proof" since the first message is already in there

        // Step 4a
        let rs: Vec<E::ScalarField> = (0..ell).map(|_| E::ScalarField::rand(rng)).collect();

        let f_js_evals: Vec<Vec<E::ScalarField>> = {
            let mut f_js_evals = vec![Vec::with_capacity(num_omegas); ell];

            for j in 0..ell {
                f_js_evals[j].push(rs[j]);
            }

            for &val in values.iter() {
                let bits = utils::scalar_to_bits_le::<E>(&val);
                for j in 0..ell {
                    f_js_evals[j].push(E::ScalarField::from(bits[j]));
                }
            }

            for f_j in &mut f_js_evals {
                f_j.resize(num_omegas, E::ScalarField::ZERO);
            }

            f_js_evals
        };

        let rhos: Vec<E::ScalarField> = std::iter::repeat_with(|| E::ScalarField::rand(rng))
            .take(ell)
            .collect();

        let hkzg_commitment_hom = univariate_hiding_kzg::CommitmentHomomorphism::<E> {
            lagr_g1,
            xi_1: *xi_1,
        };
        let Cs: Vec<_> = f_js_evals
            .iter()
            .zip(rhos.iter())
            .map(|(f_j, rho)| {
                let hkzg_commit_input = (*rho, f_j.clone());
                hkzg_commitment_hom.apply(&hkzg_commit_input).0
            })
            .collect();

        // Step 4b
        fiat_shamir::append_f_j_commitments::<E>(fs_t, &Cs);

        // Step 6
        let (beta, betas) = fiat_shamir::get_beta_challenges::<E>(fs_t, ell);

        let hat_f_evals: Vec<E::ScalarField> = {
            let mut v = Vec::with_capacity(num_omegas);
            v.push(r);
            v.extend_from_slice(values);
            v.resize(num_omegas, E::ScalarField::ZERO);
            v
        };

        let hat_f_coeffs = eval_dom.ifft(&hat_f_evals);
        debug_assert_eq!(hat_f_coeffs.len(), pk.max_n + 1);

        let diff_hat_f_evals: Vec<E::ScalarField> = {
            let mut result = polynomials::differentiate(&hat_f_coeffs);
            eval_dom.fft_in_place(&mut result);
            result
        };

        let f_j_coeffs: Vec<Vec<E::ScalarField>> = (0..ell)
            .map(|j| {
                let mut f_j = f_js_evals[j].clone();
                debug_assert_eq!(f_j.len(), pk.max_n + 1);
                eval_dom.ifft_in_place(&mut f_j);
                debug_assert_eq!(f_j.len(), pk.max_n + 1);
                f_j
            })
            .collect();

        let diff_f_js_evals: Vec<Vec<E::ScalarField>> = f_js_evals
            .iter()
            .map(|f_j_eval| {
                let mut result = eval_dom.ifft(f_j_eval); // Convert to coefficients
                polynomials::differentiate_in_place(&mut result); // Differentiate
                eval_dom.fft_in_place(&mut result); // Convert back to evaluations
                result
            })
            .collect();

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

        let rho_h = E::ScalarField::rand(rng);
        let D = hkzg_commitment_hom.apply(&(rho_h, h_evals.clone())).0;

        // Step 7b
        fiat_shamir::append_h_commitment::<E>(fs_t, &D);

        // Step 8
        let (mu, mu_h, mus) = fiat_shamir::get_mu_challenges::<E>(fs_t, ell);

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

        // Step 9
        let gamma = fiat_shamir::get_gamma_challenge::<E>(fs_t, &ck_S.roots_of_unity_in_eval_dom);

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

        let a_js: Vec<E::ScalarField> = (0..ell)
            .map(|i| {
                let poly = ark_poly::univariate::DensePolynomial {
                    coeffs: f_j_coeffs[i].clone(),
                };
                poly.evaluate(&gamma)
            }) // Again, using Horner's here
            .collect();

        // Step 10
        let s = E::ScalarField::rand(rng);

        let rho_u = mu * (*rho + delta_rho)
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
            &univariate_hiding_kzg::CommitmentRandomness(s),
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
    fn verify(
        &self,
        vk: &Self::VerificationKey,
        n: usize,
        ell: usize,
        comm: &Self::Commitment,
        fs_t: &mut merlin::Transcript,
    ) -> anyhow::Result<()> {
        // Step 1
        let VerificationKey {
            xi_1,
            lagr_0,
            vk_hkzg,
            verifier_precomputed,
        } = vk;

        assert!(
            ell <= verifier_precomputed.powers_of_two.len(),
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
        fiat_shamir::append_initial_data(fs_t, Self::DST, vk, PublicStatement {
            n,
            ell,
            comm: comm.clone(),
        });

        // Step 2b
        fiat_shamir::append_hat_f_commitment::<E>(fs_t, &hatC);

        // Step 3
        two_term_msm::Homomorphism {
            base_1: *lagr_0,
            base_2: *xi_1,
        }
        .verify(&(two_term_msm::CodomainShape(*hatC - comm.0)), pi_PoK, fs_t)?;

        // Step 4a
        fiat_shamir::append_sigma_proof(fs_t, &pi_PoK);

        // Step 4b
        fiat_shamir::append_f_j_commitments::<E>(fs_t, &Cs);

        // Step 5
        let (beta, beta_js) = fiat_shamir::get_beta_challenges::<E>(fs_t, ell);

        // Step 6
        fiat_shamir::append_h_commitment::<E>(fs_t, &D);

        // Step 7
        let (mu, mu_h, mu_js) = fiat_shamir::get_mu_challenges::<E>(fs_t, ell);

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

        // Step 9
        let gamma =
            fiat_shamir::get_gamma_challenge::<E>(fs_t, &verifier_precomputed.roots_of_unity);

        // Step 10
        let a_u = *a * mu
            + *a_h * mu_h
            + a_js
                .iter()
                .zip(&mu_js)
                .map(|(&a_j, &mu_j)| a_j * mu_j)
                .sum::<E::ScalarField>();

        univariate_hiding_kzg::CommitmentHomomorphism::verify(
            vk_hkzg.clone(),
            univariate_hiding_kzg::Commitment(U),
            gamma,
            a_u,
            pi_gamma.clone(),
        )?;

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

        Ok(())
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
        pi_PoK: &sigma_protocol::Proof<E, two_term_msm::Homomorphism<E>>,
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
pub mod two_term_msm {
    // TODO: maybe fixed_base_msms should become a folder and put its code inside mod.rs? Then put this mod inside of that folder?
    use super::*;
    use crate::sigma_protocol::homomorphism::fixed_base_msms;
    use aptos_crypto_derive::SigmaProtocolWitness;
    pub use sigma_protocol::homomorphism::TrivialShape as CodomainShape;

    /// Represents a homomorphism with two base points over an elliptic curve group.
    ///
    /// This structure defines a map from two scalars to one group element:
    /// `f(x1, x2) = base_1 * x1 + base_2 * x2`.
    #[derive(CanonicalSerialize, Clone, Debug, PartialEq, Eq)]
    pub struct Homomorphism<E: Pairing> {
        pub base_1: E::G1Affine,
        pub base_2: E::G1Affine,
    }

    #[derive(
        SigmaProtocolWitness, CanonicalSerialize, CanonicalDeserialize, Clone, Debug, PartialEq, Eq,
    )]
    pub struct Witness<E: Pairing> {
        pub poly_randomness: Scalar<E>,
        pub hiding_kzg_randomness: Scalar<E>,
    }

    impl<E: Pairing> homomorphism::Trait for Homomorphism<E> {
        type Codomain = CodomainShape<E::G1>;
        type Domain = Witness<E>;

        fn apply(&self, input: &Self::Domain) -> Self::Codomain {
            // Not doing `self.apply_msm(self.msm_terms(input))` because E::G1::msm is slower!
            // `msm_terms()` is still useful for verification though: there the code will use it to produce an MSM
            //  of size 2+2 (the latter two are for the first prover message A and the statement P)
            CodomainShape(
                self.base_1 * input.poly_randomness.0 + self.base_2 * input.hiding_kzg_randomness.0,
            )
        }
    }

    impl<E: Pairing> fixed_base_msms::Trait for Homomorphism<E> {
        type Base = E::G1Affine;
        type CodomainShape<T>
            = CodomainShape<T>
        where
            T: CanonicalSerialize + CanonicalDeserialize + Clone + Eq + Debug;
        type MsmInput = fixed_base_msms::MsmInput<Self::Base, Self::Scalar>;
        type MsmOutput = E::G1;
        type Scalar = E::ScalarField;

        fn msm_terms(&self, input: &Self::Domain) -> Self::CodomainShape<Self::MsmInput> {
            let mut scalars = Vec::with_capacity(2);
            scalars.push(input.poly_randomness.0);
            scalars.push(input.hiding_kzg_randomness.0);

            let mut bases = Vec::with_capacity(2);
            bases.push(self.base_1);
            bases.push(self.base_2);

            CodomainShape(fixed_base_msms::MsmInput { bases, scalars })
        }

        fn msm_eval(bases: &[Self::Base], scalars: &[Self::Scalar]) -> Self::MsmOutput {
            E::G1::msm(bases, scalars).expect("MSM failed in TwoTermMSM")
        }
    }

    impl<E: Pairing> sigma_protocol::Trait<E> for Homomorphism<E> {
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
