// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// A lot of this code is copy-pasted from `jolt-core`. TODO: benchmark them against each other

// THIS CODE HAS NOT YET BEEN VETTED, ONLY USE FOR BENCHMARKING PURPOSES!!!!!

use crate::{
    fiat_shamir::PolynomialCommitmentScheme as _,
    pcs::{
        traits::PolynomialCommitmentScheme,
        univariate_hiding_kzg::{self, CommitmentRandomness},
    },
    Scalar,
};
use aptos_crypto::{
    arkworks::{
        random::{sample_field_element, sample_field_elements},
        srs::SrsType,
        GroupGenerators,
    },
    utils::powers,
};
use ark_ec::{pairing::Pairing, AffineRepr, CurveGroup, VariableBaseMSM};
use ark_ff::batch_inversion;
use ark_poly::{
    evaluations::multivariate::multilinear::DenseMultilinearExtension,
    polynomial::univariate::DensePolynomial as UniPoly, DenseUVPolynomial, MultilinearExtension,
    Polynomial,
};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::{One, Zero};
use core::fmt::Debug;
use itertools::izip;
use rand::thread_rng;
use rand_core::{CryptoRng, RngCore};
use rayon::prelude::*;
use std::{iter, marker::PhantomData};

#[derive(Clone, Debug, CanonicalSerialize, CanonicalDeserialize)]
pub struct ZeromorphProverKey<P: Pairing> {
    pub commit_pp: univariate_hiding_kzg::CommitmentKey<P>,
    pub open_pp: univariate_hiding_kzg::CommitmentKey<P>, // get rid of this?
}

#[allow(non_snake_case)]
#[derive(Copy, Clone, Debug, CanonicalSerialize, CanonicalDeserialize)]
pub struct ZeromorphVerifierKey<P: Pairing> {
    pub kzg_vk: univariate_hiding_kzg::VerificationKey<P>,
    pub tau_N_max_sub_2_N: P::G2Affine,
}

#[derive(Debug, PartialEq, CanonicalSerialize, CanonicalDeserialize, Clone)]
pub struct ZeromorphCommitment<P: Pairing>(P::G1);

impl<P: Pairing> Default for ZeromorphCommitment<P> {
    fn default() -> Self {
        Self(P::G1::zero())
    }
}

#[derive(Clone, CanonicalSerialize, CanonicalDeserialize, Debug)]
pub struct ZeromorphProof<P: Pairing> {
    pub pi: univariate_hiding_kzg::OpeningProof<P>,
    pub q_hat_com: univariate_hiding_kzg::Commitment<P>, // KZG commitment to the batched, lifted-degree poly constructed out of the q_k
    pub q_k_com: Vec<univariate_hiding_kzg::Commitment<P>>, // Vec<P::G1>, // vector of KZG commitments for the q_k
}

/// Computes the multilinear quotient polynomials for a given polynomial and evaluation point.
///
/// Given a multilinear polynomial `poly` over `n` variables and a point `point = [x_0, ..., x_{n-1}]`,
/// this function returns `(quotients, eval)`, where:
///
/// - `quotients` is a vector of univariate polynomials `[q_0, q_1, ..., q_{n-1}]`, each representing
///   the quotient along one variable such that:
///
///     poly(X) - poly(point) = sum_{k=0}^{n-1} (X_k - point_k) * q_k(X_0, ..., X_{k-1})
///
/// - `eval` is the polynomial evaluated at the point, i.e., `poly(point)`.
fn compute_multilinear_quotients<P: Pairing>(
    poly: &DenseMultilinearExtension<P::ScalarField>,
    point: &[P::ScalarField],
) -> (Vec<UniPoly<P::ScalarField>>, P::ScalarField) {
    let num_vars = poly.num_vars;
    assert_eq!(num_vars, point.len());

    let mut remainder = poly.to_evaluations();
    let mut quotients: Vec<_> = point
        .iter()
        .enumerate()
        .map(|(i, x_i)| {
            let (remainder_lo, remainder_hi) = remainder.split_at_mut(1 << (num_vars - 1 - i));
            let mut quotient = vec![P::ScalarField::zero(); remainder_lo.len()];

            quotient
                .par_iter_mut()
                .zip(&*remainder_lo)
                .zip(&*remainder_hi)
                .for_each(|((q, r_lo), r_hi)| {
                    *q = *r_hi - *r_lo;
                });

            remainder_lo
                .par_iter_mut()
                .zip(remainder_hi)
                .for_each(|(r_lo, r_hi)| {
                    *r_lo += (*r_hi - *r_lo) * *x_i;
                });

            remainder.truncate(1 << (num_vars - 1 - i));

            UniPoly::from_coefficients_vec(quotient)
        })
        .collect();
    quotients.reverse();
    (quotients, remainder[0])
}

/// Compute the batched, lifted-degree quotient `\hat{q}`
///
/// Example:
/// num_vars = 3
/// N = 1 << num_vars = 8
///
/// q_hat has 8 coefficients:
/// indices:   0 1 2 3 4 5 6 7
/// q_hat:    [0 0 0 0 0 0 0 0]
///
/// q0 = [a]
/// q1 = [b0, b1]
/// q2 = [c0, c1, c2, c3]
///
/// indices:   0 1 2 3 4        5        6               7
/// q_hat:    [0 0 0 0 y²*c0  y²*c1  y*b0 + y²*c2  a + y*b1 + y²*c3]
fn compute_batched_lifted_degree_quotient<P: Pairing>(
    quotients: &[UniPoly<P::ScalarField>],
    y_challenge: &P::ScalarField,
) -> (UniPoly<P::ScalarField>, usize) {
    let num_vars = quotients.len();

    // Compute \hat{q} = \sum_k y^k * X^{N - d_k - 1} * q_k
    let mut scalar = P::ScalarField::one(); // y^k

    // Rather than explicitly computing the shifts of q_k by N - d_k - 1 (i.e. multiplying q_k by X^{N - d_k - 1})
    // then accumulating them, we simply accumulate y^k*q_k into \hat{q} at the index offset N - d_k - 1
    let q_hat = quotients.iter().enumerate().fold(
        vec![P::ScalarField::zero(); 1 << num_vars], // the coefficient vector
        |mut q_hat, (k, q)| {
            let q_hat_iter = q_hat[(1 << num_vars) - (1 << k)..].par_iter_mut();
            q_hat_iter.zip(&q.coeffs).for_each(|(q_hat, q)| {
                *q_hat += scalar * *q;
            });
            scalar *= *y_challenge;
            q_hat
        },
    );

    (UniPoly::from_coefficients_vec(q_hat), 1 << (num_vars - 1))
}

fn eval_and_quotient_scalars<P: Pairing>(
    y_challenge: P::ScalarField,
    x_challenge: P::ScalarField,
    z_challenge: P::ScalarField,
    challenges: &[P::ScalarField],
) -> (P::ScalarField, (Vec<P::ScalarField>, Vec<P::ScalarField>)) {
    let num_vars = challenges.len();

    // squares of x = [x, x^2, .. x^{2^k}, .. x^{2^num_vars}]
    let squares_of_x: Vec<_> = std::iter::successors(Some(x_challenge), |&x| Some(x * x))
        .take(num_vars + 1)
        .collect();

    //    - These are cumulative products of powers of `x` in reverse order:
    //      ```text
    //      offsets_of_x[k] = Π_{j=k+1}^{n-1} x^{2^j}
    //      ```
    //    - Example: let `num_vars = 3` and `x_challenge = x`. Then
    //      ```text
    //      squares_of_x = [x, x^2, x^4, x^8]
    //      offsets_of_x = [x^7, x^6, x^4]
    let offsets_of_x = {
        let mut offsets_of_x = squares_of_x
            .iter()
            .rev()
            .skip(1)
            .scan(P::ScalarField::one(), |acc, pow_x| {
                *acc *= *pow_x;
                Some(*acc)
            })
            .collect::<Vec<_>>();
        offsets_of_x.reverse();
        offsets_of_x
    };

    // vs[i] = (x^{2^n} - 1)/(x^{2^i} - 1)
    let vs = {
        let v_numer = squares_of_x[num_vars] - P::ScalarField::one();
        let mut v_denoms = squares_of_x
            .iter()
            .map(|squares_of_x| *squares_of_x - P::ScalarField::one())
            .collect::<Vec<_>>();
        batch_inversion(&mut v_denoms);
        v_denoms
            .iter()
            .map(|v_denom| v_numer * *v_denom)
            .collect::<Vec<_>>()
    };

    let q_scalars = izip!(
        iter::successors(Some(P::ScalarField::one()), |acc| Some(*acc * y_challenge))
            .take(num_vars),
        offsets_of_x,
        squares_of_x,
        &vs,
        &vs[1..],
        challenges.iter().rev()
    )
    .map(|(power_of_y, offset_of_x, square_of_x, v_i, v_j, u_i)| {
        (
            -(power_of_y * offset_of_x),
            -(z_challenge * (square_of_x * *v_j - *u_i * *v_i)),
        )
    })
    .unzip();
    // -vs[0] * z = -z * (x^(2^num_vars) - 1) / (x - 1) = -z Φ_n(x)
    (-vs[0] * z_challenge, q_scalars)
}

#[derive(Clone)]
pub struct Zeromorph<P: Pairing> {
    _phantom: PhantomData<P>,
}

impl<P> Zeromorph<P>
where
    P: Pairing,
{
    pub fn protocol_name() -> &'static [u8] {
        b"Zeromorph"
    }

    // Commits to the evaluations on the hypercube
    pub fn commit<R: RngCore + CryptoRng>(
        pp: &ZeromorphProverKey<P>,
        poly: &DenseMultilinearExtension<P::ScalarField>,
        rng: &mut R,
    ) -> (
        ZeromorphCommitment<P>,
        univariate_hiding_kzg::CommitmentRandomness<P::ScalarField>,
    ) {
        // TODO: PUT THIS BACK IN
        // if pp.commit_pp.g1_powers().len() < poly.len() {
        //     return Err(ProofVerifyError::KeyLengthError(
        //         pp.commit_pp.g1_powers().len(),
        //         poly.len(),
        //     ));
        // }
        let r = Scalar(sample_field_element(rng));
        (
            ZeromorphCommitment(
                univariate_hiding_kzg::commit_with_randomness(
                    &pp.commit_pp,
                    &poly.to_evaluations(),
                    &r,
                )
                .0,
            ),
            r,
        )
    }

    pub fn open<R: RngCore + CryptoRng>(
        pp: &ZeromorphProverKey<P>,
        poly: &DenseMultilinearExtension<P::ScalarField>,
        point: &[P::ScalarField],
        eval: P::ScalarField, // Can be calculated
        s: CommitmentRandomness<P::ScalarField>,
        rng: &mut R,
        transcript: &mut merlin::Transcript,
    ) -> ZeromorphProof<P> {
        transcript.append_sep(Self::protocol_name());

        // TODO: PUT THIS BACK IN
        // if pp.commit_pp.msm_basis.len() < poly.len() {
        //     return Err(ProofVerifyError::KeyLengthError(
        //         pp.commit_pp.g1_powers().len(),
        //         poly.len(),
        //     ));
        // }

        // assert_eq!(poly.evaluate(point), *eval);

        let (quotients, _): (Vec<UniPoly<P::ScalarField>>, P::ScalarField) =
            compute_multilinear_quotients::<P>(poly, point);
        assert_eq!(quotients.len(), poly.num_vars);
        // assert_eq!(remainder, *eval); TODO: put back in?

        // Step 1: commit to all of the q_k
        let rs: Vec<Scalar<P::ScalarField>> =
            sample_field_elements::<P::ScalarField, _>(quotients.len(), rng)
                .into_iter()
                .map(Scalar)
                .collect();
        //let r = Scalar(sample_field_element::<P::ScalarField>(rng));
        let q_k_com: Vec<univariate_hiding_kzg::Commitment<P>> = quotients
            .iter()
            .zip(rs.iter())
            .map(|(quotient, r)| {
                univariate_hiding_kzg::commit_with_randomness(&pp.commit_pp, &quotient.coeffs, r)
            })
            .collect();

        // Step 2: verifier challenge to aggregate degree bound proofs
        q_k_com.iter().for_each(|c| transcript.append_point(&c.0));
        let y_challenge: P::ScalarField = transcript.challenge_scalar();

        // Step 3: Aggregate shifted q_k into \hat{q} and compute commitment

        // Compute the batched, lifted-degree quotient `\hat{q}`
        // qq_hat = ∑_{i=0}^{num_vars-1} y^i * X^(2^num_vars - d_k - 1) * q_i(x)
        let (q_hat, offset) = compute_batched_lifted_degree_quotient::<P>(&quotients, &y_challenge);

        // Compute and absorb the commitment C_q = [\hat{q}]
        let r = Scalar(sample_field_element::<P::ScalarField, _>(rng));
        let q_hat_com = univariate_hiding_kzg::commit_with_randomness_and_offset(
            &pp.commit_pp,
            &q_hat,
            &r,
            offset,
        );
        transcript.append_point(&q_hat_com.0);

        // Step 4/6: Obtain x challenge to evaluate the polynomial, and z challenge to aggregate two challenges
        let x_challenge = transcript.challenge_scalar();
        let z_challenge = transcript.challenge_scalar();

        // Step 5/7: Compute this batched poly

        // Compute batched degree and ZM-identity quotient polynomial pi
        let (eval_scalar, (degree_check_q_scalars, zmpoly_q_scalars)): (
            P::ScalarField,
            (Vec<P::ScalarField>, Vec<P::ScalarField>),
        ) = eval_and_quotient_scalars::<P>(y_challenge, x_challenge, z_challenge, point);
        // f = z * poly.Z + q_hat + (-z * Φ_n(x) * e) + ∑_k (q_scalars_k * q_k)   hmm why no sign for the q_hat????
        let mut f = UniPoly::from_coefficients_vec(poly.to_evaluations());
        f = f * z_challenge; // TODO: add MulAssign to arkworks so you can write f *= z_challenge?
        f += &q_hat;
        f[0] += eval_scalar * eval;
        quotients
            .into_iter()
            .zip(degree_check_q_scalars)
            .zip(zmpoly_q_scalars)
            .for_each(|((mut q, degree_check_scalar), zm_poly_scalar)| {
                q = q * (degree_check_scalar + zm_poly_scalar);
                f += &q;
            });
        //debug_assert_eq!(f.evaluate(&x_challenge), P::ScalarField::zero());

        // Compute and send proof commitment pi
        let rho = sample_field_element::<P::ScalarField, _>(rng);

        let pi = univariate_hiding_kzg::CommitmentHomomorphism::open(
            &pp.open_pp,
            f.coeffs,
            rho,
            x_challenge,
            P::ScalarField::zero(),
            &s,
        );

        ZeromorphProof {
            pi,
            q_hat_com,
            q_k_com,
        }
    }

    pub fn verify(
        vk: &ZeromorphVerifierKey<P>,
        comm: &ZeromorphCommitment<P>,
        point: &[P::ScalarField],
        eval: &P::ScalarField,
        proof: &ZeromorphProof<P>,
        transcript: &mut merlin::Transcript,
    ) -> anyhow::Result<()> {
        transcript.append_sep(Self::protocol_name());

        //let q_comms: Vec<P::G1> = proof.q_k_com.iter().map(|c| c.into_group()).collect();
        proof
            .q_k_com
            .iter()
            .for_each(|c| transcript.append_point(&c.0));

        // Challenge y
        let y_challenge: P::ScalarField = transcript.challenge_scalar();

        // Receive commitment C_q_hat
        transcript.append_point(&proof.q_hat_com.0);

        // Get x and z challenges
        let x_challenge = transcript.challenge_scalar();
        let z_challenge = transcript.challenge_scalar();

        // Compute batched degree and ZM-identity quotient polynomial pi
        let (eval_scalar, (mut q_scalars, zmpoly_q_scalars)): (
            P::ScalarField,
            (Vec<P::ScalarField>, Vec<P::ScalarField>),
        ) = eval_and_quotient_scalars::<P>(y_challenge, x_challenge, z_challenge, point);
        q_scalars
            .iter_mut()
            .zip(zmpoly_q_scalars)
            .for_each(|(scalar, zm_poly_q_scalar)| {
                *scalar += zm_poly_q_scalar;
            });
        let scalars = [
            vec![P::ScalarField::one(), z_challenge, eval_scalar * *eval],
            q_scalars,
        ]
        .concat();

        let mut bases_proj = Vec::with_capacity(3 + proof.q_k_com.len());

        bases_proj.push(proof.q_hat_com.0);
        bases_proj.push(comm.0);
        bases_proj.push(vk.kzg_vk.group_generators.g1.into_group()); // Not so ideal to include this in `normalize_batch` but the effect should be negligible
        bases_proj.extend(proof.q_k_com.iter().map(|w| w.0));

        let bases = P::G1::normalize_batch(&bases_proj);

        let zeta_z_com = <P::G1 as VariableBaseMSM>::msm(&bases, &scalars)
            .expect("MSM failed in ZeroMorph")
            .into_affine();

        // e(pi, [tau]_2 - x * [1]_2) == e(C_{\zeta,Z}, -[X^(N_max - 2^n - 1)]_2) <==> e(C_{\zeta,Z} - x * pi, [X^{N_max - 2^n - 1}]_2) * e(-pi, [tau_2]) == 1
        let pairing = P::multi_pairing(
            [
                zeta_z_com,
                proof.pi.pi_1.0.into_affine(),
                proof.pi.pi_2.into_affine(),
            ],
            [
                (-vk.tau_N_max_sub_2_N.into_group()).into_affine(),
                (vk.kzg_vk.tau_2.into_group() - (vk.kzg_vk.group_generators.g2 * x_challenge))
                    .into(),
                vk.kzg_vk.xi_2,
            ],
        );
        if !pairing.is_zero() {
            return Err(anyhow::anyhow!("Expected zero during multi-pairing check"));
        }

        Ok(())
    }
}

impl<P> PolynomialCommitmentScheme for Zeromorph<P>
where
    P: Pairing,
{
    type Commitment = ZeromorphCommitment<P>;
    type CommitmentKey = ZeromorphProverKey<P>;
    type Polynomial = DenseMultilinearExtension<P::ScalarField>;
    type Proof = ZeromorphProof<P>;
    type VerificationKey = ZeromorphVerifierKey<P>;
    type WitnessField = P::ScalarField;

    fn polynomial_from_vec(vec: Vec<Self::WitnessField>) -> Self::Polynomial {
        let len = vec.len();
        let next_pow2 = len.next_power_of_two();
        let mut vec2 = vec.clone();

        // Pad with zeros if needed
        if len < next_pow2 {
            vec2.resize(next_pow2, Self::WitnessField::zero());
        }

        let num_vars = next_pow2.ilog2() as usize;

        DenseMultilinearExtension::from_evaluations_vec(num_vars, vec2)
    }

    // TODO: use a batch_mul algorith, like in ZK Samaritan
    fn setup<R: RngCore + CryptoRng>(
        degree_bounds: Vec<usize>,
        rng: &mut R,
    ) -> (Self::CommitmentKey, Self::VerificationKey) {
        let number_of_coefficients = degree_bounds
            .iter()
            .map(|&x| x + 1)
            .product::<usize>()
            .next_power_of_two();

        let trapdoor = univariate_hiding_kzg::Trapdoor::<P>::rand(rng);
        let (kzg_vk_pp, kzg_commit_pp) = univariate_hiding_kzg::setup_extra(
            number_of_coefficients + 1,
            SrsType::PowersOfTau,
            GroupGenerators::default(),
            trapdoor,
        );
        //let open_pp = commit_pp;

        let prover_key = ZeromorphProverKey {
            commit_pp: kzg_commit_pp.clone(),
            open_pp: kzg_commit_pp,
        };

        // Derive verification key
        let vk = ZeromorphVerifierKey {
            kzg_vk: kzg_vk_pp.vk,
            tau_N_max_sub_2_N: kzg_vk_pp.g2_powers[number_of_coefficients],
        };

        (prover_key, vk)
    }

    fn commit(
        ck: &Self::CommitmentKey,
        poly: Self::Polynomial,
        _r: Option<Self::WitnessField>,
    ) -> Self::Commitment {
        let mut rng = thread_rng();
        Zeromorph::commit(&ck, &poly, &mut rng).0
    }

    fn open<R: RngCore + CryptoRng>(
        ck: &Self::CommitmentKey,
        poly: Self::Polynomial,
        challenge: Vec<Self::WitnessField>,
        r: Option<Self::WitnessField>,
        rng: &mut R,
        trs: &mut merlin::Transcript,
    ) -> Self::Proof {
        let s = Scalar(r.expect("open(): expected randomness r, got None"));

        let eval = Self::evaluate_point(&poly, &challenge);
        Zeromorph::open(&ck, &poly, &challenge, eval, s, rng, trs)
    }

    // TODO: also implement this in dekart_univariate_v2... hmm or defer to hiding KZG?
    fn batch_open<R: RngCore + CryptoRng>(
        ck: Self::CommitmentKey,
        polys: Vec<Self::Polynomial>,
        //   coms: Vec<Commitment>,
        challenge: Vec<Self::WitnessField>,
        rs: Option<Vec<Self::WitnessField>>,
        rng: &mut R,
        trs: &mut merlin::Transcript,
    ) -> Self::Proof {
        let rs = rs.expect("rs must be present");

        let gamma = trs.challenge_scalar();
        let gammas = powers(gamma, polys.len());

        let combined_poly = polys
            .iter()
            .zip(gammas.iter())
            .fold(Self::Polynomial::zero(), |acc, (poly, gamma_i)| {
                acc + poly * gamma_i
            });
        let eval = Self::evaluate_point(&combined_poly, &challenge);

        let s = rs
            .iter()
            .zip(gammas.iter())
            .fold(Self::WitnessField::zero(), |acc, (r, gamma_i)| {
                acc + (*r * gamma_i)
            });

        Zeromorph::open(&ck, &combined_poly, &challenge, eval, Scalar(s), rng, trs)
    }

    fn verify(
        vk: &Self::VerificationKey,
        com: Self::Commitment,
        challenge: Vec<Self::WitnessField>,
        eval: Self::WitnessField,
        proof: Self::Proof,
        trs: &mut merlin::Transcript,
    ) -> anyhow::Result<()> {
        Zeromorph::verify(&vk, &com, &challenge, &eval, &proof, trs)
    }

    fn random_witness<R: RngCore + CryptoRng>(rng: &mut R) -> Self::WitnessField {
        sample_field_element(rng)
    }

    fn evaluate_point(
        poly: &Self::Polynomial,
        point: &Vec<Self::WitnessField>,
    ) -> Self::WitnessField {
        poly.evaluate(point)
    }

    fn scheme_name() -> &'static [u8] {
        b"Zeromorph"
    }
}
