// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::iter::once;
use anyhow::ensure;
use blstrs::{G1Projective, G2Projective, Gt, Scalar};
use ff::Field;
use group::Group;
use rand_core::{CryptoRng, RngCore};
use crate::algebra::evaluation_domain::BatchEvaluationDomain;
use crate::algebra::fft::{fft_assign, ifft_assign, ifft_assign_g1, ifft_assign_g2};
use crate::utils::{g1_multi_exp, g2_multi_exp, multi_pairing_g1_g2};
use crate::utils::random::{random_g1_point, random_g2_point, random_scalar};

pub struct PowersOfTauValues {
    g1_values: Vec<G1Projective>,
    g2_values: Vec<G2Projective>,
}

pub fn powers_of_tau<R>(rng: &mut R, n: usize) -> PowersOfTauValues
where
    R: RngCore + rand::Rng + CryptoRng,
{
    let g1 = random_g1_point(rng);
    let g2 = random_g2_point(rng);
    let tau = random_scalar(rng);
    let mut g1_values = vec![g1];
    let mut g2_values = vec![g2];
    for i in 0..n {
        g1_values.push(g1_values[i] * tau);
        g2_values.push(g2_values[i] * tau);
    }
    PowersOfTauValues {
        g1_values,
        g2_values,
    }
}

pub struct PubParams {
    ptau: PowersOfTauValues,
    num_chunks: usize, // e.g., if 6, we prove for range [0,2^6).
    batch_size: usize, // the $n$.
    lagrange_basis: Vec<G1Projective>, // size: batch_size + 1
    lagrange_basis_g2: Vec<G2Projective>, // size: batch_size + 1
    pub denom_com_g2: G2Projective,
    dom: BatchEvaluationDomain,
}

pub struct Commitment (G1Projective);

pub struct ProverState {
    r: Scalar,
}

#[derive(Clone)]
pub struct Proof {
    d: G1Projective,
    c: Vec<G1Projective>,
    c_hat: Vec<G2Projective>,
}

impl Proof {
    #[cfg(test)]
    fn maul(&mut self) {
        self.c[0] += G1Projective::generator();
    }
}

pub fn setup(ptau: PowersOfTauValues, num_chunks: usize, batch_size: usize) -> PubParams {
    let num_omegas = batch_size + 1;
    assert!(num_omegas.is_power_of_two());

    let dom = BatchEvaluationDomain::new(num_omegas * 2);
    let dom_n = dom.get_subdomain(num_omegas);
    let powers_of_omega: Vec<Scalar> = (0..num_omegas).map(|i|dom.get_all_roots_of_unity()[i*2]).collect();

    // Lagrange basis.
    let mut lagrange_basis = ptau.g1_values[0..num_omegas].to_vec();
    ifft_assign_g1(&mut lagrange_basis, &dom_n);
    let mut lagrange_basis_g2 = ptau.g2_values[0..num_omegas].to_vec();
    ifft_assign_g2(&mut lagrange_basis_g2, &dom_n);

    // demon(x) = (x^num_omegas-1) / (x-rho)
    let denom_com_g2 = {
        let last_eval: Scalar = (0..num_omegas-1).map(|i|powers_of_omega[num_omegas-1] - powers_of_omega[i]).product();
        let evals: Vec<Scalar> = (0..num_omegas-1).map(|_|Scalar::ZERO).chain(once(last_eval)).collect();
        g2_multi_exp(&lagrange_basis_g2, &evals)
    };

    PubParams {
        ptau,
        num_chunks,
        batch_size,
        lagrange_basis,
        lagrange_basis_g2,
        denom_com_g2,
        dom,
    }
}

pub fn commit<R>(pp: &PubParams, z: &[Scalar], rng: &mut R) -> (Commitment, ProverState)
where R: RngCore + rand::Rng + CryptoRng,
{
    let mut scalars = z.to_vec();
    let r = random_scalar(rng);
    scalars.push(r);
    let c = g1_multi_exp(&pp.lagrange_basis, &scalars);
    let prover_state = ProverState { r };
    (Commitment(c), prover_state)
}

/// Divide `f(x)` by `x^n+c`. Polys are in coef repr, least significant coef first.
fn poly_div_xnc(mut coefs: Vec<Scalar>, n: usize, c: Scalar) -> (Vec<Scalar>, Vec<Scalar>) {
    let max_degree = coefs.len() - 1 - n;
    let mut quotient = vec![Scalar::ZERO; max_degree + 1];
    for i in (n..coefs.len()).rev() {
        let coef = coefs.pop().unwrap();
        quotient[i-n] = coef;
        coefs[i-n] -= c*coef;
    }
    (quotient, coefs)
}

pub fn batch_prove<R>(rng: &mut R, pp: &PubParams, z: &[Scalar], com: &Commitment, pcr: &ProverState) -> Proof
where R: RngCore + rand::Rng + CryptoRng,
{
    let num_omegas = pp.batch_size + 1;
    let z_bit_vecs: Vec<Vec<bool>> = z.iter().map(|z_val|{
        z_val
            .to_bytes_le()
            .into_iter().flat_map(byte_to_bits_le)
            .take(pp.num_chunks)
            .collect::<Vec<_>>()
    }).collect();
    assert_eq!(pp.batch_size, z_bit_vecs.len());
    assert_eq!(pp.num_chunks, z_bit_vecs[0].len());

    let r_vals = correlated_randomness(rng, 2, pp.num_chunks, &pcr.r);
    assert_eq!(pp.num_chunks, r_vals.len());
    let c: Vec<G1Projective> = (0..pp.num_chunks).map(|j|{
        let scalars: Vec<Scalar> = (0..pp.batch_size).map(|idx|Scalar::from(z_bit_vecs[idx][j] as u64)).chain(once(r_vals[j])).collect();
        g1_multi_exp(&pp.lagrange_basis, &scalars)
    }).collect();

    let c_hat: Vec<G2Projective> = (0..pp.num_chunks).map(|j|{
        let scalars: Vec<Scalar> = (0..pp.batch_size).map(|idx|Scalar::from(z_bit_vecs[idx][j] as u64)).chain(once(r_vals[j])).collect();
        g2_multi_exp(&pp.lagrange_basis_g2, &scalars)
    }).collect();
    let dom_n = pp.dom.get_subdomain(num_omegas);
    let dom_2n = pp.dom.get_subdomain(num_omegas * 2);

    // h_j(x) = f_j(x) * [f_j(x) - 1] * [x - omega^{n-1}] / (x^n - 1)
    let h_coefs: Vec<Vec<Scalar>> = (0..pp.num_chunks).map(|j| {
        let mut poly_f: Vec<Scalar> = (0..pp.batch_size).map(|i|Scalar::from(z_bit_vecs[i][j] as u64)).chain(once(r_vals[j])).collect();
        // poly_f in eval-at-n-roots repr
        ifft_assign(&mut poly_f, &dom_n);
        // poly_f in coef repr
        poly_f.extend(vec![Scalar::ZERO; num_omegas]);

        fft_assign(&mut poly_f, &dom_2n);
        // poly_f in eval-at-2n-roots repr

        // numerator = f(x)*[f(x)-1]
        // numerator_plus = numerator*(x-omega^{n-1})
        let mut numerator_plus: Vec<Scalar> = poly_f.into_iter().enumerate().map(|(i, eval)|{
            eval * (eval - Scalar::ONE) * (pp.dom.get_all_roots_of_unity()[i] - pp.dom.get_all_roots_of_unity()[2*num_omegas-2])
        }).collect();
        // numerator in eval-at-2n-roots repr
        ifft_assign(&mut numerator_plus, &dom_2n);

        // numerator in coef repr
        // apply denominator (x^n-1)
        let (quotient, remainder) = poly_div_xnc(numerator_plus, num_omegas, -Scalar::ONE);
        // quotient in coef repr
        assert_eq!(num_omegas, quotient.len());
        for item in remainder {
            assert_eq!(Scalar::ZERO, item);
        }
        quotient
    }).collect();

    let (_alpha_vals, beta_vals) = fiat_shamir_challenges(&com, c.as_slice(), c_hat.as_slice());
    assert_eq!(pp.num_chunks, beta_vals.len());

    // h_agg_coefs[j] = beta[0]*h_coefs[0][j] + ... + beta[n-1]*h_coefs[n-1][j].
    let h_agg_coefs: Vec<Scalar> = (0..num_omegas).map(|i|{
        (0..pp.num_chunks).map(|j| beta_vals[j] * h_coefs[j][i]).sum()
    }).collect();

    let d = g1_multi_exp(&pp.ptau.g1_values[0..num_omegas], &h_agg_coefs);
    Proof {
        d,
        c,
        c_hat,
    }
}

pub fn batch_verify(pp: &PubParams, com: &Commitment, proof: &Proof) -> anyhow::Result<()> {

    let mut scalars = vec![Scalar::ONE; pp.num_chunks];
    for i in 1..pp.num_chunks {
        let x = scalars[i-1].double();
        scalars[i] = x;
    }
    ensure!(com.0 == g1_multi_exp(&proof.c, &scalars));

    // Ensure duality: c[j] matches c_hat[j].
    let (alpha_vals, beta_vals) = fiat_shamir_challenges(&com, &proof.c, &proof.c_hat);
    let check_1 = multi_pairing_g1_g2(
        vec![g1_multi_exp(&proof.c, &alpha_vals), -pp.ptau.g1_values[0]].iter(),
        vec![pp.ptau.g2_values[0], g2_multi_exp(&proof.c_hat, &alpha_vals)].iter(),
    );
    ensure!(Gt::identity() == check_1);

    // Verify h(tau).
    let check_2 = multi_pairing_g1_g2(
        (0..pp.num_chunks).map(|j|proof.c[j] * beta_vals[j])
            .chain(once(-proof.d))
            .collect::<Vec<_>>().iter(),
        (0..pp.num_chunks).map(|j|proof.c_hat[j] - pp.ptau.g2_values[0])
            .chain(once(pp.denom_com_g2))
            .collect::<Vec<_>>().iter(),
    );
    ensure!(Gt::identity() == check_2);
    Ok(())
}

fn byte_to_bits_le(val: u8) -> Vec<bool> {
    (0..8).map(|i| (val >> i) & 1 == 1).collect()
}

/// Compute alpha, beta.
fn fiat_shamir_challenges(_com: &Commitment, c: &[G1Projective], c_hat: &[G2Projective]) -> (Vec<Scalar>, Vec<Scalar>) {
    assert_eq!(c.len(), c_hat.len());
    //TODO: real fiat-shamir.
    let alpha_vals = (0..c.len()).map(|j|Scalar::from(2 * j as u64)).collect();
    let beta_vals = (0..c.len()).map(|j|Scalar::from((2 * j + 1) as u64)).collect();
    (alpha_vals, beta_vals)
}

fn correlated_randomness<R>(rng: &mut R, radix: u64, num_chunks: usize, target_sum: &Scalar) -> Vec<Scalar>
where R: RngCore + rand::Rng + CryptoRng,
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
    use blstrs::Scalar;
    use ff::Field;
    use rand::thread_rng;
    use rand_core::RngCore;
    use crate::algebra::polynomials::poly_eval;
    use crate::range_proof::{batch_prove, batch_verify, byte_to_bits_le, commit, correlated_randomness, poly_div_xnc, powers_of_tau, setup};
    use crate::utils::random::{random_scalar, random_scalars};

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
        let actual = (x.pow(&[n as u64]) + c) * poly_eval(&quotient, &x) + poly_eval(&remainder, &x);
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_byte_to_bits_le() {
        assert_eq!(vec![true; 8], byte_to_bits_le(255));
        assert_eq!(vec![true, true, true, true, true, true, true, false], byte_to_bits_le(127));
        assert_eq!(vec![false, true, true, true, true, true, true, true], byte_to_bits_le(254));
    }

    #[test]
    fn test_correlated_randomness() {
        let mut rng = thread_rng();
        let target_sum = Scalar::ONE;
        let radix: u64 = 4;
        let num_chunks: usize = 8;
        let coefs = correlated_randomness(&mut rng, radix, num_chunks, &target_sum);
        let actual_sum: Scalar = (0..num_chunks).map(|i| coefs[i] * Scalar::from(radix.pow(i as u32) as u64)).sum();
        assert_eq!(target_sum, actual_sum);
    }


    #[test]
    fn completeness() {
        let mut rng = thread_rng();
        let num_chunks = 32; // we prove z < 2^64.
        let batch_size = 8191;
        let n_ptau_required = batch_size + 2;
        let ptau = powers_of_tau(&mut rng, n_ptau_required);
        println!("ptau finished, setup starting");
        let pp = setup(ptau, num_chunks, batch_size);
        println!("setup finished, prove starting");
        let z_vals: Vec<Scalar> = (0..batch_size).map(|_| {
            let val = rng.next_u64() >> (64 - num_chunks);
            Scalar::from(val)
        }).collect();
        let (com, prover_state) = commit(&pp, &z_vals, &mut rng);
        let proof = batch_prove(&mut rng, &pp, &z_vals, &com, &prover_state);
        println!("prove finished, vrfy1 starting");
        batch_verify(&pp, &com, &proof).unwrap();
        println!("vrfy finished, vrfy2 starting");
        let mut invalid_proof = proof.clone();
        invalid_proof.maul();
        assert!(batch_verify(&pp, &com, &invalid_proof).is_err())
    }
}
