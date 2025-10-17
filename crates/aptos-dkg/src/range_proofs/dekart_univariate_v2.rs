// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    algebra::{polynomials, polynomials as OURpolynomials, GroupGenerators},
    pcs::univariate_hiding_kzg,
    range_proofs::traits,
    sigma_protocol::{
        self, homomorphism,
        homomorphism::{fixed_base_msms::Trait as FixedBaseMsmsTrait, Trait as HomomorphismTrait},
        Trait,
    },
    utils, Scalar,
};
use ark_ec::{pairing::Pairing, CurveGroup, PrimeGroup, VariableBaseMSM};
use ark_ff::{AdditiveGroup, Field};
use ark_poly::{self, EvaluationDomain, Polynomial};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::{
    rand::{CryptoRng, RngCore},
    UniformRand,
};

pub const DST: &[u8; 42] = b"APTOS_UNIVARIATE_DEKART_V2_RANGE_PROOF_DST";

#[allow(non_snake_case)]
#[derive(CanonicalSerialize, Clone, CanonicalDeserialize)]
pub struct Proof<E: Pairing> {
    hatC: E::G1,
    pi_PoK: sigma_protocol::Proof<E, two_term_msm::Homomorphism<E>>,
    Cj: Vec<E::G1>,
    D: E::G1,
    a: E::ScalarField,
    a_h: E::ScalarField,
    aj: Vec<E::ScalarField>,
    pi_gamma: univariate_hiding_kzg::Proof<E>,
}

#[derive(CanonicalSerialize, CanonicalDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct Commitment<E: Pairing>(E::G1);

#[allow(non_snake_case)]
#[derive(Clone, Debug)]
pub struct ProverKey<E: Pairing> {
    vk: VerificationKey<E>,
    ck_S: univariate_hiding_kzg::CommitmentKey<E>,
    ck_L: univariate_hiding_kzg::CommitmentKey<E>, // Not using this yet
    max_n: usize,
    precomputed_stuff: PrecomputedStuff<E>,
}

#[derive(CanonicalSerialize)]
pub struct PublicStatement<E: Pairing> {
    n: usize,
    ell: usize,
    comm: Commitment<E>,
}

#[derive(CanonicalSerialize, Clone, Debug, PartialEq, Eq)]
pub struct VerificationKey<E: Pairing> {
    b: usize,
    xi_1: E::G1Affine,
    zeroth_lagr: E::G1Affine,
    xi_2: E::G2Affine,
    tau_2: E::G2Affine,
    group_data: GroupGenerators<E>,
    roots_of_unity: Vec<E::ScalarField>,
}

#[derive(CanonicalSerialize, Clone, Debug)]
pub struct PrecomputedStuff<E: Pairing> {
    powers_of_two: Vec<E::ScalarField>,
    h_denom_eval: Vec<E::ScalarField>,
}

impl<E: Pairing> traits::BatchedRangeProof<E> for Proof<E> {
    type Commitment = Commitment<E>;
    type CommitmentKey = univariate_hiding_kzg::CommitmentKey<E>;
    type CommitmentRandomness = E::ScalarField;
    type Input = E::ScalarField;
    type ProverKey = ProverKey<E>;
    type PublicStatement = PublicStatement<E>;
    type VerificationKey = VerificationKey<E>;

    const DST: &[u8] = DST;

    fn commitment_key_from_prover_key(pk: &Self::ProverKey) -> Self::CommitmentKey {
        pk.ck_S.clone()
    }

    #[allow(non_snake_case)]
    fn setup<R: RngCore + CryptoRng>(
        max_n: usize,
        max_ell: usize,
        rng: &mut R,
    ) -> (ProverKey<E>, VerificationKey<E>) {
        let group = GroupGenerators::sample(rng);
        let g1 = group.g1; // TODO: make group part of the setup(...) in trait?

        let max_n = (max_n + 1).next_power_of_two() - 1;
        let num_omegas = max_n + 1;
        debug_assert!(num_omegas.is_power_of_two());

        let xi = E::ScalarField::rand(rng);
        let tau = E::ScalarField::rand(rng);
        let trapdoor = univariate_hiding_kzg::Trapdoor { xi, tau };
        let xi_1_proj: E::G1 = g1 * xi;

        let (
            univariate_hiding_kzg::VerificationKey {
                xi_2,
                tau_2,
                group_data: _,
            },
            ck_S,
        ) = univariate_hiding_kzg::setup(max_n + 1, group.clone(), trapdoor.clone(), rng);

        let L = 2 * num_omegas; // Not using this yet

        let (_, ck_L) = univariate_hiding_kzg::setup(L, group.clone(), trapdoor, rng); // Not using this yet

        let n_plus_1_inv = E::ScalarField::from((num_omegas) as u64).inverse().unwrap();
        // let mut omega_in_pows: Vec<E::ScalarField> = (1..=max_n + 1) // TODO: uh =??
        //     .map(|i| ck_S.roots_of_unity_in_eval_dom[(i * max_n) % (max_n + 1)]) // safe modulo access
        //     .collect();

        // // Batch invert them
        // ark_ff::batch_inversion(&mut omega_in_pows);

        // Compute results
        //let mut precomputed_stuff = Vec::with_capacity(max_n + 1);
        // for (i, omega_in_inv) in omega_in_pows.into_iter().enumerate() {
        //     let i_plus_1 = i + 1;
        //     let omega_i = ck_S.roots_of_unity_in_eval_dom[i_plus_1 % (max_n + 1)];
        //     let numerator = omega_i - E::ScalarField::ONE;
        //     let value = numerator * n_plus_1_inv * omega_in_inv;
        //     precomputed_stuff.push(value);
        // }

        let powers_of_two: Vec<E::ScalarField> = (0..max_ell)
            .map(|j| E::ScalarField::from(1i64 << (j as u32)))
            .collect();

        let mut h_denom_eval = Vec::with_capacity(num_omegas);
        let first_val = E::ScalarField::from((max_n * (max_n + 1) / 2) as u64)
            .inverse()
            .expect("Value should be invertible");
        h_denom_eval.push(first_val);
        let remaining_val: Vec<E::ScalarField> = ck_S
            .roots_of_unity_in_eval_dom
            .iter()
            .skip(1) // skip the first root omega^0
            .take(max_n) // Not sure this is needed
            .map(|root| {
                let root_val = *root;
                (root_val * (root_val - E::ScalarField::ONE))
                    / E::ScalarField::from(num_omegas as u64)
            })
            .collect();
        h_denom_eval.extend(remaining_val);

        let precomputed_stuff = PrecomputedStuff {
            powers_of_two,
            h_denom_eval,
        };

        let vk = VerificationKey {
            b: 2,
            xi_1: xi_1_proj.into_affine(),
            zeroth_lagr: ck_S.lagr_g1[0],
            xi_2,
            tau_2,
            group_data: group,
            roots_of_unity: ck_S.roots_of_unity_in_eval_dom.clone(),
        };
        let prk = ProverKey {
            vk: vk.clone(),
            ck_S,
            ck_L,
            max_n,
            precomputed_stuff,
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

        let hiding_kzg_hom = univariate_hiding_kzg::Homomorphism::<E> {
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
        fs_transcript: &mut merlin::Transcript,
        rng: &mut R,
    ) -> Proof<E>
    where
        R: RngCore + CryptoRng,
    {
        // Step 1a
        let ProverKey {
            vk,
            ck_S,
            ck_L: _,
            max_n,
            precomputed_stuff,
        } = pk;

        let n = values.len();
        let max_ell = precomputed_stuff.powers_of_two.len();

        assert!(
            n <= *max_n,
            "n (got {}) must be ≤ max_n (which is {})",
            n,
            max_n
        );
        assert!(
            ell <= max_ell,
            "ell (got {}) must be ≤ max_ell (which is {})",
            ell,
            max_ell
        );

        let num_omegas = max_n + 1;

        let univariate_hiding_kzg::CommitmentKey {
            xi_1,
            tau_1: _,
            lagr_g1,
            eval_dom,
            roots_of_unity_in_eval_dom: _,
            one_1: _,
        } = ck_S;
        // let lagr2_g1 = ck_L.lagr_g1;   // Not used yet

        // Step 1b
        fiat_shamir::append_initial_data(fs_transcript, DST, vk, n, ell, &comm);

        // Step 2a
        let r = E::ScalarField::rand(rng);
        let delta_rho = E::ScalarField::rand(rng);
        let hatC = *xi_1 * delta_rho + lagr_g1[0] * r + comm.0;

        // Step 2b
        fiat_shamir::append_hat_f_commitment::<E>(fs_transcript, &hatC);

        // Step 3a
        let sigma_protocol = two_term_msm::Homomorphism {
            base_1: lagr_g1[0],
            base_2: *xi_1,
        };
        let pi_PoK = sigma_protocol.prove(
            &two_term_msm::Witness {
                kzg_randomness: Scalar(r),
                hiding_kzg_randomness: Scalar(delta_rho),
            },
            fs_transcript,
            rng,
        );

        // Step 3b
        fiat_shamir::append_sigma_proof(fs_transcript, &pi_PoK);

        // Step 4a
        let hkzg_commit = univariate_hiding_kzg::Homomorphism::<E> {
            lagr_g1,
            xi_1: *xi_1,
        };

        let values_resized = {
            let mut v = Vec::with_capacity(*max_n);
            v.extend_from_slice(values);
            v.resize(*max_n, E::ScalarField::ZERO);
            v
        };

        // z_bits[i][j] = z_ij, i.e. the j-bit of z_i (little-endian)
        let z_bits: Vec<Vec<bool>> = values_resized
            .iter()
            .map(|z_val| {
                utils::scalar_to_bits_le::<E>(z_val)
                    .into_iter()
                    .take(ell)
                    .collect::<Vec<_>>()
            })
            .collect();
        // Debug assert: reconstruct z[0] from z_bits[0]
        debug_assert_eq!(
            values_resized[0],
            z_bits[0]
                .iter()
                .enumerate()
                .fold(E::ScalarField::ZERO, |acc, (i, &bit)| {
                    if bit {
                        acc + precomputed_stuff.powers_of_two[i]
                    } else {
                        acc
                    }
                }),
            "Reconstructed value from z_bits[0] does not match values[0]"
        );

        // f_j_evals_without_r[j][i] = z_ij
        let f_j_evals_without_r: Vec<Vec<E::ScalarField>> = (0..ell)
            .map(|j| {
                z_bits
                    .iter()
                    .map(|z_i| E::ScalarField::from(z_i[j]))
                    .collect()
            })
            .collect(); // This is just transposing the bits matrix, also moving them into E::ScalarField

        let rs: Vec<E::ScalarField> = std::iter::repeat_with(|| E::ScalarField::rand(rng))
            .take(ell)
            .collect();

        let f_js_evals: Vec<Vec<E::ScalarField>> = (0..ell)
            .map(|j| {
                let mut f_j_evals = Vec::with_capacity(num_omegas);
                f_j_evals.push(rs[j]);
                f_j_evals.extend_from_slice(&f_j_evals_without_r[j]);
                f_j_evals
            })
            .collect();

        let rhos: Vec<E::ScalarField> = std::iter::repeat_with(|| E::ScalarField::rand(rng))
            .take(ell)
            .collect();

        let Cj: Vec<_> = f_js_evals
            .iter()
            .zip(rhos.iter())
            .map(|(f_j, rho)| {
                let hkzg_commit_input = (*rho, f_j.clone());
                hkzg_commit.apply(&hkzg_commit_input).0
            })
            .collect();

        // Step 4b
        fiat_shamir::append_f_j_commitments::<E>(fs_transcript, &Cj);

        // Step 6
        let (beta, betas) = fiat_shamir::get_beta_and_betas::<E>(fs_transcript, ell);

        let mut hat_f_evals = Vec::with_capacity(1 + values.len());
        hat_f_evals.push(r);
        hat_f_evals.extend_from_slice(values); // TODO: resize to max_n? seems it's not needed

        let hat_f_coeffs = eval_dom.ifft(&hat_f_evals);

        let diff_hat_f = polynomials::differentiate(&hat_f_coeffs);

        let diff_hat_f_evals = eval_dom.fft(&diff_hat_f);

        let f_j_coeffs: Vec<Vec<E::ScalarField>> = (0..ell)
            .map(|j| {
                let mut f_j = f_js_evals[j].clone();
                assert_eq!(f_j.len(), pk.max_n + 1);
                eval_dom.ifft_in_place(&mut f_j);
                assert_eq!(f_j.len(), pk.max_n + 1);
                f_j
            })
            .collect();

        let diff_f_js_evals: Vec<Vec<E::ScalarField>> = f_js_evals
            .iter()
            .map(|f_j_eval| {
                let mut coeffs = f_j_eval.clone();
                eval_dom.ifft_in_place(&mut coeffs); // Convert to coefficients
                let diff_coeffs = polynomials::differentiate(&coeffs); // Differentiate
                eval_dom.fft(&diff_coeffs) // Convert back to evaluations
            })
            .collect();

        // N_prime[j][i] = ...
        let N_prime: Vec<Vec<E::ScalarField>> = diff_f_js_evals
            .iter()
            .zip(f_js_evals.iter())
            .map(|(diff_f_j, f_j)| {
                diff_f_j
                    .iter()
                    .zip(f_j.iter())
                    .map(|(&diff_f_j_i, &f_j_i)| {
                        diff_f_j_i * (E::ScalarField::from(2u64) * f_j_i - E::ScalarField::ONE)
                    })
                    .collect()
            })
            .collect();

        let mut h_evals: Vec<E::ScalarField> = (1..num_omegas)
            .map(|i| {
                // First term: beta * diff_hat_f_evals[i]
                let mut val = diff_hat_f_evals[i];

                // Second term: -beta * sum_j 2^j * f_j_evals[j][i]
                let sum1: E::ScalarField = diff_f_js_evals
                    .iter()
                    .enumerate()
                    .map(|(j, diff_f_j)| {
                        let pow2 = E::ScalarField::from(1u64 << j); // 2^j add asserts that ell is less than 64, and make this better
                        pow2 * diff_f_j[i]
                    })
                    .sum();
                val -= sum1;
                val *= beta;

                // Third term: sum_j betas[j] * N_prime[j][i]
                let sum2: E::ScalarField = N_prime
                    .iter()
                    .enumerate()
                    .map(|(j, N_j_prime)| betas[j] * N_j_prime[i])
                    .sum();
                val += sum2;

                // Divide by denoms[i]
                val * precomputed_stuff.h_denom_eval[i]
            })
            .collect();

        let first_h_eval = {
            let mut pow2 = E::ScalarField::ONE;
            let mut sum_pow2_fj = E::ScalarField::ZERO;
            for f_j_eval in f_js_evals.iter() {
                sum_pow2_fj += pow2 * f_j_eval[0];
                pow2 = pow2.double(); // multiply by 2 each iteration
            }

            // sum_j betas[j] * f_j(0) * (f_j(0) - 1)
            let mut sum_betas_term = E::ScalarField::ZERO;
            for (beta_j, f_j) in betas.iter().zip(f_js_evals.iter()) {
                sum_betas_term += *beta_j * f_j[0] * (f_j[0] - E::ScalarField::ONE);
            }

            // numerator: β * (f_evals[0] - Σ 2^j f_j_evals[0]) + Σ β_j f_j(0)(f_j(0) - 1)
            let numerator = beta * (hat_f_evals[0] - sum_pow2_fj) + sum_betas_term;

            numerator / E::ScalarField::from((n + 1) as u64)
        };
        let first_h_eval_two = {
            let mut pow2 = E::ScalarField::ONE;
            let mut sum_pow2_rs = E::ScalarField::ZERO;
            for r_j in rs.iter() {
                sum_pow2_rs += pow2 * r_j;
                pow2 = pow2.double(); // multiply by 2 each iteration
            }

            // sum_j betas[j] * f_j(0) * (f_j(0) - 1)
            let mut sum_betas_term = E::ScalarField::ZERO;
            for (beta_j, r_j) in betas.iter().zip(rs.iter()) {
                sum_betas_term += *beta_j * r_j * (*r_j - E::ScalarField::ONE);
            }

            // numerator: β * (f_evals[0] - Σ 2^j f_j_evals[0]) + Σ β_j f_j(0)(f_j(0) - 1)
            let numerator = beta * (r - sum_pow2_rs) + sum_betas_term;

            numerator / E::ScalarField::from((n + 1) as u64)
        };
        assert_eq!(first_h_eval, first_h_eval_two);
        h_evals.insert(0, first_h_eval);

        let rho_h = E::ScalarField::rand(rng);
        let D = hkzg_commit.apply(&(rho_h, h_evals.clone())).0;

        // Step 7b
        fiat_shamir::append_h_commitment::<E>(fs_transcript, &D);

        // Step 8
        let (mu, mu_h, mus) = fiat_shamir::get_mu_challenges::<E>(fs_transcript, ell);

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
        let gamma =
            fiat_shamir::get_gamma_challenge::<E>(fs_transcript, &ck_S.roots_of_unity_in_eval_dom);

        let a: E::ScalarField = {
            let poly = ark_poly::univariate::DensePolynomial {
                coeffs: hat_f_coeffs,
            };
            poly.evaluate(&gamma)
        }; // This algorithm should be Horner's

        let a_h =
            OURpolynomials::barycentric_eval(&h_evals, &ck_S.roots_of_unity_in_eval_dom, gamma);

        let a_j: Vec<E::ScalarField> = (0..ell)
            .map(|i| {
                let poly = ark_poly::univariate::DensePolynomial {
                    coeffs: f_j_coeffs[i].clone(),
                };
                poly.evaluate(&gamma)
            }) // This algorithm should be Horner's
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

        let pi_gamma = univariate_hiding_kzg::Homomorphism::open(
            ck_S,
            u_values,
            rho_u,
            gamma,
            &univariate_hiding_kzg::CommitmentRandomness(s),
        );

        Proof {
            hatC,
            pi_PoK,
            Cj,
            D,
            a,
            a_h,
            aj: a_j,
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
        fs_transcript: &mut merlin::Transcript,
    ) -> anyhow::Result<()> {
        // assert!(
        //     ell <= vk.max_ell,
        //     "ell (got {}) must be ≤ max_ell (which is {})",
        //     ell,
        //     vk.max_ell
        // );

        // Step 1
        let VerificationKey {
            b: _,
            xi_1,
            zeroth_lagr,
            xi_2,
            tau_2,
            group_data,
            roots_of_unity,
        } = vk;
        let Proof {
            hatC,
            pi_PoK,
            Cj,
            D,
            a,
            a_h,
            aj,
            pi_gamma,
        } = self;

        // Step 2a
        fiat_shamir::append_initial_data(fs_transcript, DST, vk, n, ell, &comm);

        // Step 2b
        fiat_shamir::append_hat_f_commitment::<E>(fs_transcript, &hatC);

        // Step 2c
        let sigma_protocol = two_term_msm::Homomorphism {
            base_1: *zeroth_lagr,
            base_2: *xi_1,
        };
        assert!(sigma_protocol
            .verify(
                &two_term_msm::CodomainShape(*hatC - comm.0),
                pi_PoK,
                fs_transcript,
            )
            .is_ok()); // TODO: propagate error

        fiat_shamir::append_sigma_proof(fs_transcript, &pi_PoK);

        // Step 2d
        fiat_shamir::append_f_j_commitments::<E>(fs_transcript, &Cj);

        // Step 3
        let (beta, betas) = fiat_shamir::get_beta_and_betas::<E>(fs_transcript, ell);

        // Step 4
        fiat_shamir::append_h_commitment::<E>(fs_transcript, &D);

        let (mu, mu_h, mus) = fiat_shamir::get_mu_challenges::<E>(fs_transcript, ell);

        // Step 6
        let U_bases_proj: Vec<E::G1> = std::iter::once(*hatC)
            .chain(std::iter::once(*D))
            .chain(Cj.iter().copied())
            .collect();
        let U_bases = E::G1::normalize_batch(&U_bases_proj);

        let U_scalars: Vec<_> = std::iter::once(mu)
            .chain(std::iter::once(mu_h))
            .chain(mus.iter().copied())
            .collect();

        let U = E::G1::msm(&U_bases, &U_scalars).expect("problem computing MSM in DeKART v2");

        // Step 7
        let gamma = fiat_shamir::get_gamma_challenge::<E>(fs_transcript, &roots_of_unity);

        // Step 8
        let a_u = *a * mu
            + *a_h * mu_h
            + aj.iter()
                .zip(&mus)
                .map(|(&a_j, &mu_j)| a_j * mu_j)
                .sum::<E::ScalarField>();

        univariate_hiding_kzg::Homomorphism::verify(
            univariate_hiding_kzg::VerificationKey {
                xi_2: *xi_2,
                tau_2: *tau_2,
                group_data: group_data.clone(),
            },
            univariate_hiding_kzg::Commitment(U),
            gamma,
            a_u,
            pi_gamma.clone(),
        )?;

        // assert!(univariate_hiding_kzg::Homomorphism::verify(
        //     univariate_hiding_kzg::VerificationKey {
        //         xi_2: *xi_2,
        //         tau_2: *tau_2,
        //         group_data: group_data.clone()
        //     },
        //     univariate_hiding_kzg::Commitment(U),
        //     gamma,
        //     a_u,
        //     pi_gamma.clone()
        // ).is_ok());

        // Step 9
        let gamma_pow = gamma.pow(&[(n + 1) as u64]); // gamma^(n+1) // TODO: change to some max_n ?
        let V_eval_gamma =
            (gamma_pow - E::ScalarField::ONE) * (gamma - E::ScalarField::ONE).inverse().unwrap();

        let powers_of_two: Vec<E::ScalarField> = (0..ell)
            .map(|j| E::ScalarField::from(1i64 << (j as u32)))
            .collect();

        //        let LHS = *a_h * V_eval_gamma;
        let LHS = *a_h;

        let sum1: E::ScalarField = powers_of_two
            .iter()
            .zip(aj.iter())
            .map(|(p, a)| *p * *a)
            .sum();

        let sum2: E::ScalarField = betas
            .iter()
            .zip(aj.iter())
            .map(|(b, a)| *b * *a * (*a - E::ScalarField::ONE))
            .sum();

        //        let RHS = beta * (*a - sum1) + sum2;
        let RHS = (beta * (*a - sum1) + sum2) / V_eval_gamma;

        assert_eq!(LHS, RHS); // TODO!!!!!

        Ok(())
    }

    fn maul(&mut self) {
        self.D = self.D + E::G1::generator();
    }
}

mod fiat_shamir {
    use super::*;
    use crate::fiat_shamir::RangeProof;
    use merlin::Transcript;

    pub(crate) fn append_initial_data<E: Pairing>(
        fs_transcript: &mut Transcript,
        dst: &[u8],
        vk: &VerificationKey<E>,
        n: usize,
        ell: usize,
        comm: &Commitment<E>,
    ) {
        <Transcript as RangeProof<E, Proof<E>>>::append_sep(fs_transcript, dst);
        <Transcript as RangeProof<E, Proof<E>>>::append_vk(fs_transcript, vk);
        <Transcript as RangeProof<E, Proof<E>>>::append_public_statement(
            fs_transcript,
            PublicStatement {
                n,
                ell,
                comm: comm.clone(),
            },
        );
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
        Cj: &Vec<E::G1>,
    ) {
        <Transcript as RangeProof<E, Proof<E>>>::append_f_j_commitments(fs_transcript, Cj);
    }

    pub(crate) fn get_beta_and_betas<E: Pairing>(
        fs_transcript: &mut Transcript,
        ell: usize,
    ) -> (E::ScalarField, Vec<E::ScalarField>) {
        let mut betas =
            <Transcript as RangeProof<E, Proof<E>>>::challenges_for_quotient_polynomials(
                fs_transcript,
                ell,
            );
        let beta = betas.pop().expect("betas must have at least one element");
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

        let mu = mus.pop().expect("mus must have at least one element");
        let mu_h = mus.pop().expect("mus must have at least two elements");

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

pub mod two_term_msm {
    // TODO: maybe fixed_base_msms should become a folder? Then put this inside of that?
    use super::*;
    use crate::sigma_protocol::homomorphism::fixed_base_msms;
    use aptos_crypto_derive::SigmaProtocolWitness;
    pub use sigma_protocol::homomorphism::TrivialShape as CodomainShape;

    #[derive(CanonicalSerialize, Clone, Debug, PartialEq, Eq)]
    pub struct Homomorphism<E: Pairing> {
        // This is rather similar to a Pedersen commitment
        pub base_1: E::G1Affine,
        pub base_2: E::G1Affine,
    }

    impl<E: Pairing> Default for Homomorphism<E> {
        // I guess Default is a bad name, there is none?
        fn default() -> Self {
            let base_1 = E::G1::generator().into_affine();
            let base_2 = (base_1 * E::ScalarField::from(123456789u64)).into_affine();
            Self { base_1, base_2 }
        }
    }

    #[derive(SigmaProtocolWitness, CanonicalSerialize, CanonicalDeserialize, Clone)]
    pub struct Witness<E: Pairing> {
        pub kzg_randomness: Scalar<E>,
        pub hiding_kzg_randomness: Scalar<E>,
    }

    impl<'a, E: Pairing> homomorphism::Trait for Homomorphism<E> {
        type Codomain = CodomainShape<E::G1>;
        type Domain = Witness<E>;

        fn apply(&self, input: &Self::Domain) -> Self::Codomain {
            self.apply_msm(self.msm_terms(input))
        }
    }

    impl<'a, E: Pairing> fixed_base_msms::Trait for Homomorphism<E> {
        type Base = E::G1Affine;
        type CodomainShape<T>
            = CodomainShape<T>
        where
            T: CanonicalSerialize + CanonicalDeserialize + Clone;
        type MsmInput = fixed_base_msms::MsmInput<Self::Base, Self::Scalar>;
        type MsmOutput = E::G1;
        type Scalar = E::ScalarField;

        fn msm_terms(&self, input: &Self::Domain) -> Self::CodomainShape<Self::MsmInput> {
            let mut scalars = Vec::with_capacity(2);
            scalars.push(input.kzg_randomness.0);
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

        fn dst_verifier(&self) -> Vec<u8> {
            b"DEKART_V2_SIGMA_PROTOCOL_VERIFIER".to_vec()
        }
    }
}
