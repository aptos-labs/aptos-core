// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_crypto::CryptoMaterialError;
use blstrs::Scalar;
use ff::{Field, PrimeField};
use more_asserts::{assert_gt, assert_le};
use serde::{Deserialize, Serialize};

/// This struct abstracts the notion of an FFT evaluation domain over the scalar field of our curve.
/// This consists of $N = 2^k$ and an $N$th root of unity. (The $\log_2{N}$ field is just handy in our FFT
/// implementation.)
#[allow(non_snake_case)]
#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct EvaluationDomain {
    /// The actual number $n$ of evaluations we want, which might not be a power of two.
    pub(crate) n: usize,
    /// The smallest power of two $N \ge n$.
    pub(crate) N: usize,
    /// $\log_2{N}$
    pub(crate) log_N: usize,
    /// An $N$th primitive root of unity $\omega$ that generates a multiplicative subgroup $\{\omega^0, \omega^1, \ldots, \omega^{N-1}\}$.
    pub(crate) omega: Scalar,
    /// The inverse $\omega^{-1}$.
    pub(crate) omega_inverse: Scalar,
    // geninv: Scalar,
    /// The inverse of $N$ when viewed as a scalar in the multiplicative group of the scalar field.
    pub(crate) N_inverse: Scalar,
}

/// A `BatchEvaluationDomain` struct encodes multiple `EvaluationDomain` structs in a more efficient way.
/// This is very useful when doing FFTs of different sizes (e.g., 1, 2, 4, 8, 16, ... in an accumulator-style
/// polynomial multiplication; see `accumulator_poly` function) and we want to avoid recomputing the
/// same roots of unity multiple times, as well as other scalars.
#[allow(non_snake_case)]
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct BatchEvaluationDomain {
    /// $\log_2{N}$
    pub(crate) log_N: usize,
    /// A vector of all $N$th roots of unity $\{\omega^0, \omega^1, \omega^2, \ldots, \omega^{N-1}\}$
    omegas: Vec<Scalar>,
    /// A vector of $i^{-1}$, for all $i \in \{1, 2, 4, 8, \ldots, N/2, N\}$
    N_inverses: Vec<Scalar>,
}

/// Returns the highest $N = 2^k$ such that $N \ge n$.
#[allow(non_snake_case)]
pub fn smallest_power_of_2_greater_than_or_eq(n: usize) -> (usize, usize) {
    let mut N = 1;
    let mut log_N: usize = 0;

    while N < n {
        N <<= 1;
        log_N += 1;
    }

    (N, log_N)
}

impl EvaluationDomain {
    /// `n` is the max number of evaluations at the roots of unity we want to get, but might not be
    /// a power of two. So we find the smallest $N$ which a power of two such that $N >= n$.
    ///
    /// For FFT-based multiplication, we set $n - 1$ to be the degree of the polynomials we are
    /// multiplying.
    #[allow(non_snake_case)]
    pub fn new(n: usize) -> Result<EvaluationDomain, CryptoMaterialError> {
        // Compute the size of our evaluation domain
        let (N, log_N) = smallest_power_of_2_greater_than_or_eq(n);

        // The pairing-friendly curve may not be able to support
        // large enough (radix2) evaluation domains.
        if log_N >= Scalar::S as usize {
            return Err(CryptoMaterialError::WrongLengthError);
        }

        // Compute $\omega$, the $N$th primitive root of unity
        let omega = Self::get_Nth_root_of_unity(log_N);

        Ok(EvaluationDomain {
            n,
            N,
            log_N,
            omega,
            omega_inverse: omega.invert().unwrap(),
            // geninv: Scalar::multiplicative_generator().invert().unwrap(),
            N_inverse: Scalar::from(N as u64).invert().unwrap(),
        })
    }

    /// Efficiently returns the $N$th primitive root of unity that's been precomputed in this evaluation domain.
    pub fn get_primitive_root_of_unity(&self) -> &Scalar {
        &self.omega
    }

    /// Returns a primitive $N$th root of unity in the scalar field, given $\log_2{N}$ as an argument.
    #[allow(non_snake_case)]
    fn get_Nth_root_of_unity(log_N: usize) -> Scalar {
        let mut omega = Scalar::ROOT_OF_UNITY;
        for _ in log_N..Scalar::S as usize {
            omega = omega.square();
        }
        omega
    }
}

impl BatchEvaluationDomain {
    /// Returns a batch evaluation domain for FFTs of size $1, 2, 4, 8, 16, \ldots n$, where $n$ is the
    /// number of coefficients in the polynomial $f(X) \cdot g(X)$.
    ///
    /// This then allows more efficient fetching of subdomains for any of those sizes than via
    /// `get_evaluation_dom_for_multiplication`.
    #[allow(non_snake_case)]
    pub fn new(n: usize) -> Self {
        let (N, log_N) = smallest_power_of_2_greater_than_or_eq(n);
        let omega = EvaluationDomain::get_Nth_root_of_unity(log_N);

        let mut omegas = Vec::with_capacity(N);
        omegas.push(Scalar::ONE);

        let mut acc = omega;
        for _ in 1..N {
            omegas.push(acc);
            acc *= omega; // $\omega^i$
        }

        debug_assert_eq!(omegas.len(), N);

        let mut N_inverses = Vec::with_capacity(log_N);
        let mut i = 1u64;
        for _ in 0..=log_N {
            N_inverses.push(Scalar::from(i).invert().unwrap());

            i *= 2;
        }

        debug_assert_eq!(
            N_inverses.last().unwrap().invert().unwrap(),
            Scalar::from(N as u64)
        );

        BatchEvaluationDomain {
            log_N,
            omegas,
            N_inverses,
        }
    }

    #[allow(non_snake_case)]
    pub fn N(&self) -> usize {
        self.omegas.len()
    }

    /// Returns the equivalent of `EvaluationDomain::new(k)`, but much faster since everything is precomputed.
    #[allow(non_snake_case)]
    pub fn get_subdomain(&self, k: usize) -> EvaluationDomain {
        assert_le!(k, self.omegas.len());
        assert_ne!(k, 0);

        let (K, log_K) = smallest_power_of_2_greater_than_or_eq(k);
        assert_gt!(K, 0);

        let K_inverse = self.N_inverses[log_K];
        debug_assert_eq!(K_inverse.invert().unwrap(), Scalar::from(K as u64));

        let mut idx = 1;
        for _ in log_K..self.log_N {
            // i.e., omega = omega.square();
            idx *= 2;
        }

        let N = self.omegas.len();
        let omega = self.omegas[idx % N];
        debug_assert!(Self::is_order(&omega, K));

        let omega_inverse = self.omegas[(N - idx) % N];
        debug_assert_eq!(omega_inverse.invert().unwrap(), omega);

        EvaluationDomain {
            n: k,
            N: K,
            log_N: log_K,
            omega,
            omega_inverse,
            N_inverse: K_inverse,
        }
    }

    /// Efficiently returns the $i$th $N$th root of unity $\omega^i$, for $i\in[0, N)$.
    pub fn get_root_of_unity(&self, i: usize) -> Scalar {
        self.omegas[i]
    }

    /// Efficiently returns all the $N$th roots of unity.
    pub fn get_all_roots_of_unity(&self) -> &Vec<Scalar> {
        self.omegas.as_ref()
    }

    /// Asserts the order of $\omega$ is $K$.
    #[allow(non_snake_case)]
    fn is_order(omega: &Scalar, K: usize) -> bool {
        assert_gt!(K, 0);
        let mut acc = omega.clone();

        // First, check that \omega^1, \omega^2, \omega^{K-1} are NOT the identity.
        for _ in 1..=K - 1 {
            // println!("\\omega^{i}: {acc}");
            if acc == Scalar::ONE {
                return false;
            }
            acc *= omega;
        }

        // Last, check that \omega^K = \omega^0, i.e., the identity
        // println!("\\omega^{K}: {acc}");
        if acc != Scalar::ONE {
            return false;
        }
        // println!();

        return true;
    }
}
