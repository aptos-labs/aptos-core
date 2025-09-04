// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

/// Low-degree test from the SCRAPE paper that checks whether $n$ evaluations encode a degree $\le t-1$
/// polynomial.
use crate::{
    algebra::{
        evaluation_domain::BatchEvaluationDomain, fft::fft_assign,
        lagrange::all_lagrange_denominators,
    },
    utils::{g1_multi_exp, g2_multi_exp, random::random_scalars},
};
use anyhow::{bail, Context};
use blstrs::{G1Projective, G2Projective, Scalar};
use ff::Field;
use group::Group;
use std::ops::Mul;

/// A dual code word polynomial $f$ of degree $n-t-1$ for checking that the $n$ evaluations of another
/// polynomial (typically at the roots-of-unity $p(\omega^i)$, \forall \in [0, n)$) encode a degree
/// $\le t-1$ polynomial.
///
/// When `includes_zero` is true, $n-1$ of the $n$ evaluations are at the roots of unity and the $n$th
/// evaluation is at zero.
pub struct LowDegreeTest<'a> {
    /// Consider a degree-$(t-1)$ polynomial $p(X)$. Its "dual" polynomial $f(X)$ will be of degree
    /// $n - t - 1$, and will have $n - t$ coefficients.
    f: Vec<Scalar>,
    includes_zero: bool,
    t: usize,
    n: usize,
    batch_dom: &'a BatchEvaluationDomain,
}

impl<'a> LowDegreeTest<'a> {
    /// Creates a new LDT given a pre-generated random polynomial `f` of expected degree `n-t-1`.
    pub fn new(
        f: Vec<Scalar>,
        t: usize,
        n: usize,
        includes_zero: bool,
        batch_dom: &'a BatchEvaluationDomain,
    ) -> anyhow::Result<Self> {
        let min_size = if includes_zero { n - 1 } else { n };
        if batch_dom.N() < min_size {
            bail!(
                "expected batch evaluation domain size {} to be >= {}",
                batch_dom.N(),
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
    pub fn random<R: rand_core::RngCore + rand_core::CryptoRng>(
        mut rng: &mut R,
        t: usize,
        n: usize,
        includes_zero: bool,
        batch_dom: &'a BatchEvaluationDomain,
    ) -> Self {
        Self::new(
            random_scalars(n - t, &mut rng),
            t,
            n,
            includes_zero,
            batch_dom,
        )
        .unwrap()
    }

    /// When `include_zero` is false, checks if the evaluations $p(\omega^i)$, \forall \in [0, n)$ stored
    /// in `evals[i]` encode a degree $\le t-1$ polynomial.
    ///
    /// When `include_zero` is true, checks if the evaluations $p(0)$ in `evals[n-1]` and
    /// $p(\omega^i)$ in `evals[i]` encode a degree $\le t-1$ polynomial (i.e., there are only $n-1$
    /// evaluations at the roots of unity).
    pub fn low_degree_test(self, evals: &Vec<Scalar>) -> anyhow::Result<()> {
        // This includes the extra evaluation at zero when `includes_zero` is true.
        if evals.len() != self.n {
            bail!("Expected {} evaluations; got {}", self.n, evals.len());
        }

        // println!(
        //     "\nscrape_low_degree_test> N: {}, t: {t}, n: {n}, include_zero: {includes_zero}",
        //     batch_dom.N()
        // );

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
        debug_assert_eq!(evals.len(), v_times_f.len());
        let zero: Scalar = evals
            .iter()
            .zip(v_times_f.iter())
            .map(|(p, vf)| p.mul(vf))
            .sum();

        (zero == Scalar::ZERO).then_some(()).context(format!(
            "the LDT scalar inner product should return zero, but instead returned {}",
            zero
        ))
    }

    /// Like `low_degree_test` but for `evals[i]` being $g^{p(\omega^i)} \in \mathbb{G}_1$.
    pub fn low_degree_test_on_g1(self, evals: &Vec<G1Projective>) -> anyhow::Result<()> {
        if evals.len() != self.n {
            bail!("Expected {} evaluations; got {}", self.n, evals.len())
        }

        if self.t == self.n {
            return Ok(());
        }

        let v_times_f = self.dual_code_word();

        debug_assert_eq!(evals.len(), v_times_f.len());
        let zero = g1_multi_exp(evals.as_ref(), v_times_f.as_slice());

        (zero == G1Projective::identity())
            .then_some(())
            .context(format!(
                "the LDT G1 multiexp should return zero, but instead returned {}",
                zero
            ))
    }

    /// Like `low_degree_test` but for `evals[i]` being $g^{p(\omega^i)} \in \mathbb{G}_2$.
    pub fn low_degree_test_on_g2(self, evals: &Vec<G2Projective>) -> anyhow::Result<()> {
        if evals.len() != self.n {
            bail!("Expected {} evaluations; got {}", self.n, evals.len())
        }

        if self.t == self.n {
            return Ok(());
        }

        let v_times_f = self.dual_code_word();

        debug_assert_eq!(evals.len(), v_times_f.len());
        let zero = g2_multi_exp(evals.as_ref(), v_times_f.as_slice());

        (zero == G2Projective::identity())
            .then_some(())
            .context(format!(
                "the LDT G2 multiexp should return zero, but instead returned {}",
                zero
            ))
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
    pub fn dual_code_word(self) -> Vec<Scalar> {
        // println!("dual_code_word   > t: {t}, n: {n}, includes_zero: {includes_zero}");

        // Accounts for the size of `f` being the `n` evaluations of f(X) at the roots-of-unity and f(0)
        // when `include_zero` is true.
        let fft_size = if self.includes_zero {
            self.n - 1
        } else {
            self.n
        };
        let f_0 = self.f[0];

        // Compute $f(\omega^i)$ for all $i \in [0, n)$
        let dom = self.batch_dom.get_subdomain(fft_size);
        let mut f_evals = self.f;
        fft_assign(&mut f_evals, &dom);
        f_evals.truncate(fft_size);

        let v = all_lagrange_denominators(&self.batch_dom, fft_size, self.includes_zero);

        // Append f(0), if `include_zero` is true
        let mut extra = Vec::with_capacity(1);
        if self.includes_zero {
            extra.push(f_0);
        }

        // println!(
        //     "|v| = {}, |f_evals| = {}, |extra| = {}",
        //     v.len(),
        //     f_evals.len(),
        //     extra.len()
        // );

        // Compute $v_i f(\omega^i), \forall i \in [0, n)$, and $v_n f(0)$ if `include_zero` is true.
        debug_assert_eq!(f_evals.len() + extra.len(), v.len());
        f_evals
            .iter()
            .chain(extra.iter())
            .zip(v.iter())
            .map(|(v, f)| v.mul(f))
            .collect::<Vec<Scalar>>()
    }
}

#[cfg(test)]
mod test {
    use crate::{
        algebra::{evaluation_domain::BatchEvaluationDomain, fft::fft_assign},
        pvss::{test_utils, LowDegreeTest, ThresholdConfig},
        utils::random::random_scalars,
    };
    use blstrs::Scalar;
    use rand::{prelude::ThreadRng, thread_rng};

    #[test]
    fn test_ldt_correctness() {
        let mut rng = thread_rng();

        for sc in test_utils::get_threshold_configs_for_testing() {
            // A degree t-1 polynomial p(X)
            let (p_0, batch_dom, mut evals) = random_polynomial_evals(&mut rng, &sc);

            // Test deg(p) < t, given evals at roots of unity
            let ldt = LowDegreeTest::random(&mut rng, sc.t, sc.n, false, &batch_dom);
            assert!(ldt.low_degree_test(&evals).is_ok());

            if sc.t < sc.n {
                // Test deg(p) < t + 1, given evals at roots of unity
                let ldt = LowDegreeTest::random(&mut rng, sc.t + 1, sc.n, false, &batch_dom);
                assert!(ldt.low_degree_test(&evals).is_ok());
            }

            // Test deg(p) < t, given evals at roots of unity and given p(0)
            evals.push(p_0);
            let ldt = LowDegreeTest::random(&mut rng, sc.t, sc.n + 1, true, &batch_dom);
            assert!(ldt.low_degree_test(&evals).is_ok());
        }
    }

    /// Test the soundness of the LDT: a polynomial of degree > t - 1 should not pass the check.
    #[test]
    fn test_ldt_soundness() {
        let mut rng = thread_rng();

        for t in 1..8 {
            for n in (t + 1)..(3 * t + 1) {
                let sc = ThresholdConfig::new(t, n).unwrap();
                let sc_higher_degree = ThresholdConfig::new(sc.t + 1, sc.n).unwrap();

                // A degree t polynomial p(X), higher by 1 than what the LDT expects
                let (p_0, batch_dom, mut evals) =
                    random_polynomial_evals(&mut rng, &sc_higher_degree);

                // Test deg(p) < t, given evals at roots of unity
                // This should fail, since deg(p) = t
                let ldt = LowDegreeTest::random(&mut rng, sc.t, sc.n, false, &batch_dom);
                assert!(ldt.low_degree_test(&evals).is_err());

                // Test deg(p) < t, given evals at roots of unity and given p(0)
                // This should fail, since deg(p) = t
                evals.push(p_0);
                let ldt = LowDegreeTest::random(&mut rng, sc.t, sc.n + 1, true, &batch_dom);
                assert!(ldt.low_degree_test(&evals).is_err());
            }
        }
    }

    fn random_polynomial_evals(
        mut rng: &mut ThreadRng,
        sc: &ThresholdConfig,
    ) -> (Scalar, BatchEvaluationDomain, Vec<Scalar>) {
        let p = random_scalars(sc.t, &mut rng);
        let p_0 = p[0];
        let batch_dom = BatchEvaluationDomain::new(sc.n);

        // Compute p(\omega^i) for all i's
        // (e.g., in SCRAPE we will be given A_i = g^{p(\omega^i)})
        let mut p_evals = p;
        fft_assign(&mut p_evals, &batch_dom.get_subdomain(sc.n));
        p_evals.truncate(sc.n);
        (p_0, batch_dom, p_evals)
    }
}
