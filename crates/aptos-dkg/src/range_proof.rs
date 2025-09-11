// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    algebra::{
        evaluation_domain::{BatchEvaluationDomain, EvaluationDomain},
        fft::{fft_assign, ifft_assign, ifft_assign_g1, ifft_assign_g2},
        polynomials::{poly_add_assign, poly_differentiate, poly_mul_scalar},
    },
    fiat_shamir,
    utils::{
        g1_multi_exp, g2_multi_exp, multi_pairing_g1_g2, pad_to_pow2_len_minus_one,
        random::{random_g1_point, random_g2_point, random_scalar},
    },
};
use anyhow::ensure;
use blstrs::{G1Projective, G2Projective, Gt, Scalar};
#[cfg(feature = "range_proof_timing")]
use ff::derive::bitvec::macros::internal::funty::Fundamental;
use ff::Field;
use group::Group;
use rand::thread_rng;
use rand_core::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
#[cfg(feature = "range_proof_timing")]
use std::time::{Duration, Instant};
use std::{
    iter::once,
    ops::{AddAssign, Mul},
};

pub const DST: &[u8; 42] = b"APTOS_UNIVARIATE_DEKART_V1_RANGE_PROOF_DST";

pub struct PowersOfTau {
    t1: Vec<G1Projective>, // g_1, g_1^{tau}, g_1^{tau^2}, ..., g_1^{tau^n}, where `n` is the batch size
    t2: Vec<G2Projective>,
}

pub fn powers_of_tau<R>(rng: &mut R, n: usize) -> PowersOfTau
where
    R: RngCore + rand::Rng + CryptoRng,
{
    let g1 = random_g1_point(rng);
    let g2 = random_g2_point(rng);
    let tau = random_scalar(rng);
    let mut t1 = vec![g1];
    let mut t2 = vec![g2];
    for i in 0..n {
        t1.push(t1[i] * tau);
        t2.push(t2[i] * tau);
    }
    PowersOfTau { t1, t2 }
}

pub struct PublicParameters {
    taus: PowersOfTau,               // g_1, g_1^{tau}, g_1^{tau^2}, ..., g_1^{tau^n}
    ell: usize,                      // the range is [0, 2^\ell)
    n: usize,                        // the number of values we are batch proving; i.e., batch size
    lagr_g1: Vec<G1Projective>,      // of size n + 1
    lagr_g2: Vec<G2Projective>,      // of size n + 1
    pub vanishing_com: G2Projective, // commitment to deg-n vanishing polynomial (X^{n+1} - 1) / (X - \omega^n) used to test h(X)
    batch_dom_n1: BatchEvaluationDomain, // batch evaluation domain of size (n+1)
    dom_n1: EvaluationDomain,        // (n+1)th root of unity
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Commitment(G1Projective);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Proof {
    d: G1Projective,          // commitment to h(X) = \sum_{j=0}^{\ell-1} beta_j h_j(X)
    c: Vec<G1Projective>,     // of size \ell
    c_hat: Vec<G2Projective>, // of size \ell
}

impl Proof {
    pub fn maul(&mut self) {
        self.c[0] += G1Projective::generator();
    }
}

/// Sets up the Borgeaud range proof for proving that size-`n` batches are in the range [0, 2^\ell).
pub fn setup(ell: usize, n: usize) -> PublicParameters {
    let mut rng = thread_rng();

    let n = (n + 1).next_power_of_two() - 1;
    let num_omegas = n + 1;
    assert!(num_omegas.is_power_of_two());

    let taus = powers_of_tau(&mut rng, n);

    let batch_dom_n1 = BatchEvaluationDomain::new(num_omegas);
    let batch_dom_2n2 = BatchEvaluationDomain::new(num_omegas * 2);
    let dom_n1 = batch_dom_2n2.get_subdomain(num_omegas);
    let omega_n: Vec<Scalar> = (0..num_omegas)
        .map(|i| batch_dom_2n2.get_all_roots_of_unity()[i * 2])
        .collect();

    // Lagrange bases
    let mut lagr_g1 = taus.t1[0..num_omegas].to_vec();
    ifft_assign_g1(&mut lagr_g1, &dom_n1);
    let mut lagr_g2 = taus.t2[0..num_omegas].to_vec();
    ifft_assign_g2(&mut lagr_g2, &dom_n1);

    // Vanishing polynomial that we test h(X) with is (X^{n+1} - 1) / (X - \omega^n)
    //
    // Zhoujun's faster algorithm in Lagrange basis:
    // Let $V(X) = \frac{X^{n+1} - 1}{X - \omega^n}$ denote the vanishing polynomial.

    // Note that the $n$-th Lagrange polynomial (w.r.t. our $(n+1)$-sized FFT evaluation domain) is $\ell_n(X) = \frac{V(X)}{ \prod_{i\in[n)} (\omega^n - \omega^i) }$.

    // Therefore, below we commit to $V(X)$ by simply scaling it down by $\prod_{i\in[n)} (\omega^n - \omega^i)$!
    let vanishing_com = {
        let last_eval: Scalar = (0..n).map(|i| omega_n[n] - omega_n[i]).product();

        lagr_g2[n] * last_eval
    };

    // Alin's slower algorithm in coefficient basis
    //
    // let mut numerator = vec![Scalar::ZERO; num_omegas + 1];
    // numerator[num_omegas] = Scalar::ONE; // X^{n+1}
    // numerator[0] = Scalar::ONE.neg(); // -1
    // let (vanishing_poly, _) = poly_div_xnc(numerator, 1, omega_n[n].neg());
    // let _vanishing_com = g2_multi_exp(
    //     &taus.t2[0..vanishing_poly.len())],
    //     &vanishing_poly,
    // );
    // assert_eq!(_vanishing_com, vanishing_com);

    PublicParameters {
        taus,
        ell,
        n,
        lagr_g1,
        lagr_g2,
        vanishing_com,
        dom_n1,
        batch_dom_n1,
    }
}

pub fn commit<R>(pp: &PublicParameters, z: &[Scalar], rng: &mut R) -> (Commitment, Scalar)
where
    R: RngCore + rand::Rng + CryptoRng,
{
    let r = random_scalar(rng);
    let c = commit_with_randomness(pp, z, &r);
    (c, r)
}

pub(crate) fn commit_with_randomness(
    pp: &PublicParameters,
    z: &[Scalar],
    r: &Scalar,
) -> Commitment {
    let mut scalars = z.to_vec();
    let mut bases: Vec<G1Projective> = pp.lagr_g1[..scalars.len()].to_vec(); // TODO: atm the range proof algorithm couples `r` with `lagr_g1.last()` causing a copy here; this can be avoided by coupling `r` with `lagr_g1.first()` instead

    scalars.push(*r);
    let last_base = pp.lagr_g1.last().expect("pp.lagr_g1 must not be empty");
    bases.push(*last_base);

    let c = g1_multi_exp(&bases, &scalars);
    Commitment(c)
}

#[allow(non_snake_case)]
pub fn batch_prove<R>(
    rng: &mut R,
    pp: &PublicParameters,
    zz: &[Scalar],
    cc: &Commitment,
    rr: &Scalar,
    fs_transcript: &mut merlin::Transcript,
) -> Proof
where
    R: RngCore + rand::Rng + CryptoRng,
{
    let zz = pad_to_pow2_len_minus_one(zz.to_vec());

    assert_eq!(zz.len(), pp.n);
    assert_eq!(pp.taus.t1.len(), pp.n + 1); // g_1, g_1^{tau}, g_1^{tau^2}, ..., g_1^{tau^n}
    assert_eq!(pp.taus.t2.len(), pp.n + 1);

    #[cfg(feature = "range_proof_timing")]
    println!("n = {:?}, ell = {:?}", pp.n, pp.ell);
    #[cfg(feature = "range_proof_timing")]
    let mut cumulative = Duration::ZERO;
    #[cfg(feature = "range_proof_timing")]
    let mut print_cumulative = |duration: Duration| {
        cumulative += duration;
        println!("     \\--> Cumulative time: {:?}", cumulative);
    };

    // Step 1: Convert z_i's to bits.
    #[cfg(feature = "range_proof_timing")]
    let start = Instant::now();

    let bits: Vec<Vec<bool>> = zz
        .iter()
        .map(|z_val| {
            z_val
                .to_bytes_le()
                .into_iter()
                .flat_map(byte_to_bits_le)
                .take(pp.ell)
                .collect::<Vec<_>>()
        })
        .collect();

    #[cfg(feature = "range_proof_timing")]
    {
        let duration = start.elapsed();
        println!(
            "{:>8.2} mus: Chunking {:?} z_i's into bits",
            duration.as_micros().as_f64(),
            pp.n
        );
        print_cumulative(duration);
    }

    assert_eq!(pp.n, bits.len());
    assert_eq!(pp.ell, bits[0].len());

    // Step 2: Sample correlated randomness r_j for each f_j polynomial commitment.
    #[cfg(feature = "range_proof_timing")]
    let start = Instant::now();

    let r = correlated_randomness(rng, 2, pp.ell, &rr);

    #[cfg(feature = "range_proof_timing")]
    {
        let duration = start.elapsed();
        println!(
            "{:>8.2} mus: Correlating {:?} pieces of randomness",
            duration.as_micros().as_f64(),
            pp.ell
        );
        print_cumulative(duration);
    }

    assert_eq!(pp.ell, r.len());

    // Step 3: Compute f_j(X) = \sum_{i=0}^{n-1} z_i[j] \ell_i(X) + r[j] \ell_n(X),
    // where \ell_i(X) is the ith Lagrange polynomial for the (n+1)th roots-of-unity evaluation domain.
    #[cfg(feature = "range_proof_timing")]
    let start = Instant::now();
    // f_evals[j] = the evaluations of f_j(x) at all the (n+1)-th roots of unity.
    //            = (z_0[j], ..., z_{n-1}[j], r[j]), where z_i[j] is the j-th bit of z_i.
    let f_evals = (0..pp.ell)
        .map(|j| {
            (0..pp.n)
                .map(|i| Scalar::from(bits[i][j] as u64))
                .chain(once(r[j]))
                .collect::<Vec<Scalar>>()
        })
        .collect::<Vec<Vec<Scalar>>>();
    // Assert f_evals is either 0 or 1s or r_j
    // for (j, evals) in f_evals.iter().enumerate() {
    //     for (i, e) in evals.iter().take(pp.n).enumerate() {
    //         assert!(e.eq(&Scalar::ZERO) || e.eq(&Scalar::ONE), "f_evals[{}][{}] = {}", j, i, e);
    //     }
    // }
    #[cfg(feature = "range_proof_timing")]
    {
        let duration = start.elapsed();
        println!(
            "{:>8.2} mus: Convert {:?} z_{{i,j}} bits to scalars",
            duration.as_micros().as_f64(),
            pp.ell * pp.n
        );
        print_cumulative(duration);
    }
    // Step 4: Compute c_j = g_1^{f_j(\tau)}
    #[cfg(feature = "range_proof_timing")]
    let start = Instant::now();
    // c[j] = c_j = g_1^{f_j(\tau)}
    let c: Vec<G1Projective> = (0..pp.ell)
        // Note: Using a multiexp will be 10-20% slower than manually multiplying.
        // .map(|j|
        //     g1_multi_exp(&pp.lagrange_basis, &f_evals[j]))
        .map(|j| {
            // TODO(Performance): Can we speed this up with tables? There are `n` bits, so a single
            //  (2^n)-sized table that maps `n` bits into their multiproduct \prod_{i=0}^{n} L_i^{f_j(\omega_i)}
            //  would be too large: e.g., for n = 24 such a table would take 768 MiB.
            //  If we pick a chunk size of `c` bits such that it evenly divides `n`, we would have
            //  `k = n / c` chunks. (Assuming `n` is a power of two for now; can tweak later.)
            //  So we could have `k` tables, each of size 2^c. Each table `j \in[0, k)` maps
            //  exponents into their multiproduct `\prod_{i=j*c}^{(j+1)*c} L_i^{f_j(\omega_i)}`
            //  For example, if we want to handle n = 2048, we can set c = 16, which gives
            //  `k = \ell / c = 2048 / 16 = 128` tables, each of size 2^c => 2^{16} * 48 bytes =
            //  3 MiB / table => 384 MiB total.
            let mut c: G1Projective = pp
                .lagr_g1
                .iter()
                .take(pp.n)
                .zip(f_evals[j].iter().take(pp.n))
                .map(|(lagr, eval)| {
                    // Using G1Projective::mul here will be way slower! (Not sure why...)
                    if eval.is_zero_vartime() {
                        G1Projective::identity()
                    } else {
                        *lagr
                    }
                })
                .sum();
            c.add_assign(pp.lagr_g1[pp.n].mul(&f_evals[j][pp.n]));
            c
        })
        .collect();
    #[cfg(feature = "range_proof_timing")]
    {
        let duration = start.elapsed();
        println!(
            "{:>8.2} mus: All {:?} deg-{:?} f_j G_1 commitments",
            duration.as_micros().as_f64(),
            pp.ell,
            pp.n
        );
        print_cumulative(duration);
        println!("        + Each c_j took: {:?}", duration / pp.ell as u32);
    }

    // Step 5: Compute c_hat[j] = \hat{c}_j = g_2^{f_j(\tau)}
    #[cfg(feature = "range_proof_timing")]
    let start = Instant::now();
    let c_hat: Vec<G2Projective> = (0..pp.ell)
        // Note: Using a multiexp will be 10-20% slower than manually multiplying.
        // .map(|j| g2_multi_exp(&pp.lagrange_basis_g2, &f_evals[j]))
        .map(|j| {
            let mut c_hat_j: G2Projective = pp
                .lagr_g2
                .iter()
                .take(pp.n)
                .zip(f_evals[j].iter().take(pp.n))
                .map(|(lagr, eval)| {
                    // Using G1Projective::mul here will be way slower! (Not sure why...)
                    if eval.is_zero_vartime() {
                        G2Projective::identity()
                    } else {
                        *lagr
                    }
                })
                .sum();
            c_hat_j.add_assign(pp.lagr_g2[pp.n].mul(&f_evals[j][pp.n]));
            c_hat_j
        })
        .collect();
    #[cfg(feature = "range_proof_timing")]
    {
        let duration = start.elapsed();
        println!(
            "{:>8.2} mus: All {:?} deg-{:?} f_j G_2 commitments",
            duration.as_micros().as_f64(),
            pp.ell,
            pp.n
        );
        print_cumulative(duration);
        println!(
            "        + Each \\hat{{c}}_j took: {:?}",
            duration / pp.ell as u32
        );
    }

    let num_omegas = pp.n + 1;

    // Step 6:
    //  1. Compute each f_j(X) in coefficient form via a size-(n+1) FFT on f_j(X)
    //  2. Comptue f'_j(X) via a differentiation.
    //  3. Evaluate f'_j at all (n+1)th roots of unity via a size-(n+1) FFT.
    //  4. \forall i \in [0,n), compute N_j'(\omega^i) = (\omega^i - \omega^n) f_j'(\omega^i)(2f_j(\omega^i) - 1)
    //  5. for i = n, compute N_j'(\omega^n) = r_j(r_j - 1)
    //  6. \forall i \in [0,n], compute h_j(\omega^i) = N_j'(\omega^i) / ( (n+1)\omega^{i n} )
    #[cfg(feature = "range_proof_timing")]
    let start = Instant::now();
    let omega_n = pp.batch_dom_n1.get_root_of_unity(pp.n);
    let n1_inv = Scalar::from(pp.n as u64 + 1).invert().unwrap();
    let mut omega_i_minus_n = Vec::with_capacity(pp.n);
    for i in 0..pp.n {
        let omega_i = pp.batch_dom_n1.get_root_of_unity(i);
        omega_i_minus_n.push(omega_i - omega_n);
    }
    let h: Vec<Vec<Scalar>> = (0..pp.ell)
        .map(|j| {
            // Interpolate f_j coeffs
            let mut f_j = f_evals[j].clone();
            ifft_assign(&mut f_j, &pp.dom_n1);
            assert_eq!(f_j.len(), pp.n + 1);

            // Compute f'_j derivative
            let mut diff_f_j = f_j.clone();
            poly_differentiate(&mut diff_f_j);
            assert_eq!(diff_f_j.len(), pp.n);

            // Evaluate f'_j at all (n+1)th roots of unity
            let mut diff_f_j_evals = diff_f_j.clone();
            fft_assign(&mut diff_f_j_evals, &pp.dom_n1);
            assert_eq!(diff_f_j_evals.len(), pp.n + 1);

            // \forall i \in [0,n), N'_j(\omega^i) = (\omega^i - \omega^n) f_j'(\omega^i)(2f_j(\omega^i) - 1)
            let mut diff_n_j_evals = Vec::with_capacity(num_omegas);
            for i in 0..pp.n {
                diff_n_j_evals.push(
                    (omega_i_minus_n[i])
                        * diff_f_j_evals[i]
                        * (f_evals[j][i].double() - Scalar::ONE),
                );
            }

            // N'_j(\omega^n) = r_j(r_j - 1)
            diff_n_j_evals.push(r[j].square() - r[j]);
            assert_eq!(diff_n_j_evals.len(), num_omegas);

            // \forall i \in [0,n], h_j(\omega^i)
            //  = N_j'(\omega^i) / ( (n+1)\omega^{i n} )
            //  = N_j'(\omega^i) * (\omega^i / (n+1))
            let mut h_j = Vec::with_capacity(num_omegas);
            for i in 0..pp.n + 1 {
                h_j.push(
                    diff_n_j_evals[i]
                        .mul(pp.batch_dom_n1.get_root_of_unity(i))
                        .mul(n1_inv),
                );
            }
            assert_eq!(h_j.len(), num_omegas);

            h_j
        })
        .collect();
    #[cfg(feature = "range_proof_timing")]
    {
        let duration = start.elapsed();
        println!(
            "{:>8.2} mus: All {:?} deg-{:?} h_j(X) coeffs",
            duration.as_micros().as_f64(),
            pp.ell,
            num_omegas - 1
        );
        print_cumulative(duration);
    }
    // Step 7: Fiat-Shamir transform for beta_j's.
    #[cfg(feature = "range_proof_timing")]
    let start = Instant::now();
    // Note: The first output of `fiat_shamir_challenges` is unused, it is intended for the verifier.
    // This is not ideal, but it should not significantly affect performance.
    let vk = (
        &pp.lagr_g1[0],
        &pp.lagr_g2[0],
        &pp.taus.t2[0],
        &pp.vanishing_com,
    );
    let public_statement = (pp.ell, cc);
    let bit_commitments = (c.as_slice(), c_hat.as_slice());
    let (_, betas) = fiat_shamir_challenges(
        &vk,
        &public_statement,
        &bit_commitments,
        c.as_slice().len(),
        fs_transcript,
    ); // TODO: Keeping it at None until discussed
    assert_eq!(pp.ell, betas.len());
    #[cfg(feature = "range_proof_timing")]
    {
        let duration = start.elapsed();
        println!(
            "{:>8.2} mus: {:?} Fiat-Shamir challenges",
            duration.as_micros().as_f64(),
            betas.len()
        );
        print_cumulative(duration);
    }
    // Step 8: Compute h(X) = \sum_{j=0}^{ell-1} beta_j h_j(X)
    #[cfg(feature = "range_proof_timing")]
    let start = Instant::now();
    let mut hh: Vec<Scalar> = vec![Scalar::ZERO; pp.n + 1];
    for j in 0..betas.len() {
        let beta_j_h_j = poly_mul_scalar(&h[j], betas[j]);
        poly_add_assign(&mut hh, &beta_j_h_j);
    }
    assert_eq!(hh.len(), num_omegas);
    #[cfg(feature = "range_proof_timing")]
    {
        let duration = start.elapsed();
        println!(
            "{:>8.2} mus: h(X) as a size-{:?} linear combination of h_j(X)'s",
            duration.as_micros().as_f64(),
            betas.len()
        );
        print_cumulative(duration);
    }

    // Step 9: Compute d = g_1^{h(X)}
    #[cfg(feature = "range_proof_timing")]
    let start = Instant::now();
    let d = g1_multi_exp(&pp.lagr_g1[0..num_omegas], &hh);
    #[cfg(feature = "range_proof_timing")]
    {
        let duration = start.elapsed();
        println!(
            "{:>8.2} mus: deg-{:?} h(X) commitment",
            duration.as_micros().as_f64(),
            hh.len() - 1
        );
        print_cumulative(duration);
    }

    Proof { d, c, c_hat }
}

/// Verifies a batch proof against the given public parameters and commitment.
///
/// Returns `Ok(())` if the proof is valid, or an error otherwise.
pub fn batch_verify(
    pp: &PublicParameters,
    c: &Commitment,
    proof: &Proof,
    fs_transcript: &mut merlin::Transcript,
) -> anyhow::Result<()> {
    // TODO(Perf): Can have these precomputed in pp
    let mut powers_of_two = vec![Scalar::ONE; pp.ell];
    for i in 1..pp.ell {
        let x = powers_of_two[i - 1].double();
        powers_of_two[i] = x;
    }
    ensure!(c.0 == g1_multi_exp(&proof.c, &powers_of_two));

    let vk = (
        &pp.lagr_g1[0],
        &pp.lagr_g2[0],
        &pp.taus.t2[0],
        &pp.vanishing_com,
    );
    let public_statement = (pp.ell, c);
    let bit_commitments = (&proof.c[..], &proof.c_hat[..]);
    let (alphas, betas) = fiat_shamir_challenges(
        &vk,
        &public_statement,
        &bit_commitments,
        proof.c.len(),
        fs_transcript,
    );

    // Verify h(\tau)
    let h_check = multi_pairing_g1_g2(
        (0..pp.ell)
            .map(|j| proof.c[j] * betas[j])
            .chain(once(-proof.d))
            .collect::<Vec<_>>()
            .iter(),
        (0..pp.ell)
            .map(|j| proof.c_hat[j] - pp.taus.t2[0])
            .chain(once(pp.vanishing_com))
            .collect::<Vec<_>>()
            .iter(),
    );
    ensure!(Gt::identity() == h_check);

    // Ensure duality: c[j] matches c_hat[j].
    let c_check = multi_pairing_g1_g2(
        vec![g1_multi_exp(&proof.c, &alphas), -pp.taus.t1[0]].iter(),
        vec![pp.taus.t2[0], g2_multi_exp(&proof.c_hat, &alphas)].iter(),
    );
    ensure!(Gt::identity() == c_check);

    Ok(())
}

fn byte_to_bits_le(val: u8) -> Vec<bool> {
    (0..8).map(|i| (val >> i) & 1 == 1).collect()
}

/// Compute alpha, beta.
fn fiat_shamir_challenges(
    vk: &(&G1Projective, &G2Projective, &G2Projective, &G2Projective),
    public_statement: &(usize, &Commitment),
    bit_commitments: &(&[G1Projective], &[G2Projective]),
    num_scalars: usize,
    fs_transcript: &mut merlin::Transcript,
) -> (Vec<Scalar>, Vec<Scalar>) {
    <merlin::Transcript as fiat_shamir::RangeProof>::append_sep(fs_transcript);

    <merlin::Transcript as fiat_shamir::RangeProof>::append_vk(fs_transcript, vk);

    <merlin::Transcript as fiat_shamir::RangeProof>::append_public_statement(
        fs_transcript,
        public_statement,
    );

    <merlin::Transcript as fiat_shamir::RangeProof>::append_bit_commitments(
        fs_transcript,
        bit_commitments,
    );

    // Generate the Fiat–Shamir challenges from the updated transcript
    let beta_vals =
        <merlin::Transcript as fiat_shamir::RangeProof>::challenge_linear_combination_128bit(
            fs_transcript,
            num_scalars,
        );

    let alpha_vals =
        <merlin::Transcript as fiat_shamir::RangeProof>::challenge_linear_combination_128bit(
            fs_transcript,
            num_scalars,
        );

    (alpha_vals, beta_vals)
}

fn correlated_randomness<R>(
    rng: &mut R,
    radix: u64,
    num_chunks: usize,
    target_sum: &Scalar,
) -> Vec<Scalar>
where
    R: RngCore + rand::Rng + CryptoRng,
{
    let mut r_vals = vec![Scalar::ZERO; num_chunks];
    let mut remaining = *target_sum;
    let radix = Scalar::from(radix);
    let mut cur_base = radix;
    for i in 1..num_chunks {
        r_vals[i] = random_scalar(rng);
        remaining -= r_vals[i] * cur_base;
        cur_base *= radix;
    }
    r_vals[0] = remaining;
    r_vals
}

#[cfg(test)]
mod tests {
    use crate::{
        algebra::polynomials::{poly_div_xnc, poly_eval},
        range_proof::{byte_to_bits_le, correlated_randomness},
        utils::random::{random_scalar, random_scalars},
    };
    use blstrs::Scalar;
    use ff::Field;
    use rand::thread_rng;

    #[test]
    fn test_poly_div_xnc() {
        let mut rng = thread_rng();
        let coefs = random_scalars(10, &mut rng);
        let c = random_scalar(&mut rng);
        let n = 3;
        let (quotient, remainder) = poly_div_xnc(coefs.clone(), n, c);
        assert_eq!(n, remainder.len());
        let x = random_scalar(&mut rng);
        let expected = poly_eval(&coefs, &x);
        let actual =
            (x.pow(&[n as u64]) + c) * poly_eval(&quotient, &x) + poly_eval(&remainder, &x);
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_byte_to_bits_le() {
        assert_eq!(vec![true; 8], byte_to_bits_le(255));
        assert_eq!(
            vec![true, true, true, true, true, true, true, false],
            byte_to_bits_le(127)
        );
        assert_eq!(
            vec![false, true, true, true, true, true, true, true],
            byte_to_bits_le(254)
        );
    }

    #[test]
    fn test_correlated_randomness() {
        let mut rng = thread_rng();
        let target_sum = Scalar::ONE;
        let radix: u64 = 4;
        let num_chunks: usize = 8;
        let coefs = correlated_randomness(&mut rng, radix, num_chunks, &target_sum);
        let actual_sum: Scalar = (0..num_chunks)
            .map(|i| coefs[i] * Scalar::from(radix.pow(i as u32)))
            .sum();
        assert_eq!(target_sum, actual_sum);
    }
}
