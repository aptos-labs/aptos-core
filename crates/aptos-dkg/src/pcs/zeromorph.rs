// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// A lot of this code is copy-pasted from `jolt-core`. TODO: benchmark them against each other

// WARNING: THIS CODE HAS NOT BEEN PROPERLY VETTED, ONLY USE FOR BENCHMARKING PURPOSES!!!!!

use crate::{
    fiat_shamir::PolynomialCommitmentScheme as _,
    pcs::{
        traits::PolynomialCommitmentScheme,
        univariate_hiding_kzg::{self, CommitmentRandomness},
    },
    sigma_protocol::homomorphism::TrivialShape,
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
use rand_core::{CryptoRng, RngCore};
use rayon::prelude::*;
use std::{iter, marker::PhantomData};

#[derive(Clone, Debug, CanonicalSerialize, CanonicalDeserialize)]
pub struct ZeromorphProverKey<P: Pairing> {
    pub hiding_kzg_pp: univariate_hiding_kzg::CommitmentKey<P>,
    /// Offset for opening the batched polynomial f (original `jolt-core` code
    /// has a copy of the pp called `open_pp` but with basis at offset).
    pub open_offset: usize,
}

#[allow(non_snake_case)]
#[derive(Copy, Clone, Debug, CanonicalSerialize, CanonicalDeserialize)]
pub struct ZeromorphVerifierKey<P: Pairing> {
    pub hkzg_vk: univariate_hiding_kzg::VerificationKey<P>,
    pub tau_N_max_sub_2_N: P::G2Affine,
}

#[derive(Debug, PartialEq, CanonicalSerialize, CanonicalDeserialize, Clone)]
pub struct ZeromorphCommitment<P: Pairing>(P::G1);

impl<P: Pairing> ZeromorphCommitment<P> {
    /// Reference to the inner G1 element (e.g. for combining in batch verify).
    pub fn as_inner(&self) -> &P::G1 {
        &self.0
    }

    /// Build a commitment from a G1 element (e.g. combined commitment in batch verify).
    pub fn from_g1(g: P::G1) -> Self {
        Self(g)
    }
}

impl<P: Pairing> Default for ZeromorphCommitment<P> {
    fn default() -> Self {
        Self(P::G1::zero())
    }
}

/// Verifier input type; same as commitment for Zeromorph (no MSM merging).
#[derive(Clone, Debug, PartialEq, CanonicalSerialize, CanonicalDeserialize)]
pub struct ZeromorphVerifierCommitment<P: Pairing>(pub ZeromorphCommitment<P>);

impl<P: Pairing> From<ZeromorphCommitment<P>> for ZeromorphVerifierCommitment<P> {
    fn from(c: ZeromorphCommitment<P>) -> Self {
        Self(c)
    }
}

#[derive(Clone, CanonicalSerialize, CanonicalDeserialize, Debug)]
pub struct ZeromorphProof<P: Pairing> {
    pub pi: univariate_hiding_kzg::OpeningProof<P>,
    pub q_hat_com: univariate_hiding_kzg::Commitment<P>, // KZG commitment to the batched, lifted-degree poly constructed out of the q_k
    pub q_k_com: Vec<univariate_hiding_kzg::Commitment<P>>, // has type Vec<P::G1>, this is a vector of KZG commitments for the q_k
}

/// Batched instance produced by Zeromorph opening: the univariate polynomial `f`, opening point,
/// claimed value, and randomness. Can be opened with `open_batched_instance_with_hkzg`, or combined
/// with other instances before a single univariate KZG open.
#[derive(Clone, Debug)]
pub struct ZeromorphBatchedOpeningInstance<P: Pairing> {
    pub f_coeffs: Vec<P::ScalarField>,
    pub rho: P::ScalarField,
    pub x: P::ScalarField,
    pub y: P::ScalarField,
    pub s: CommitmentRandomness<P::ScalarField>,
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

    // The algorithm processes variables from MSB to LSB (i=0 processes first variable with largest split).
    // DenseMultilinearExtension::to_evaluations() returns evaluations with variable 0 as LSB.
    // The original DensePolynomial.Z likely had variable 0 as MSB.
    // We need to reverse the point to match the variable processing order (MSB to LSB).
    let mut remainder = poly.to_evaluations();

    // Reverse the point so that point[0] corresponds to the MSB (last variable in original order)
    let mut point_reversed = point.to_vec();
    point_reversed.reverse();

    let mut quotients: Vec<_> = point_reversed
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
    pub fn commit(
        pp: &ZeromorphProverKey<P>,
        poly: &DenseMultilinearExtension<P::ScalarField>,
        r: P::ScalarField,
    ) -> ZeromorphCommitment<P> {
        // TODO: PUT THIS BACK IN
        // if pp.commit_pp.g1_powers().len() < poly.len() {
        //     return Err(ProofVerifyError::KeyLengthError(
        //         pp.commit_pp.g1_powers().len(),
        //         poly.len(),
        //     ));
        // }
        ZeromorphCommitment(
            univariate_hiding_kzg::commit_with_randomness(
                &pp.hiding_kzg_pp,
                &poly.to_evaluations(),
                &Scalar(r),
            )
            .0,
        )
    }

    /// Produces the batched opening instance (and commitments) that would be opened by univariate
    /// hiding KZG. Call `open_batched_instance_with_hkzg` to get the opening proof, or batch this
    /// instance with others first.
    pub fn open_to_batched_instance<R: RngCore + CryptoRng>(
        pp: &ZeromorphProverKey<P>,
        poly: &DenseMultilinearExtension<P::ScalarField>,
        point: &[P::ScalarField],
        eval: P::ScalarField,
        s: CommitmentRandomness<P::ScalarField>,
        rng: &mut R,
        trs: &mut merlin::Transcript,
    ) -> (
        ZeromorphBatchedOpeningInstance<P>,
        univariate_hiding_kzg::Commitment<P>,
        Vec<univariate_hiding_kzg::Commitment<P>>,
    ) {
        trs.append_sep(Self::protocol_name());

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
            Scalar::vec_from_inner(sample_field_elements::<P::ScalarField, _>(
                quotients.len(),
                rng,
            ));

        //let r = Scalar(sample_field_element::<P::ScalarField>(rng));
        let q_k_com: Vec<univariate_hiding_kzg::Commitment<P>> = quotients
            .iter()
            .zip(rs.iter())
            .map(|(quotient, r)| {
                univariate_hiding_kzg::commit_with_randomness(
                    &pp.hiding_kzg_pp,
                    &quotient.coeffs,
                    r,
                )
            })
            .collect();

        // Step 2: verifier challenge to aggregate degree bound proofs
        q_k_com
            .iter()
            .for_each(|c| trs.append_point(&c.0.into_affine()));
        let y_challenge: P::ScalarField = trs.challenge_scalar();

        // Step 3: Aggregate shifted q_k into \hat{q} and compute commitment

        // Compute the batched, lifted-degree quotient `\hat{q}`
        // qq_hat = ∑_{i=0}^{num_vars-1} y^i * X^(2^num_vars - d_k - 1) * q_i(x)
        let (q_hat, offset) = compute_batched_lifted_degree_quotient::<P>(&quotients, &y_challenge);

        // Compute and absorb the commitment C_q = [\hat{q}]
        let r = Scalar(sample_field_element::<P::ScalarField, _>(rng));
        let q_hat_com = univariate_hiding_kzg::commit_with_randomness_and_offset(
            &pp.hiding_kzg_pp,
            &q_hat.coeffs,
            &r,
            offset,
        );
        trs.append_point(&q_hat_com.0.into_affine());

        // Step 4/6: Obtain x challenge to evaluate the polynomial, and z challenge to aggregate two challenges
        let x_challenge = trs.challenge_scalar();
        let z_challenge = trs.challenge_scalar();

        // Step 5/7: Compute this batched poly

        // eval_and_quotient_scalars uses challenges.iter().rev(), so with challenges = point we get
        // q_scalars[k] for variable n-1-k. Pass point in reversed order so we get q_scalars[k] for variable k, matching quotients[k].
        let point_reversed_for_scalars: Vec<P::ScalarField> = point.iter().rev().cloned().collect();
        let (eval_scalar, (degree_check_q_scalars, zmpoly_q_scalars)): (
            P::ScalarField,
            (Vec<P::ScalarField>, Vec<P::ScalarField>),
        ) = eval_and_quotient_scalars::<P>(
            y_challenge,
            x_challenge,
            z_challenge,
            &point_reversed_for_scalars,
        );
        // f = z * poly.Z + q_hat + (-z * Φ_n(x) * e) + ∑_k (q_scalars_k * q_k)
        let mut f = UniPoly::from_coefficients_vec(poly.to_evaluations());
        f = f * z_challenge;
        f += &q_hat;
        f[0] += eval_scalar * eval;
        let q_scalars_for_s: Vec<P::ScalarField> = degree_check_q_scalars
            .iter()
            .zip(zmpoly_q_scalars.iter())
            .map(|(a, b)| *a + *b)
            .collect();
        let s_combined = r.0
            + z_challenge * s.0
            + q_scalars_for_s
                .iter()
                .zip(rs.iter())
                .map(|(scalar, rk)| *scalar * rk.0)
                .sum::<P::ScalarField>();

        quotients
            .into_iter()
            .zip(degree_check_q_scalars)
            .zip(zmpoly_q_scalars)
            .for_each(|((mut q, degree_check_scalar), zm_poly_scalar)| {
                q = q * (degree_check_scalar + zm_poly_scalar);
                f += &q;
            });
        debug_assert_eq!(
            f.evaluate(&x_challenge),
            P::ScalarField::zero(),
            "batched polynomial f must vanish at x_challenge"
        );

        let s_combined_scalar = Scalar(s_combined);
        let batched_instance = ZeromorphBatchedOpeningInstance {
            f_coeffs: f.coeffs,
            rho: s_combined,
            x: x_challenge,
            y: P::ScalarField::zero(),
            s: s_combined_scalar,
        };

        (batched_instance, q_hat_com, q_k_com)
    }

    /// Run univariate hiding KZG open on a batched instance (e.g. from `open_to_batched_instance`).
    /// Use this to complete a single Zeromorph open or to open after batching with more instances.
    pub fn open_batched_instance_with_hkzg(
        pp: &ZeromorphProverKey<P>,
        instance: &ZeromorphBatchedOpeningInstance<P>,
    ) -> univariate_hiding_kzg::OpeningProof<P> {
        univariate_hiding_kzg::CommitmentHomomorphism::open(
            &pp.hiding_kzg_pp,
            instance.f_coeffs.clone(),
            instance.rho,
            instance.x,
            instance.y,
            &instance.s,
            pp.open_offset,
        )
    }

    pub fn open<R: RngCore + CryptoRng>(
        pp: &ZeromorphProverKey<P>,
        poly: &DenseMultilinearExtension<P::ScalarField>,
        point: &[P::ScalarField],
        eval: P::ScalarField, // Can be calculated
        s: CommitmentRandomness<P::ScalarField>,
        rng: &mut R,
        trs: &mut merlin::Transcript,
    ) -> ZeromorphProof<P> {
        let (batched_instance, q_hat_com, q_k_com) =
            Self::open_to_batched_instance(pp, poly, point, eval, s, rng, trs);
        let pi = Self::open_batched_instance_with_hkzg(pp, &batched_instance);
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
        trs: &mut merlin::Transcript,
        batch: bool,
    ) -> anyhow::Result<()> {
        // Use the caller's transcript so verification is bound to the same protocol context as the
        // prover; otherwise proofs could be replayed across contexts sharing the same DST.
        if batch {
            let _gamma: P::ScalarField = trs.challenge_scalar(); // consume gamma so state matches batch_open
        }
        trs.append_sep(Self::protocol_name());
        proof
            .q_k_com
            .iter()
            .for_each(|c| trs.append_point(&c.0.into_affine()));
        let y_challenge: P::ScalarField = trs.challenge_scalar();
        trs.append_point(&proof.q_hat_com.0.into_affine());
        let x_challenge = trs.challenge_scalar();
        let z_challenge = trs.challenge_scalar();

        // Must match prover: use point in reversed order so q_scalars[k] aligns with quotients[k].
        let point_reversed_for_scalars: Vec<P::ScalarField> = point.iter().rev().cloned().collect();
        let (eval_scalar, (mut q_scalars, zmpoly_q_scalars)): (
            P::ScalarField,
            (Vec<P::ScalarField>, Vec<P::ScalarField>),
        ) = eval_and_quotient_scalars::<P>(
            y_challenge,
            x_challenge,
            z_challenge,
            &point_reversed_for_scalars,
        );
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
        bases_proj.push(vk.hkzg_vk.group_generators.g1.into_group()); // Not so ideal to include this in `normalize_batch` but the effect should be negligible
        bases_proj.extend(proof.q_k_com.iter().map(|w| w.0));

        let bases = P::G1::normalize_batch(&bases_proj);

        let zeta_z_com = <P::G1 as VariableBaseMSM>::msm(&bases, &scalars)
            .expect("MSM failed in ZeroMorph")
            .into_affine();

        // Delegate to standard hiding KZG verify: e(C - y*[1]_1, [1]_2) + e(-pi_1, [τ-x]_2) + e(-pi_2, ξ_2) = 0.
        // Commitment type is TrivialShape<P::G1>; y = 0 since the batched polynomial vanishes at x.
        let zeta_z_commitment = TrivialShape(zeta_z_com.into_group());
        univariate_hiding_kzg::CommitmentHomomorphism::verify(
            vk.hkzg_vk,
            zeta_z_commitment,
            x_challenge,
            P::ScalarField::zero(),
            proof.pi.clone(),
        )?;

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
    type VerifierCommitment = ZeromorphVerifierCommitment<P>;
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
        // setup_extra requires m to be a power of 2. number_of_coefficients is already a power of 2.
        // Use m = N so that offset = 0 and tau_N_max_sub_2_N = g2_powers[0] = [1]_2, giving the
        // standard hiding KZG check: e(zeta_z_com, [1]_2) = e(pi_1, [τ-x]_2) * e(pi_2, xi_2).
        let m = number_of_coefficients;
        let (hkzg_vk_pp, hkzg_commit_pp) = univariate_hiding_kzg::setup_extra(
            m,
            SrsType::PowersOfTau,
            GroupGenerators::default(),
            trapdoor,
        );

        let offset = m - number_of_coefficients; // 0 when m = N
        let prover_key = ZeromorphProverKey {
            hiding_kzg_pp: hkzg_commit_pp,
            open_offset: offset,
        };
        let vk = ZeromorphVerifierKey {
            hkzg_vk: hkzg_vk_pp.vk,
            tau_N_max_sub_2_N: hkzg_vk_pp.g2_powers[offset],
        };

        (prover_key, vk)
    }

    fn commit(
        ck: &Self::CommitmentKey,
        poly: Self::Polynomial,
        r: Option<Self::WitnessField>,
    ) -> Self::Commitment {
        let r = r.expect("Should not be empty");
        Zeromorph::commit(&ck, &poly, r)
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
        com: impl Into<Self::VerifierCommitment>,
        challenge: Vec<Self::WitnessField>,
        eval: Self::WitnessField,
        proof: Self::Proof,
        trs: &mut merlin::Transcript,
        batch: bool,
    ) -> anyhow::Result<()> {
        let com = com.into();
        Zeromorph::verify(&vk, &com.0, &challenge, &eval, &proof, trs, batch)
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

    fn default_num_point_dims_for_tests() -> u32 {
        4
    }

    /// Multilinear: degree 1 per variable → `[1; n]` gives 2^n coefficients.
    fn degree_bounds_for_test_point_dims(num_point_dims: u32) -> Vec<usize> {
        vec![1; num_point_dims as usize]
    }
}

#[allow(non_snake_case)]
#[cfg(test)]
mod test {
    use super::*;
    use ark_bn254::{Bn254, Fr};
    use ark_ff::{BigInt, Field, Zero};
    use ark_std::{test_rng, UniformRand};
    use rand::{rngs::StdRng, SeedableRng};

    // Evaluate Phi_k(x) = \sum_{i=0}^k x^i using the direct inefficient formula
    fn phi<P: Pairing>(challenge: &P::ScalarField, subscript: usize) -> P::ScalarField {
        let len = (1 << subscript) as u64;
        (0..len).fold(P::ScalarField::zero(), |mut acc, i| {
            //Note this is ridiculous DevX
            acc += challenge.pow(BigInt::<1>::from(i));
            acc
        })
    }

    /// Test for computing qk given multilinear f
    /// Given 𝑓(𝑋₀, …, 𝑋ₙ₋₁), and `(𝑢, 𝑣)` such that \f(\u) = \v, compute `qₖ(𝑋₀, …, 𝑋ₖ₋₁)`
    /// such that the following identity holds:
    ///
    /// `𝑓(𝑋₀, …, 𝑋ₙ₋₁) − 𝑣 = ∑ₖ₌₀ⁿ⁻¹ (𝑋ₖ − 𝑢ₖ) qₖ(𝑋₀, …, 𝑋ₖ₋₁)`
    #[test]
    fn quotient_construction() {
        // Define size params
        let num_vars = 4;
        let n: u64 = 1 << num_vars;

        // Construct a random multilinear polynomial f, and (u,v) such that f(u) = v
        let mut rng = test_rng();
        let evals: Vec<_> = (0..n).map(|_| Fr::rand(&mut rng)).collect();
        // Use polynomial_from_vec which is the standard way to create polynomials in this codebase
        let multilinear_f = Zeromorph::<Bn254>::polynomial_from_vec(evals.clone());
        let u_challenge = (0..num_vars)
            .map(|_| Fr::rand(&mut rng))
            .collect::<Vec<_>>();
        let v_evaluation = multilinear_f.evaluate(&u_challenge);

        // Verify the polynomial was created correctly - check hypercube evaluations
        // The evaluations should be in order: f(0,0,...,0), f(1,0,...,0), f(0,1,...,0), ..., f(1,1,...,1)
        // For index i, the binary representation gives the hypercube point
        // Bit j (LSB = 0) corresponds to variable j
        let poly_evals = multilinear_f.to_evaluations();
        assert_eq!(poly_evals.len(), evals.len(), "Evaluation count mismatch");
        for (i, &expected_eval) in evals.iter().enumerate() {
            // Reconstruct the hypercube point for index i
            // LSB (bit 0) is variable 0, bit 1 is variable 1, etc.
            let mut point = Vec::new();
            for j in 0..num_vars {
                point.push(
                    if (i >> j) & 1 == 1 {
                        Fr::one()
                    } else {
                        Fr::zero()
                    },
                );
            }
            let computed_eval = multilinear_f.evaluate(&point);
            // Check both that evaluate() works and that to_evaluations() returns the right order
            if expected_eval != computed_eval || poly_evals[i] != expected_eval {
                panic!(
                    "Evaluation mismatch at index {}: expected={:?}, computed={:?}, to_evals={:?}, point={:?}",
                    i, expected_eval, computed_eval, poly_evals[i], point
                );
            }
        }

        // Compute multilinear quotients `qₖ(𝑋₀, …, 𝑋ₖ₋₁)`
        // The function now handles the variable ordering internally
        let (quotients, constant_term) =
            compute_multilinear_quotients::<Bn254>(&multilinear_f, &u_challenge);

        // The constant term should equal the evaluation at u_challenge
        assert_eq!(
            constant_term, v_evaluation,
            "The constant term equal to the evaluation of the polynomial at challenge point. \
             constant_term={:?}, v_evaluation={:?}",
            constant_term, v_evaluation
        );

        //To demonstrate that q_k was properly constructed we show that the identity holds at a random multilinear challenge
        // i.e. 𝑓(𝑧) − 𝑣 − ∑ₖ₌₀ᵈ⁻¹ (𝑧ₖ − 𝑢ₖ)𝑞ₖ(𝑧) = 0
        let z_challenge = (0..num_vars)
            .map(|_| Fr::rand(&mut rng))
            .collect::<Vec<_>>();

        let mut res = multilinear_f.evaluate(&z_challenge);
        res -= v_evaluation;

        // After the fix in compute_multilinear_quotients, quotients[k] corresponds to variable k
        // and depends on variables 0..k-1 (first k variables)
        for (k, q_k_uni) in quotients.iter().enumerate() {
            // q_k depends on the first k variables: X_0, ..., X_{k-1}
            let z_partial = if k > 0 {
                &z_challenge[0..k]
            } else {
                // k=0: q_0 is a constant, no variables to evaluate
                &[]
            };
            //This is a weird consequence of how things are done.. the univariate polys are of the multilinear commitment in lagrange basis. Therefore we evaluate as multilinear
            // q_k_uni has 2^k coefficients representing evaluations on a k-variable hypercube
            let q_k_ml = DenseMultilinearExtension::from_evaluations_vec(k, q_k_uni.coeffs.clone());
            let q_k_eval = if k > 0 {
                q_k_ml.evaluate(&z_partial.to_vec())
            } else {
                // k=0: q_0 is constant, just use the coefficient
                q_k_uni.coeffs[0]
            };

            // Multiply by (z_k - u_k) where k is the variable index
            res -= (z_challenge[k] - u_challenge[k]) * q_k_eval;
        }
        assert!(res.is_zero());
    }

    /// Test for construction of batched lifted degree quotient:
    ///  ̂q = ∑ₖ₌₀ⁿ⁻¹ yᵏ Xᵐ⁻ᵈᵏ⁻¹ ̂qₖ, 𝑑ₖ = deg(̂q), 𝑚 = 𝑁
    #[test]
    fn batched_lifted_degree_quotient() {
        let num_vars = 3;
        let n = 1 << num_vars;

        // Define mock qₖ with deg(qₖ) = 2ᵏ⁻¹
        let q_0 = UniPoly::from_coefficients_vec(vec![Fr::one()]);
        let q_1 = UniPoly::from_coefficients_vec(vec![Fr::from(2u64), Fr::from(3u64)]);
        let q_2 = UniPoly::from_coefficients_vec(vec![
            Fr::from(4u64),
            Fr::from(5u64),
            Fr::from(6u64),
            Fr::from(7u64),
        ]);
        let quotients = vec![q_0, q_1, q_2];

        let mut rng = test_rng();
        let y_challenge = Fr::rand(&mut rng);

        //Compute batched quptient  ̂q
        let (batched_quotient, _) =
            compute_batched_lifted_degree_quotient::<Bn254>(&quotients, &y_challenge);

        //Explicitly define q_k_lifted = X^{N-2^k} * q_k and compute the expected batched result
        let q_0_lifted = UniPoly::from_coefficients_vec(vec![
            Fr::zero(),
            Fr::zero(),
            Fr::zero(),
            Fr::zero(),
            Fr::zero(),
            Fr::zero(),
            Fr::zero(),
            Fr::one(),
        ]);
        let q_1_lifted = UniPoly::from_coefficients_vec(vec![
            Fr::zero(),
            Fr::zero(),
            Fr::zero(),
            Fr::zero(),
            Fr::zero(),
            Fr::zero(),
            Fr::from(2u64),
            Fr::from(3u64),
        ]);
        let q_2_lifted = UniPoly::from_coefficients_vec(vec![
            Fr::zero(),
            Fr::zero(),
            Fr::zero(),
            Fr::zero(),
            Fr::from(4u64),
            Fr::from(5u64),
            Fr::from(6u64),
            Fr::from(7u64),
        ]);

        //Explicitly compute  ̂q i.e. RLC of lifted polys
        let mut batched_quotient_expected = UniPoly::from_coefficients_vec(vec![Fr::zero(); n]);

        batched_quotient_expected += &q_0_lifted;
        batched_quotient_expected += &(q_1_lifted * y_challenge);
        batched_quotient_expected += &(q_2_lifted * (y_challenge * y_challenge));
        assert_eq!(batched_quotient, batched_quotient_expected);
    }

    /// evaluated quotient \zeta_x
    ///
    /// 𝜁 = 𝑓 − ∑ₖ₌₀ⁿ⁻¹𝑦ᵏ𝑥ʷˢ⁻ʷ⁺¹𝑓ₖ  = 𝑓 − ∑_{d ∈ {d₀, ..., dₙ₋₁}} X^{d* - d + 1}  − ∑{k∶ dₖ=d} yᵏ fₖ , where d* = lifted degree
    ///
    /// 𝜁 =  ̂q - ∑ₖ₌₀ⁿ⁻¹ yᵏ Xᵐ⁻ᵈᵏ⁻¹ ̂qₖ, m = N
    #[test]
    fn partially_evaluated_quotient_zeta() {
        let num_vars = 3;
        let n: u64 = 1 << num_vars;

        let mut rng = test_rng();
        let x_challenge = Fr::rand(&mut rng);
        let y_challenge = Fr::rand(&mut rng);

        let challenges: Vec<_> = (0..num_vars).map(|_| Fr::rand(&mut rng)).collect();
        let z_challenge = Fr::rand(&mut rng);

        let (_, (zeta_x_scalars, _)) =
            eval_and_quotient_scalars::<Bn254>(y_challenge, x_challenge, z_challenge, &challenges);

        // To verify we manually compute zeta using the computed powers and expected
        // 𝜁 =  ̂q - ∑ₖ₌₀ⁿ⁻¹ yᵏ Xᵐ⁻ᵈᵏ⁻¹ ̂qₖ, m = N
        assert_eq!(
            zeta_x_scalars[0],
            -x_challenge.pow(BigInt::<1>::from(n - 1))
        );

        assert_eq!(
            zeta_x_scalars[1],
            -y_challenge * x_challenge.pow(BigInt::<1>::from(n - 1 - 1))
        );

        assert_eq!(
            zeta_x_scalars[2],
            -y_challenge * y_challenge * x_challenge.pow(BigInt::<1>::from(n - 3 - 1))
        );
    }

    /// Test efficiently computing 𝛷ₖ(x) = ∑ᵢ₌₀ᵏ⁻¹xⁱ
    /// 𝛷ₖ(𝑥) = ∑ᵢ₌₀ᵏ⁻¹𝑥ⁱ = (𝑥²^ᵏ − 1) / (𝑥 − 1)
    #[test]
    fn phi_n_x_evaluation() {
        const N: u64 = 8u64;
        let log_N = (N as usize).ilog2() as usize;

        // 𝛷ₖ(𝑥)
        let mut rng = test_rng();
        let x_challenge = Fr::rand(&mut rng);

        let efficient = (x_challenge.pow(BigInt::<1>::from((1 << log_N) as u64)) - Fr::one())
            / (x_challenge - Fr::one());
        let expected: Fr = phi::<Bn254>(&x_challenge, log_N);
        assert_eq!(efficient, expected);
    }

    /// Test efficiently computing 𝛷ₖ(x) = ∑ᵢ₌₀ᵏ⁻¹xⁱ
    /// 𝛷ₙ₋ₖ₋₁(𝑥²^ᵏ⁺¹) = (𝑥²^ⁿ − 1) / (𝑥²^ᵏ⁺¹ − 1)
    #[test]
    fn phi_n_k_1_x_evaluation() {
        const N: u64 = 8u64;
        let log_N = (N as usize).ilog2() as usize;

        // 𝛷ₖ(𝑥)
        let mut rng = test_rng();
        let x_challenge = Fr::rand(&mut rng);
        let k = 2;

        //𝑥²^ᵏ⁺¹
        let x_pow = x_challenge.pow(BigInt::<1>::from((1 << (k + 1)) as u64));

        //(𝑥²^ⁿ − 1) / (𝑥²^ᵏ⁺¹ − 1)
        let efficient = (x_challenge.pow(BigInt::<1>::from((1 << log_N) as u64)) - Fr::one())
            / (x_pow - Fr::one());
        let expected: Fr = phi::<Bn254>(&x_challenge, log_N - k - 1);
        assert_eq!(efficient, expected);
    }

    /// Test construction of 𝑍ₓ
    /// 𝑍ₓ =  ̂𝑓 − 𝑣 ∑ₖ₌₀ⁿ⁻¹(𝑥²^ᵏ𝛷ₙ₋ₖ₋₁(𝑥ᵏ⁺¹)− 𝑢ₖ𝛷ₙ₋ₖ(𝑥²^ᵏ)) ̂qₖ
    #[test]
    fn partially_evaluated_quotient_z_x() {
        let num_vars = 3;

        // Construct a random multilinear polynomial f, and (u,v) such that f(u) = v.
        let mut rng = test_rng();
        let challenges: Vec<_> = (0..num_vars).map(|_| Fr::rand(&mut rng)).collect();

        let u_rev = {
            let mut res = challenges.clone();
            res.reverse();
            res
        };

        let x_challenge = Fr::rand(&mut rng);
        let y_challenge = Fr::rand(&mut rng);
        let z_challenge = Fr::rand(&mut rng);

        // Construct Z_x scalars
        let (_, (_, z_x_scalars)) =
            eval_and_quotient_scalars::<Bn254>(y_challenge, x_challenge, z_challenge, &challenges);

        for k in 0..num_vars {
            let x_pow_2k = x_challenge.pow(BigInt::<1>::from((1 << k) as u64)); // x^{2^k}
            let x_pow_2kp1 = x_challenge.pow(BigInt::<1>::from((1 << (k + 1)) as u64)); // x^{2^{k+1}}
                                                                                        // x^{2^k} * \Phi_{n-k-1}(x^{2^{k+1}}) - u_k *  \Phi_{n-k}(x^{2^k})
            let mut scalar = x_pow_2k * phi::<Bn254>(&x_pow_2kp1, num_vars - k - 1)
                - u_rev[k] * phi::<Bn254>(&x_pow_2k, num_vars - k);
            scalar *= z_challenge;
            scalar *= Fr::from(-1);
            assert_eq!(z_x_scalars[k], scalar);
        }
    }

    #[test]
    fn zeromorph_commit_prove_verify() {
        for num_vars in [4, 5, 6] {
            let mut rng = StdRng::seed_from_u64(num_vars as u64);
            let mut ark_rng = test_rng();

            let poly = DenseMultilinearExtension::rand(num_vars, &mut ark_rng);
            let point: Vec<<Bn254 as Pairing>::ScalarField> = (0..num_vars)
                .map(|_| <Bn254 as Pairing>::ScalarField::rand(&mut ark_rng))
                .collect();
            let eval = poly.evaluate(&point);

            // degree_bounds should be the degree, not the number of coefficients
            // For a multilinear polynomial with num_vars variables, the degree is (1 << num_vars) - 1
            let degree = (1 << num_vars) - 1;
            let (pk, vk) = Zeromorph::<Bn254>::setup(vec![degree], &mut rng);
            let r = <Bn254 as Pairing>::ScalarField::rand(&mut ark_rng);
            let commitment = Zeromorph::<Bn254>::commit(&pk, &poly, r);

            // Use the same DST as verify() when batch=false, so challenges match.
            let dst = <Zeromorph<Bn254> as crate::pcs::traits::PolynomialCommitmentScheme>::transcript_dst_for_single_open();
            let mut prover_transcript = merlin::Transcript::new(dst);
            let s = Scalar(r);
            let proof = Zeromorph::<Bn254>::open(
                &pk,
                &poly,
                &point,
                eval,
                s,
                &mut rng,
                &mut prover_transcript,
            );

            // Verify proof (verifier derives challenges from an internal transcript with the same DST).
            let mut verifier_transcript = merlin::Transcript::new(dst);
            Zeromorph::<Bn254>::verify(
                &vk,
                &commitment,
                &point,
                &eval,
                &proof,
                &mut verifier_transcript,
                false,
            )
            .unwrap();

            // evaluate bad proof for soundness
            let altered_verifier_point = point
                .iter()
                .map(|s| *s + <Bn254 as Pairing>::ScalarField::one())
                .collect::<Vec<_>>();
            let altered_verifier_eval = poly.evaluate(&altered_verifier_point);
            let mut verifier_transcript = merlin::Transcript::new(dst);
            assert!(Zeromorph::<Bn254>::verify(
                &vk,
                &commitment,
                &altered_verifier_point,
                &altered_verifier_eval,
                &proof,
                &mut verifier_transcript,
                false,
            )
            .is_err())
        }
    }
}
