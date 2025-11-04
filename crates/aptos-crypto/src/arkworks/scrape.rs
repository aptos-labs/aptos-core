// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module provides a mechanism to check whether a set of polynomial evaluations
//! corresponds to a polynomial of bounded degree. It implements the dual code word
//! approach of the SCRAPE protocol [CD17e].

use crate::{arkworks, arkworks::rand};
use anyhow::{bail, Context};
use ark_ff::{FftField, PrimeField};
use ark_poly::domain::{EvaluationDomain, Radix2EvaluationDomain};
use ark_std::vec::Vec;

/// A dual code word polynomial $f$ of degree $n-t-1$ for checking that the $n$ evaluations of another
/// polynomial (typically at the roots-of-unity $p(\omega^i)$, \forall \in [0, n)$) encode a degree
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
    pub fn random<R: rand_core::RngCore + rand_core::CryptoRng>(
        mut rng: &mut R,
        t: usize,
        n: usize,
        includes_zero: bool,
        batch_dom: &'a Radix2EvaluationDomain<F>,
    ) -> Self {
        Self::new(
            rand::sample_field_elements(n - t, &mut rng),
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
    pub fn low_degree_test(&self, evals: &Vec<F>) -> anyhow::Result<()> {
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
        debug_assert_eq!(evals.len(), v_times_f.len());

        let mut zero = F::zero();
        for (p, vf) in evals.iter().zip(v_times_f.iter()) {
            let mut tmp = *p;
            tmp.mul_assign(vf);
            zero += tmp;
        }

        (zero.is_zero()).then_some(()).context(format!(
            "the LDT scalar inner product should return zero, but instead returned {}",
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
    pub fn dual_code_word(&self) -> Vec<F> {
        // Accounts for the size of `f` being the `n` evaluations of f(X) at the roots-of-unity and f(0)
        // when `include_zero` is true.
        let fft_size = if self.includes_zero {
            self.n - 1
        } else {
            self.n
        }; // TODO: not sure why this is called fft_size
        let f_0 = self.f[0];

        // Compute $f(\omega^i)$ for all $i \in [0, n)$
        let mut f_evals = self.batch_dom.fft(&self.f);
        f_evals.truncate(fft_size);

        // Compute Lagrange denominators
        let v =
            arkworks::shamir::all_lagrange_denominators(self.batch_dom, self.n, self.includes_zero);

        // Append f(0), if `include_zero` is true
        let mut extra = Vec::with_capacity(1);
        if self.includes_zero {
            extra.push(f_0);
        }

        debug_assert_eq!(f_evals.len() + extra.len(), v.len());

        // Compute $v_i f(\omega^i), \forall i \in [0, n)$, and $v_n f(0)$ if `include_zero` is true.
        f_evals
            .iter()
            .chain(extra.iter())
            .zip(v.iter())
            .map(|(&f, v)| {
                let mut tmp = f;
                tmp.mul_assign(v);
                tmp
            })
            .collect::<Vec<F>>()
    }
}
