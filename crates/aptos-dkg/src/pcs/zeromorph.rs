// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// A lot of this code is copy-pasted from `jolt-core`. TODO: benchmark them against each other

use crate::{
    fiat_shamir::PolynomialCommitmentScheme as _,
    pcs::{traits::PolynomialCommitmentScheme, univariate_hiding_kzg},
    Scalar,
};
use aptos_crypto::arkworks::{
    random::{sample_field_element, sample_field_elements},
    GroupGenerators,
};
use ark_ec::{pairing::Pairing, AffineRepr, CurveGroup, VariableBaseMSM};
use ark_ff::{batch_inversion, PrimeField};
use ark_poly::{
    evaluations::multivariate::multilinear::DenseMultilinearExtension,
    polynomial::univariate::DensePolynomial as UniPoly, DenseUVPolynomial, MultilinearExtension,
    Polynomial,
};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::{One, Zero};
use core::fmt::Debug;
use itertools::izip;
use rand::{rngs::StdRng, thread_rng};
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use rand_core::{CryptoRng, RngCore};
use rayon::prelude::*;
use std::{borrow::Borrow, iter, marker::PhantomData, sync::Arc};

// pub struct ZeromorphSRS<P: Pairing>(Arc<SRS<P>>);

// impl<P: Pairing> ZeromorphSRS<P> {
//     pub fn setup<R: RngCore + CryptoRng>(rng: &mut R, max_degree: usize) -> Self
// //    where
// //        P::ScalarField: JoltField,
//     {
//         Self(Arc::new(SRS::setup(rng, max_degree, max_degree)))
//     }

//     pub fn trim(self, max_degree: usize) -> (ZeromorphProverKey<P>, ZeromorphVerifierKey<P>) {
//         let (commit_pp, kzg_vk) = SRS::trim(self.0.clone(), max_degree);
//         let offset = self.0.g1_powers.len() - max_degree;
//         let tau_N_max_sub_2_N = self.0.g2_powers[offset];
//         let open_pp = KZGProverKey::new(self.0, offset, max_degree);
//         (
//             ZeromorphProverKey { commit_pp, open_pp },
//             ZeromorphVerifierKey {
//                 kzg_vk,
//                 tau_N_max_sub_2_N,
//             },
//         )
//     }
// }

#[derive(Clone, Debug, CanonicalSerialize, CanonicalDeserialize)]
pub struct ZeromorphProverKey<P: Pairing> {
    pub commit_pp: univariate_hiding_kzg::CommitmentKey<P>,
    pub open_pp: univariate_hiding_kzg::CommitmentKey<P>,
}

#[allow(non_snake_case)]
#[derive(Copy, Clone, Debug, CanonicalSerialize, CanonicalDeserialize)]
pub struct ZeromorphVerifierKey<P: Pairing> {
    pub kzg_vk: univariate_hiding_kzg::VerificationKey<P>,
    pub tau_N_max_sub_2_N: P::G2Affine,
}

// #[allow(non_snake_case)]
// impl<P: Pairing> From<&ZeromorphProverKey<P>> for ZeromorphVerifierKey<P> {
//     fn from(prover_key: &ZeromorphProverKey<P>) -> Self {
//         let kzg_vk = univariate_hiding_kzg::VerificationKey::from(&prover_key.commit_pp);
//         let max_degree = prover_key.commit_pp.supported_size - 1;
//         let offset = prover_key.commit_pp.srs.g1_powers.len() - max_degree;

//         let tau_N_max_sub_2_N = prover_key.commit_pp.srs.g2_powers[offset];
//         ZeromorphVerifierKey {
//             kzg_vk,
//             tau_N_max_sub_2_N,
//         }
//     }
// }

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
) -> (Vec<UniPoly<P::ScalarField>>, P::ScalarField)
// where
//     <P as Pairing>::ScalarField: JoltField,
{
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
) -> (UniPoly<P::ScalarField>, usize)
// where
//     <P as Pairing>::ScalarField: JoltField,
{
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
) -> (P::ScalarField, (Vec<P::ScalarField>, Vec<P::ScalarField>))
// where
//     <P as Pairing>::ScalarField: JoltField,
{
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
    //     <P as Pairing>::ScalarField: JoltField,
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
        eval: &P::ScalarField, // Can be calculated
        rng: &mut R,
        transcript: &mut merlin::Transcript,
    ) -> ZeromorphProof<P> {
        transcript.append_sep(Self::protocol_name());

        // let poly: &DenseMultilinearExtension<P::ScalarField> = poly.try_into().unwrap();

        // TODO: PUT THIS BACK IN
        // if pp.commit_pp.msm_basis.len() < poly.len() {
        //     return Err(ProofVerifyError::KeyLengthError(
        //         pp.commit_pp.g1_powers().len(),
        //         poly.len(),
        //     ));
        // }

        // assert_eq!(poly.evaluate(point), *eval);

        let (quotients, remainder): (Vec<UniPoly<P::ScalarField>>, P::ScalarField) =
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

        //let q_comms: Vec<P::G1> = q_k_com.par_iter().map(|c| c.into_group()).collect();
        // Compute the multilinear quotients q_k = q_k(X_0, ..., X_{k-1})
        // let quotient_slices: Vec<&[P::ScalarField]> =
        //     quotients.iter().map(|q| q.coeffs.as_slice()).collect();
        // let q_k_com = UnivariateKZG::commit_batch(&pp.commit_pp, &quotient_slices)?;
        // let q_comms: Vec<P::G1> = q_k_com.par_iter().map(|c| c.into_group()).collect();
        // let quotient_max_len = quotient_slices.iter().map(|s| s.len()).max().unwrap();

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
        f[0] += eval_scalar * *eval;
        quotients
            .into_iter()
            .zip(degree_check_q_scalars)
            .zip(zmpoly_q_scalars)
            .for_each(|((mut q, degree_check_scalar), zm_poly_scalar)| {
                q = q * (degree_check_scalar + zm_poly_scalar);
                f += &q;
            });
        //debug_assert_eq!(f.evaluate(&x_challenge), P::ScalarField::zero());

        // let commitment_hom= univariate_hiding_kzg::CommitmentHomomorphism {
        //     lagr_g1: &pp.open_pp.lagr_g1,
        //     xi_1: pp.open_pp.xi_1,
        // };
        // Compute and send proof commitment pi
        let rho = sample_field_element::<P::ScalarField, _>(rng);
        let s = Scalar(sample_field_element::<P::ScalarField, _>(rng));

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
            number_of_coefficients,
            univariate_hiding_kzg::BasisType::PowersOfTau,
            GroupGenerators::default(),
            trapdoor,
            rng,
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

    fn open(
        ck: &Self::CommitmentKey,
        poly: Self::Polynomial,
        challenge: Vec<Self::WitnessField>,
    ) -> Self::Proof {
        let mut rng = thread_rng();
        let mut transcript = merlin::Transcript::new(b"Zeromorph");
        let eval = Self::evaluate_point(&poly, &challenge);
        Zeromorph::open(&ck, &poly, &challenge, &eval, &mut rng, &mut transcript)
    }

    fn verify(
        vk: &Self::VerificationKey,
        com: Self::Commitment,
        challenge: Vec<Self::WitnessField>,
        eval: Self::WitnessField,
        proof: Self::Proof,
    ) -> anyhow::Result<()> {
        let mut transcript = merlin::Transcript::new(b"Zeromorph");
        Zeromorph::verify(&vk, &com, &challenge, &eval, &proof, &mut transcript)
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

// impl<P: Pairing> Zeromorph<P>
// //where
// //    <P as Pairing>::ScalarField
// {
//     type Field = P::ScalarField;
//     type ProverSetup = ZeromorphProverKey<P>;
//     type VerifierSetup = ZeromorphVerifierKey<P>;
//     type Commitment = ZeromorphCommitment<P>;
//     type Proof = ZeromorphProof<P>;
//     type BatchedProof = ZeromorphProof<P>;
//     type OpeningProofHint = ();

//     fn setup_prover(max_num_vars: usize) -> Self::ProverSetup
//     {
//         let max_len = 1 << max_num_vars;
//         ZeromorphSRS(Arc::new(SRS::setup(
//             &mut ChaCha20Rng::from_seed(*b"ZEROMORPH_POLY_COMMITMENT_SCHEME"),
//             max_len,
//             max_len,
//         )))
//         .trim(max_len)
//         .0
//     }

//     fn setup_verifier(setup: &Self::ProverSetup) -> Self::VerifierSetup {
//         ZeromorphVerifierKey::from(setup)
//     }

//     fn commit(
//         poly: &MultilinearPolynomial<Self::Field>,
//         setup: &Self::ProverSetup,
//     ) -> (Self::Commitment, Self::OpeningProofHint) {
//         assert!(
//             setup.commit_pp.g1_powers().len() > poly.len(),
//             "COMMIT KEY LENGTH ERROR {}, {}",
//             setup.commit_pp.g1_powers().len(),
//             poly.len()
//         );
//         let commitment = ZeromorphCommitment(
//             UnivariateKZG::commit_as_univariate(&setup.commit_pp, poly).unwrap(),
//         );
//         (commitment, ())
//     }

//     fn batch_commit<U>(polys: &[U], gens: &Self::ProverSetup) -> Vec<Self::Commitment>
//     where
//         U: Borrow<MultilinearPolynomial<Self::Field>> + Sync,
//     {
//         UnivariateKZG::commit_batch(&gens.commit_pp, polys)
//             .unwrap()
//             .into_iter()
//             .map(|c| ZeromorphCommitment(c))
//             .collect()
//     }

//     fn combine_commitments<C: Borrow<Self::Commitment>>(
//         commitments: &[C],
//         coeffs: &[Self::Field],
//     ) -> Self::Commitment {
//         let combined_commitment: P::G1 = commitments
//             .iter()
//             .zip(coeffs.iter())
//             .map(|(commitment, coeff)| commitment.borrow().0 * coeff)
//             .sum();
//         ZeromorphCommitment(combined_commitment.into_affine())
//     }

//     fn prove<ProofTranscript: Transcript>(
//         setup: &Self::ProverSetup,
//         poly: &MultilinearPolynomial<Self::Field>,
//         opening_point: &[Self::Field], // point at which the polynomial is evaluated
//         _: Self::OpeningProofHint,
//         transcript: &mut ProofTranscript,
//     ) -> Self::Proof {
//         let eval = poly.evaluate(opening_point);
//         Zeromorph::<P>::open(setup, poly, opening_point, &eval, transcript).unwrap()
//     }

//     fn verify<ProofTranscript: Transcript>(
//         proof: &Self::Proof,
//         setup: &Self::VerifierSetup,
//         transcript: &mut ProofTranscript,
//         opening_point: &[Self::Field], // point at which the polynomial is evaluated
//         opening: &Self::Field,         // evaluation \widetilde{Z}(r)
//         commitment: &Self::Commitment,
//     ) -> Result<(), ProofVerifyError> {
//         Zeromorph::<P>::verify(setup, commitment, opening_point, opening, proof, transcript)
//     }

//     fn protocol_name() -> &'static [u8] {
//         b"zeromorph"
//     }
// }

// #[cfg(test)]
// mod test {
//     use super::*;
//     use crate::transcripts::{Blake2bTranscript, Transcript};
//     use crate::utils::math::Math;
//     use ark_bn254::{Bn254, Fr};
//     use ark_ff::{BigInt, Field, Zero};
//     use ark_std::{test_rng, UniformRand};
//     use rand_core::SeedableRng;

//     // Evaluate Phi_k(x) = \sum_{i=0}^k x^i using the direct inefficient formula
//     fn phi<P: Pairing>(challenge: &P::ScalarField, subscript: usize) -> P::ScalarField {
//         let len = (1 << subscript) as u64;
//         (0..len).fold(P::ScalarField::zero(), |mut acc, i| {
//             //Note this is ridiculous DevX
//             acc += challenge.pow(BigInt::<1>::from(i));
//             acc
//         })
//     }

//     /// Test for computing qk given multilinear f
//     /// Given 𝑓(𝑋₀, …, 𝑋ₙ₋₁), and `(𝑢, 𝑣)` such that \f(\u) = \v, compute `qₖ(𝑋₀, …, 𝑋ₖ₋₁)`
//     /// such that the following identity holds:
//     ///
//     /// `𝑓(𝑋₀, …, 𝑋ₙ₋₁) − 𝑣 = ∑ₖ₌₀ⁿ⁻¹ (𝑋ₖ − 𝑢ₖ) qₖ(𝑋₀, …, 𝑋ₖ₋₁)`
//     #[test]
//     fn quotient_construction() {
//         // Define size params
//         let num_vars = 4;
//         let n: u64 = 1 << num_vars;

//         // Construct a random multilinear polynomial f, and (u,v) such that f(u) = v
//         let mut rng = test_rng();
//         let multilinear_f =
//             DenseMultilinearExtension::new((0..n).map(|_| Fr::rand(&mut rng)).collect::<Vec<_>>());
//         let u_challenge = (0..num_vars)
//             .map(|_| Fr::rand(&mut rng))
//             .collect::<Vec<_>>();
//         let v_evaluation = multilinear_f.evaluate(&u_challenge);

//         // Compute multilinear quotients `qₖ(𝑋₀, …, 𝑋ₖ₋₁)`
//         let (quotients, constant_term) =
//             compute_multilinear_quotients::<Bn254>(&multilinear_f, &u_challenge);

//         // Assert the constant term is equal to v_evaluation
//         assert_eq!(
//             constant_term, v_evaluation,
//             "The constant term equal to the evaluation of the polynomial at challenge point."
//         );

//         //To demonstrate that q_k was properly constructed we show that the identity holds at a random multilinear challenge
//         // i.e. 𝑓(𝑧) − 𝑣 − ∑ₖ₌₀ᵈ⁻¹ (𝑧ₖ − 𝑢ₖ)𝑞ₖ(𝑧) = 0
//         let z_challenge = (0..num_vars)
//             .map(|_| Fr::rand(&mut rng))
//             .collect::<Vec<_>>();

//         let mut res = multilinear_f.evaluate(&z_challenge);
//         res -= v_evaluation;

//         for (k, q_k_uni) in quotients.iter().enumerate() {
//             let z_partial = &z_challenge[z_challenge.len() - k..];
//             //This is a weird consequence of how things are done.. the univariate polys are of the multilinear commitment in lagrange basis. Therefore we evaluate as multilinear
//             let q_k = DenseMultilinearExtension::new(q_k_uni.coeffs.clone());
//             let q_k_eval = q_k.evaluate(z_partial);

//             res -= (z_challenge[z_challenge.len() - k - 1]
//                 - u_challenge[z_challenge.len() - k - 1])
//                 * q_k_eval;
//         }
//         assert!(res.is_zero());
//     }

//     /// Test for construction of batched lifted degree quotient:
//     ///  ̂q = ∑ₖ₌₀ⁿ⁻¹ yᵏ Xᵐ⁻ᵈᵏ⁻¹ ̂qₖ, 𝑑ₖ = deg(̂q), 𝑚 = 𝑁
//     #[test]
//     fn batched_lifted_degree_quotient() {
//         let num_vars = 3;
//         let n = 1 << num_vars;

//         // Define mock qₖ with deg(qₖ) = 2ᵏ⁻¹
//         let q_0 = UniPoly::from_coeff(vec![Fr::one()]);
//         let q_1 = UniPoly::from_coeff(vec![Fr::from(2u64), Fr::from(3u64)]);
//         let q_2 = UniPoly::from_coeff(vec![
//             Fr::from(4u64),
//             Fr::from(5u64),
//             Fr::from(6u64),
//             Fr::from(7u64),
//         ]);
//         let quotients = vec![q_0, q_1, q_2];

//         let mut rng = test_rng();
//         let y_challenge = Fr::rand(&mut rng);

//         //Compute batched quptient  ̂q
//         let (batched_quotient, _) =
//             compute_batched_lifted_degree_quotient::<Bn254>(&quotients, &y_challenge);

//         //Explicitly define q_k_lifted = X^{N-2^k} * q_k and compute the expected batched result
//         let q_0_lifted = UniPoly::from_coeff(vec![
//             Fr::zero(),
//             Fr::zero(),
//             Fr::zero(),
//             Fr::zero(),
//             Fr::zero(),
//             Fr::zero(),
//             Fr::zero(),
//             Fr::one(),
//         ]);
//         let q_1_lifted = UniPoly::from_coeff(vec![
//             Fr::zero(),
//             Fr::zero(),
//             Fr::zero(),
//             Fr::zero(),
//             Fr::zero(),
//             Fr::zero(),
//             Fr::from(2u64),
//             Fr::from(3u64),
//         ]);
//         let q_2_lifted = UniPoly::from_coeff(vec![
//             Fr::zero(),
//             Fr::zero(),
//             Fr::zero(),
//             Fr::zero(),
//             Fr::from(4u64),
//             Fr::from(5u64),
//             Fr::from(6u64),
//             Fr::from(7u64),
//         ]);

//         //Explicitly compute  ̂q i.e. RLC of lifted polys
//         let mut batched_quotient_expected = UniPoly::from_coeff(vec![Fr::zero(); n]);

//         batched_quotient_expected += &q_0_lifted;
//         batched_quotient_expected += &(q_1_lifted * y_challenge);
//         batched_quotient_expected += &(q_2_lifted * (y_challenge * y_challenge));
//         assert_eq!(batched_quotient, batched_quotient_expected);
//     }

//     /// evaluated quotient \zeta_x
//     ///
//     /// 𝜁 = 𝑓 − ∑ₖ₌₀ⁿ⁻¹𝑦ᵏ𝑥ʷˢ⁻ʷ⁺¹𝑓ₖ  = 𝑓 − ∑_{d ∈ {d₀, ..., dₙ₋₁}} X^{d* - d + 1}  − ∑{k∶ dₖ=d} yᵏ fₖ , where d* = lifted degree
//     ///
//     /// 𝜁 =  ̂q - ∑ₖ₌₀ⁿ⁻¹ yᵏ Xᵐ⁻ᵈᵏ⁻¹ ̂qₖ, m = N
//     #[test]
//     fn partially_evaluated_quotient_zeta() {
//         let num_vars = 3;
//         let n: u64 = 1 << num_vars;

//         let mut rng = test_rng();
//         let x_challenge = Fr::rand(&mut rng);
//         let y_challenge = Fr::rand(&mut rng);

//         let challenges: Vec<_> = (0..num_vars).map(|_| Fr::rand(&mut rng)).collect();
//         let z_challenge = Fr::rand(&mut rng);

//         let (_, (zeta_x_scalars, _)) =
//             eval_and_quotient_scalars::<Bn254>(y_challenge, x_challenge, z_challenge, &challenges);

//         // To verify we manually compute zeta using the computed powers and expected
//         // 𝜁 =  ̂q - ∑ₖ₌₀ⁿ⁻¹ yᵏ Xᵐ⁻ᵈᵏ⁻¹ ̂qₖ, m = N
//         assert_eq!(
//             zeta_x_scalars[0],
//             -x_challenge.pow(BigInt::<1>::from(n - 1))
//         );

//         // assert_eq!(
//         //     zeta_x_scalars[1],
//         //     -y_challenge * x_challenge.pow(BigInt::<1>::from(n - 1 - 1))
//         // );

//         // assert_eq!(
//         //     zeta_x_scalars[2],
//         //     -y_challenge * y_challenge * x_challenge.pow(BigInt::<1>::from(n - 3 - 1))
//         // );
//     }

//     /// Test efficiently computing 𝛷ₖ(x) = ∑ᵢ₌₀ᵏ⁻¹xⁱ
//     /// 𝛷ₖ(𝑥) = ∑ᵢ₌₀ᵏ⁻¹𝑥ⁱ = (𝑥²^ᵏ − 1) / (𝑥 − 1)
//     #[test]
//     fn phi_n_x_evaluation() {
//         const N: u64 = 8u64;
//         let log_N = (N as usize).log_2();

//         // 𝛷ₖ(𝑥)
//         let mut rng = test_rng();
//         let x_challenge = Fr::rand(&mut rng);

//         let efficient = (x_challenge.pow(BigInt::<1>::from((1 << log_N) as u64)) - Fr::one())
//             / (x_challenge - Fr::one());
//         let expected: Fr = phi::<Bn254>(&x_challenge, log_N);
//         assert_eq!(efficient, expected);
//     }

//     /// Test efficiently computing 𝛷ₖ(x) = ∑ᵢ₌₀ᵏ⁻¹xⁱ
//     /// 𝛷ₙ₋ₖ₋₁(𝑥²^ᵏ⁺¹) = (𝑥²^ⁿ − 1) / (𝑥²^ᵏ⁺¹ − 1)
//     #[test]
//     fn phi_n_k_1_x_evaluation() {
//         const N: u64 = 8u64;
//         let log_N = (N as usize).log_2();

//         // 𝛷ₖ(𝑥)
//         let mut rng = test_rng();
//         let x_challenge = Fr::rand(&mut rng);
//         let k = 2;

//         //𝑥²^ᵏ⁺¹
//         let x_pow = x_challenge.pow(BigInt::<1>::from((1 << (k + 1)) as u64));

//         //(𝑥²^ⁿ − 1) / (𝑥²^ᵏ⁺¹ − 1)
//         let efficient = (x_challenge.pow(BigInt::<1>::from((1 << log_N) as u64)) - Fr::one())
//             / (x_pow - Fr::one());
//         let expected: Fr = phi::<Bn254>(&x_challenge, log_N - k - 1);
//         assert_eq!(efficient, expected);
//     }

//     /// Test construction of 𝑍ₓ
//     /// 𝑍ₓ =  ̂𝑓 − 𝑣 ∑ₖ₌₀ⁿ⁻¹(𝑥²^ᵏ𝛷ₙ₋ₖ₋₁(𝑥ᵏ⁺¹)− 𝑢ₖ𝛷ₙ₋ₖ(𝑥²^ᵏ)) ̂qₖ
//     #[test]
//     fn partially_evaluated_quotient_z_x() {
//         let num_vars = 3;

//         // Construct a random multilinear polynomial f, and (u,v) such that f(u) = v.
//         let mut rng = test_rng();
//         let challenges: Vec<_> = (0..num_vars).map(|_| Fr::rand(&mut rng)).collect();

//         let u_rev = {
//             let mut res = challenges.clone();
//             res.reverse();
//             res
//         };

//         let x_challenge = Fr::rand(&mut rng);
//         let y_challenge = Fr::rand(&mut rng);
//         let z_challenge = Fr::rand(&mut rng);

//         // Construct Z_x scalars
//         let (_, (_, z_x_scalars)) =
//             eval_and_quotient_scalars::<Bn254>(y_challenge, x_challenge, z_challenge, &challenges);

//         for k in 0..num_vars {
//             let x_pow_2k = x_challenge.pow(BigInt::<1>::from((1 << k) as u64)); // x^{2^k}
//             let x_pow_2kp1 = x_challenge.pow(BigInt::<1>::from((1 << (k + 1)) as u64)); // x^{2^{k+1}}
//                                                                                         // x^{2^k} * \Phi_{n-k-1}(x^{2^{k+1}}) - u_k *  \Phi_{n-k}(x^{2^k})
//             let mut scalar = x_pow_2k * phi::<Bn254>(&x_pow_2kp1, num_vars - k - 1)
//                 - u_rev[k] * phi::<Bn254>(&x_pow_2k, num_vars - k);
//             scalar *= z_challenge;
//             scalar *= Fr::from(-1);
//             assert_eq!(z_x_scalars[k], scalar);
//         }
//     }

//     #[test]
//     fn zeromorph_commit_prove_verify() {
//         for num_vars in [4, 5, 6] {
//             let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(num_vars as u64);

//             let poly =
//                 MultilinearPolynomial::LargeScalars(DenseMultilinearExtension::random(num_vars, &mut rng));
//             let point: Vec<<Bn254 as Pairing>::ScalarField> = (0..num_vars)
//                 .map(|_| <Bn254 as Pairing>::ScalarField::rand(&mut rng))
//                 .collect();
//             let eval = poly.evaluate(&point);

//             let srs = ZeromorphSRS::<Bn254>::setup(&mut rng, 1 << num_vars);
//             let (pk, vk) = srs.trim(1 << num_vars);
//             let commitment = Zeromorph::<Bn254>::commit(&pk, &poly).unwrap();

//             let mut prover_transcript = Blake2bTranscript::new(b"TestEval");
//             let proof = Zeromorph::<Bn254>::open(&pk, &poly, &point, &eval, &mut prover_transcript)
//                 .unwrap();
//             let p_transcript_squeeze: <Bn254 as Pairing>::ScalarField =
//                 prover_transcript.challenge_scalar();

//             // Verify proof.
//             let mut verifier_transcript = Blake2bTranscript::new(b"TestEval");
//             Zeromorph::<Bn254>::verify(
//                 &vk,
//                 &commitment,
//                 &point,
//                 &eval,
//                 &proof,
//                 &mut verifier_transcript,
//             )
//             .unwrap();
//             let v_transcript_squeeze: <Bn254 as Pairing>::ScalarField =
//                 verifier_transcript.challenge_scalar();

//             assert_eq!(p_transcript_squeeze, v_transcript_squeeze);

//             // evaluate bad proof for soundness
//             let altered_verifier_point = point
//                 .iter()
//                 .map(|s| *s + <Bn254 as Pairing>::ScalarField::one())
//                 .collect::<Vec<_>>();
//             let altered_verifier_eval = poly.evaluate(&altered_verifier_point);
//             let mut verifier_transcript = Blake2bTranscript::new(b"TestEval");
//             assert!(Zeromorph::<Bn254>::verify(
//                 &vk,
//                 &commitment,
//                 &altered_verifier_point,
//                 &altered_verifier_eval,
//                 &proof,
//                 &mut verifier_transcript,
//             )
//             .is_err())
//         }
//     }
// }
