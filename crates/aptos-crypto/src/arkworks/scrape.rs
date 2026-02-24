// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This module provides a mechanism to check whether a set of polynomial evaluations
//! corresponds to a polynomial of bounded degree. It implements the dual code word
//! approach of the SCRAPE protocol [CD17e].

use crate::{
    arkworks,
    arkworks::{msm::MsmInput, random},
};
use anyhow::{bail, ensure, Context};
use ark_ec::CurveGroup;
use ark_ff::{FftField, PrimeField};
use ark_poly::domain::{EvaluationDomain, Radix2EvaluationDomain};
use ark_std::vec::Vec;

/// A dual code word polynomial $f$ of degree $n-t-1$ for checking that the $n$ evaluations of another
/// polynomial (typically at the roots-of-unity $p(\omega^i)$, \forall i \in [0, n)$) encode a degree
/// $\le t-1$ polynomial.
///
/// When `includes_zero` is true, $n-1$ of the $n$ evaluations are at the roots of unity and the $n$th
/// evaluation is at zero.
pub struct LowDegreeTest<'a, F: FftField> {
    f: Vec<F>,
    includes_zero: bool,
    t: usize,
    n: usize,
    batch_dom: &'a Radix2EvaluationDomain<F>, // TODO: maybe make this more general than Radix2EvaluationDomain?
}

impl<'a, F: PrimeField> LowDegreeTest<'a, F> {
    /// Creates a new LDT given a pre-generated random polynomial `f` of expected degree `n-t-1`.
    pub fn new(
        f: Vec<F>,
        t: usize,
        n: usize,
        includes_zero: bool,
        batch_dom: &'a Radix2EvaluationDomain<F>,
    ) -> anyhow::Result<Self> {
        let min_size = if includes_zero { n - 1 } else { n };
        if batch_dom.size() < min_size {
            bail!(
                "expected batch evaluation domain size {} to be >= {}",
                batch_dom.size(),
                min_size
            );
        }

        if t > n {
            bail!("expected threshold {} to be <= {}", t, n)
        }

        if f.len() != n - t {
            bail!(
                "random polynomial f degree is {}; expected degree n - t - 1 = {}",
                f.len() - 1,
                n - t - 1
            )
        }

        if f.is_empty() && t != n {
            bail!("expected polynomial f to be non-empty when t != n");
        }

        Ok(Self {
            f,
            includes_zero,
            t,
            n,
            batch_dom,
        })
    }

    /// Creates a new LDT by picking a random polynomial `f` of expected degree `n-t-1`.
    pub fn random<R: rand::RngCore + rand::CryptoRng>(
        rng: &mut R,
        t: usize,
        n: usize,
        includes_zero: bool,
        batch_dom: &'a Radix2EvaluationDomain<F>,
    ) -> Self {
        Self::new(
            random::sample_field_elements(n - t, rng),
            t,
            n,
            includes_zero,
            batch_dom,
        )
        .unwrap()
    }

    /// When `include_zero` is false, checks if the evaluations $p(\omega^i)$, \forall i \in [0, n)$ stored
    /// in `evals[i]` encode a degree $\le t-1$ polynomial.
    ///
    /// When `include_zero` is true, checks if the evaluations $p(0)$ in `evals[n-1]` and
    /// $p(\omega^i)$ in `evals[i]` encode a degree $\le t-1$ polynomial (i.e., there are only $n-1$
    /// evaluations at the roots of unity).
    pub fn low_degree_test(&self, evals: &[F]) -> anyhow::Result<()> {
        // This includes the extra evaluation at zero when `includes_zero` is true.
        if evals.len() != self.n {
            bail!("Expected {} evaluations; got {}", self.n, evals.len());
        }

        // In this case, $n$ evaluations will always encode a degree $\le n-1$ polynomial, so we
        // return true.
        if self.t == self.n {
            return Ok(());
        }

        let v_times_f = self.dual_code_word();

        // Let v_i be the coefficients returned by `all_lagrange_denominators` inside the
        // `dual_code_word` call.
        //
        // When `includes_zero` is false, computes \sum_{i \in [0, n)} p(\omega^i) v_i f(\omega^i), which
        // should be zero.
        // When `includes_zero` is true, computes the same as above, but times an extra term v_n f(0).
        debug_assert_eq!(
            evals.len(),
            v_times_f.len(),
            "Lengh of evals and v_times_f did not match"
        );

        let zero: F = evals
            .iter()
            .zip(v_times_f.iter())
            .map(|(p, vf)| p.mul(vf))
            .sum();

        (zero.is_zero()).then_some(()).context(format!(
            "the LDT scalar inner product should return zero, but instead returned {}",
            zero
        ))
    }

    /// Constructs the MSM input used by the LDT: the affine group elements and
    /// the corresponding dual-codeword scalars.
    pub fn ldt_msm_input<C: CurveGroup<ScalarField = F>>(
        &self,
        bases: &[C::Affine],
    ) -> anyhow::Result<MsmInput<C::Affine, F>> {
        if bases.len() != self.n {
            bail!("Expected {} evaluations; got {}", self.n, bases.len())
        }

        if self.t == self.n {
            // In this case the MSM is known to evaluate to zero, but we return an empty input
            // so that the caller can still follow a uniform pipeline.
            return Ok(MsmInput {
                bases: vec![],
                scalars: vec![],
            });
        }

        let v_times_f = self.dual_code_word();

        let scalars = v_times_f;

        Ok(MsmInput::new(bases.to_vec(), scalars).expect("Could not construct MsmInput"))
    }

    /// Performs the LDT given group elements $G^{p(\omega^i)} \in
    pub fn low_degree_test_group<C: CurveGroup<ScalarField = F>>(
        &self,
        evals: &[C::Affine],
    ) -> anyhow::Result<()> {
        // Step 1: build MSM input
        let msm_input = self.ldt_msm_input::<C>(evals)?;

        // Early return in the trivial case
        if msm_input.bases.is_empty() {
            return Ok(());
        }

        // Step 2: perform MSM
        let result = C::msm(&msm_input.bases, &msm_input.scalars).unwrap();

        // Step 3: enforce expected zero
        ensure!(
            result == C::ZERO,
            "the LDT MSM should have returned zero, but returned {}",
            result
        );

        Ok(())
    }

    /// Returns the dual code word for the SCRAPE low-degree test (as per Section 2.1 in [CD17e])
    /// on a polynomial of degree `deg` evaluated over either:
    ///
    ///  - all $n$ roots of unity in `batch_dom`, if `include_zero` is false
    ///  - 0 and all $n-1$ roots of unity in `batch_dom`, if `include_zero` is true
    ///
    /// [CD17e] SCRAPE: Scalable Randomness Attested by Public Entities; by Ignacio Cascudo and
    /// Bernardo David; in Cryptology ePrint Archive, Report 2017/216; 2017;
    /// https://eprint.iacr.org/2017/216
    pub fn dual_code_word(&self) -> Vec<F> {
        // Accounts for the size of `f` being the `n` evaluations of f(X) at the roots-of-unity and f(0)
        // when `include_zero` is true.
        let fft_size = if self.includes_zero {
            self.n - 1
        } else {
            self.n
        };
        let f_0 = self.f[0];

        // Compute $f(\omega^i)$ for all $i \in [0, n)$
        let mut f_evals = self.batch_dom.fft(&self.f);
        f_evals.truncate(fft_size);

        // Compute Lagrange denominators
        let v = arkworks::shamir::all_lagrange_denominators(
            self.batch_dom,
            fft_size,
            self.includes_zero,
        );

        // Append f(0), if `include_zero` is true
        let mut extra = Vec::with_capacity(1);
        if self.includes_zero {
            extra.push(f_0);
        }

        // Compute $v_i f(\omega^i), \forall i \in [0, n)$, and $v_n f(0)$ if `include_zero` is true.
        debug_assert_eq!(f_evals.len() + extra.len(), v.len());
        f_evals
            .iter()
            .chain(extra.iter())
            .zip(v.iter())
            .map(|(v, f)| v.mul(f))
            .collect::<Vec<F>>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arkworks::shamir::ShamirThresholdConfig;
    use ark_bn254::Fr;
    use ark_ff::PrimeField;
    use ark_std::vec::Vec;
    use rand::thread_rng;

    /// Helper to simulate sampling a random polynomial, by sampling its coefficients
    fn sample_random_polynomial<F: PrimeField, R: rand::Rng>(degree: usize, rng: &mut R) -> Vec<F> {
        random::sample_field_elements(degree + 1, rng)
    }

    #[test]
    fn test_ldt_correctness() {
        let mut rng = thread_rng();

        // TODO: Move get_threshold_configs_for_testing() and the ThresholdConfig trait to aptos-crypto
        for t in 1..8 {
            for n in (t + 1)..(3 * t + 1) {
                let sc = ShamirThresholdConfig::new(t, n);

                // A degree t-1 polynomial p(X)
                let p = sample_random_polynomial::<Fr, _>(t - 1, &mut rng);

                let mut evals = sc.domain.fft(&p);
                evals.truncate(n);

                // Test deg(p) < t, given evals at n roots of unity over a domain with N = n.next_power_of_two() roots of unity
                let ldt = LowDegreeTest::random(&mut rng, sc.t, sc.n, false, &sc.domain);
                assert!(ldt.low_degree_test(&evals).is_ok());

                if sc.t < sc.n {
                    // Test deg(p) < t + 1, given evals at roots of unity
                    let ldt = LowDegreeTest::random(&mut rng, sc.t + 1, sc.n, false, &sc.domain);
                    assert!(ldt.low_degree_test(&evals).is_ok());
                }

                // Test deg(p) < t, given evals at roots of unity and given p(0)
                evals.push(p[0]);
                let ldt = LowDegreeTest::random(&mut rng, sc.t, sc.n + 1, true, &sc.domain);
                assert!(ldt.low_degree_test(&evals).is_ok());
            }
        }
    }

    #[test]
    fn test_ldt_soundness() {
        let mut rng = thread_rng();

        for t in 1..8 {
            for n in (t + 1)..(3 * t + 1) {
                let sc = ShamirThresholdConfig::new(t, n);

                // A degree t polynomial f(X), higher by 1 than what the LDT expects
                let p = sample_random_polynomial::<Fr, _>(t, &mut rng);

                let mut evals = sc.domain.fft(&p);
                evals.truncate(n);

                // Test deg(p) < t, given evals at roots of unity
                // This should fail, since deg(p) = t
                let ldt = LowDegreeTest::random(&mut rng, sc.t, sc.n, false, &sc.domain);
                assert!(
                    ldt.low_degree_test(&evals).is_err(),
                    "LDT unexpectedly passed. n: {}, t: {}",
                    n,
                    t
                );

                // Test deg(p) < t, given evals at roots of unity and given p(0)
                // This should fail, since deg(p) = t
                evals.push(p[0]);
                let ldt = LowDegreeTest::random(&mut rng, sc.t, sc.n + 1, true, &sc.domain); // Here using n+1 because p(0) is added
                assert!(
                    ldt.low_degree_test(&evals).is_err(),
                    "LDT unexpectedly passed. n: {}, t: {}",
                    n,
                    t
                );
            }
        }
    }
}
