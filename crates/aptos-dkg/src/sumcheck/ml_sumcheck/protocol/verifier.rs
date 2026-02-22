//! Verifier
use crate::sumcheck::ml_sumcheck::{
    data_structures::PolynomialInfo,
    protocol::{prover::ProverMsg, IPForMLSumcheck},
};
use ark_ff::Field;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::{rand::RngCore, vec::Vec};

#[derive(Clone, CanonicalSerialize, CanonicalDeserialize)]
/// Verifier Message
pub struct VerifierMsg<F: Field> {
    /// randomness sampled by verifier
    pub randomness: F,
}

/// Verifier State
pub struct VerifierState<F: Field> {
    round: usize,
    nv: usize,
    max_multiplicands: usize,
    finished: bool,
    /// a list storing the univariate polynomial in evaluation form sent by the prover at each round
    polynomials_received: Vec<Vec<F>>,
    /// a list storing the randomness sampled by the verifier at each round
    randomness: Vec<F>,
}

/// Subclaim when verifier is convinced
pub struct SubClaim<F: Field> {
    /// the multi-dimensional point that this multilinear extension is evaluated to
    pub point: Vec<F>,
    /// the expected evaluation
    pub expected_evaluation: F,
}

impl<F: Field> IPForMLSumcheck<F> {
    /// initialize the verifier
    pub fn verifier_init(index_info: &PolynomialInfo) -> VerifierState<F> {
        VerifierState {
            round: 1,
            nv: index_info.num_variables,
            max_multiplicands: index_info.max_multiplicands,
            finished: false,
            polynomials_received: Vec::with_capacity(index_info.num_variables),
            randomness: Vec::with_capacity(index_info.num_variables),
        }
    }

    /// Run verifier at current round, given prover message
    pub fn verify_round<R: RngCore>(
        prover_msg: ProverMsg<F>,
        verifier_state: &mut VerifierState<F>,
        rng: &mut R,
    ) -> Option<VerifierMsg<F>> {
        if verifier_state.finished {
            panic!("Incorrect verifier state: Verifier is already finished.");
        }

        let msg = Self::sample_round(rng);
        verifier_state.randomness.push(msg.randomness);
        verifier_state
            .polynomials_received
            .push(prover_msg.evaluations);

        if verifier_state.round == verifier_state.nv {
            verifier_state.finished = true;
        } else {
            verifier_state.round += 1;
        }
        Some(msg)
    }

    /// verify the sumcheck phase, and generate the subclaim
    pub fn check_and_generate_subclaim(
        verifier_state: VerifierState<F>,
        asserted_sum: F,
    ) -> Result<SubClaim<F>, crate::sumcheck::Error> {
        if !verifier_state.finished {
            panic!("Verifier has not finished.");
        }

        let mut expected = asserted_sum;
        if verifier_state.polynomials_received.len() != verifier_state.nv {
            panic!("insufficient rounds");
        }

        for i in 0..verifier_state.nv {
            let evaluations = &verifier_state.polynomials_received[i];

            // Check that we have at least 2 evaluations
            if evaluations.len() < 2 {
                panic!("Need at least 2 evaluations per round");
            }

            let p0 = evaluations[0];
            let p1 = evaluations[1];

            // Check sumcheck relation: g(0) + g(1) = expected
            if p0 + p1 != expected {
                return Err(crate::sumcheck::Error::Reject(Some(
                    "Prover message is not consistent with the claim.".into(),
                )));
            }

            // Interpolate to get g(r)
            expected = interpolate_uni_poly(evaluations, verifier_state.randomness[i]);
        }

        Ok(SubClaim {
            point: verifier_state.randomness,
            expected_evaluation: expected,
        })
    }

    /// simulate a verifier message without doing verification
    #[inline]
    pub fn sample_round<R: RngCore>(rng: &mut R) -> VerifierMsg<F> {
        VerifierMsg {
            randomness: F::rand(rng),
        }
    }
}

/// Interpolate quadratic polynomial given evaluations at 0, 1, 2
pub(crate) fn interpolate_quadratic<F: Field>(evals: &[F], x: F) -> F {
    let g0 = evals[0];
    let g1 = evals[1];
    let g2 = evals[2];

    // Solve for coefficients: g(X) = a₀ + a₁X + a₂X²
    let a0 = g0;
    let a2 = (g2 - F::from(2u64) * g1 + g0) * F::from(2u64).inverse().unwrap();
    let a1 = g1 - g0 - a2;

    // Evaluate at x
    a0 + a1 * x + a2 * x * x
}

/// interpolate the *unique* univariate polynomial of degree *at most*
/// p_i.len()-1 passing through the y-values in p_i at x = 0,..., p_i.len()-1
/// and evaluate this polynomial at `eval_at`.
pub(crate) fn interpolate_uni_poly<F: Field>(p_i: &[F], eval_at: F) -> F {
    let len = p_i.len();

    // Special case for degree 2 (most common with binary constraints)
    if len == 3 {
        return interpolate_quadratic(p_i, eval_at);
    }

    let mut evals = vec![];

    let mut prod = eval_at;
    evals.push(eval_at);

    // prod = ∏_j (eval_at - j)
    // we return early if 0 <= eval_at < len
    let mut check = F::zero();
    for i in 1..len {
        if eval_at == check {
            return p_i[i - 1];
        }
        check += F::one();

        let tmp = eval_at - check;
        evals.push(tmp);
        prod *= tmp;
    }

    if eval_at == check {
        return p_i[len - 1];
    }

    let mut res = F::zero();

    // Use field operations for general case
    let mut denom_up = field_factorial::<F>(len - 1);
    let mut denom_down = F::one();

    for i in (0..len).rev() {
        res += p_i[i] * prod * denom_down / (denom_up * evals[i]);

        // compute denom for the next step is -current_denom * (len-i)/i
        if i != 0 {
            denom_up *= -F::from((len - i) as u64);
            denom_down *= F::from(i as u64);
        }
    }

    res
}

/// compute the factorial(a) = 1 * 2 * ... * a
#[inline]
fn field_factorial<F: Field>(a: usize) -> F {
    let mut res = F::one();
    for i in 1..=a {
        res *= F::from(i as u64);
    }
    res
}

// #[cfg(test)]
// mod test {
//     use super::*;
//     use ark_poly::univariate::DensePolynomial;
//     use ark_poly::DenseUVPolynomial;
//     use ark_poly::Polynomial;
//     use ark_std::vec::Vec;
//     use ark_std::UniformRand;

//     type F = ark_test_curves::bls12_381::Fr;

//     #[test]
//     fn test_interpolation() {
//         let mut prng = ark_std::test_rng();

//         // test a polynomial with 20 known points, i.e., with degree 19
//         let poly = DensePolynomial::<F>::rand(20 - 1, &mut prng);
//         let evals = (0..20)
//             .map(|i| poly.evaluate(&F::from(i)))
//             .collect::<Vec<F>>();
//         let query = F::rand(&mut prng);

//         assert_eq!(poly.evaluate(&query), interpolate_uni_poly(&evals, query));

//         // test a polynomial with 33 known points, i.e., with degree 32
//         let poly = DensePolynomial::<F>::rand(33 - 1, &mut prng);
//         let evals = (0..33)
//             .map(|i| poly.evaluate(&F::from(i)))
//             .collect::<Vec<F>>();
//         let query = F::rand(&mut prng);

//         assert_eq!(poly.evaluate(&query), interpolate_uni_poly(&evals, query));

//         // test a polynomial with 64 known points, i.e., with degree 63
//         let poly = DensePolynomial::<F>::rand(64 - 1, &mut prng);
//         let evals = (0..64)
//             .map(|i| poly.evaluate(&F::from(i)))
//             .collect::<Vec<F>>();
//         let query = F::rand(&mut prng);

//         assert_eq!(poly.evaluate(&query), interpolate_uni_poly(&evals, query));

//         // test interpolation when we ask for the value at an x-coordinate
//         // we are already passing
//         let evals = vec![0, 1, 4, 9]
//             .into_iter()
//             .map(|i| F::from(i))
//             .collect::<Vec<F>>();
//         assert_eq!(interpolate_uni_poly(&evals, F::from(3)), F::from(9));
//     }
// }
