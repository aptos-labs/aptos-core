// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{fiat_shamir, utils::pad_to_pow2_len_minus_one};
use anyhow::ensure;
use ark_ec::{pairing::Pairing, AffineRepr, CurveGroup, PrimeGroup, VariableBaseMSM};
use ark_ff::{AdditiveGroup, Field, PrimeField};
use ark_poly::{self, EvaluationDomain, GeneralEvaluationDomain};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::{
    rand::{thread_rng, CryptoRng, RngCore},
    UniformRand,
};
#[cfg(feature = "range_proof_timing")]
use ff::derive::bitvec::macros::internal::funty::Fundamental;
use num_traits::Zero;
#[cfg(feature = "range_proof_timing")]
use std::time::{Duration, Instant};
use std::{
    iter::once,
    ops::{AddAssign, Mul},
};

pub const DST: &[u8; 42] = b"APTOS_UNIVARIATE_DEKART_V1_RANGE_PROOF_DST";

pub struct PowersOfTau<E: Pairing> {
    t1: Vec<E::G1>, // g_1, g_1^{tau}, g_1^{tau^2}, ..., g_1^{tau^n}, where `n` is the batch size
    t2: Vec<E::G2>,
}

pub fn powers_of_tau<E: Pairing, R>(rng: &mut R, n: usize) -> PowersOfTau<E>
where
    R: RngCore + CryptoRng,
{
    let g1 = E::G1::rand(rng);
    let g2 = E::G2::rand(rng);
    let tau = E::ScalarField::rand(rng);
    let mut t1 = vec![g1];
    let mut t2 = vec![g2];
    for i in 0..n {
        t1.push(t1[i] * tau);
        t2.push(t2[i] * tau);
    }
    PowersOfTau { t1, t2 }
}

pub struct PublicParameters<E: Pairing> {
    taus: PowersOfTau<E>,      // g_1, g_1^{tau}, g_1^{tau^2}, ..., g_1^{tau^n}
    ell: usize,                // the range is [0, 2^\ell)
    n: usize,                  // the number of values we are batch proving; i.e., batch size
    lagr_g1: Vec<E::G1Affine>, // of size n + 1
    lagr_g2: Vec<E::G2Affine>, // of size n + 1
    pub vanishing_com: E::G2, // commitment to deg-n vanishing polynomial (X^{n+1} - 1) / (X - \omega^n) used to test h(X)
    eval_dom: GeneralEvaluationDomain<E::ScalarField>,
    roots_of_unity_in_eval_dom: Vec<E::ScalarField>, // setup times are a bit slow, probably because of this?
    powers_of_two: Vec<E::ScalarField>,              // [1, 2, 4, ..., 2^{\ell - 1}]
}

#[derive(CanonicalSerialize, CanonicalDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct Commitment<E: Pairing>(E::G1);

#[derive(CanonicalSerialize, CanonicalDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct Proof<E: Pairing> {
    d: E::G1,          // commitment to h(X) = \sum_{j=0}^{\ell-1} beta_j h_j(X)
    c: Vec<E::G1>,     // of size \ell
    c_hat: Vec<E::G2>, // of size \ell
}

impl<E: Pairing> Proof<E> {
    pub fn maul(&mut self) {
        self.c[0] += E::G1::generator();
    }
}

/// Sets up the Borgeaud range proof for proving that size-`n` batches are in the range [0, 2^\ell).
pub fn setup<E: Pairing>(ell: usize, n: usize) -> PublicParameters<E> {
    let mut rng = thread_rng();

    let n = (n + 1).next_power_of_two() - 1;
    let num_omegas = n + 1;
    debug_assert!(num_omegas.is_power_of_two());

    let taus = powers_of_tau(&mut rng, n); // The taus have length `n+1`

    let eval_dom = GeneralEvaluationDomain::<E::ScalarField>::new(num_omegas).unwrap();
    let roots_of_unity_in_eval_dom: Vec<E::ScalarField> = eval_dom.elements().collect(); // This is probably quite slow

    // Lagrange bases
    let lagr_g1_proj = eval_dom.ifft(&taus.t1);
    let lagr_g2_proj = eval_dom.ifft(&taus.t2);

    let lagr_g1 = E::G1::normalize_batch(&lagr_g1_proj);
    let lagr_g2 = E::G2::normalize_batch(&lagr_g2_proj);

    // Vanishing polynomial that we test h(X) with is (X^{n+1} - 1) / (X - \omega^n)
    //
    // Zhoujun's faster algorithm in Lagrange basis:
    // Let $V(X) = \frac{X^{n+1} - 1}{X - \omega^n}$ denote the vanishing polynomial.

    // Note that the $n$-th Lagrange polynomial (w.r.t. our $(n+1)$-sized FFT evaluation domain) is $\ell_n(X) = \frac{V(X)}{ \prod_{i\in[n)} (\omega^n - \omega^i) }$.

    // Therefore, below we commit to $V(X)$ by simply scaling it down by $\prod_{i\in[n)} (\omega^n - \omega^i)$!
    let vanishing_com = {
        let last_eval: E::ScalarField = (0..n)
            .map(|i| roots_of_unity_in_eval_dom[n] - roots_of_unity_in_eval_dom[i])
            .product();

        lagr_g2_proj[n] * last_eval
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

    let powers_of_two: Vec<E::ScalarField> =
        std::iter::successors(Some(E::ScalarField::ONE), |x| Some(x.double()))
            .take(ell)
            .collect();

    PublicParameters {
        taus,
        ell,
        n,
        lagr_g1,
        lagr_g2,
        vanishing_com,
        eval_dom,
        roots_of_unity_in_eval_dom,
        powers_of_two,
    }
}

pub fn commit<E: Pairing, R>(
    pp: &PublicParameters<E>,
    z: &[E::ScalarField],
    rng: &mut R,
) -> (Commitment<E>, E::ScalarField)
where
    R: RngCore + CryptoRng,
{
    let r = E::ScalarField::rand(rng);
    let c = commit_with_randomness(pp, z, &r);
    (c, r)
}

pub(crate) fn commit_with_randomness<E: Pairing>(
    pp: &PublicParameters<E>,
    z: &[E::ScalarField],
    r: &E::ScalarField,
) -> Commitment<E> {
    let mut scalars = z.to_vec();
    let mut bases = pp.lagr_g1[..scalars.len()].to_vec(); // TODO: atm the range proof algorithm couples `r` with `lagr_g1.last()` causing a copy here; this can be avoided by coupling `r` with `lagr_g1.first()` instead

    scalars.push(*r);
    let last_base = pp.lagr_g1.last().expect("pp.lagr_g1 must not be empty");
    bases.push(*last_base);

    let c = E::G1::msm(&bases, &scalars).expect("could not compute msm in range proof commitment");
    Commitment(c)
}

fn msm_bool<G: AffineRepr>(bases: &[G], scalars: &[bool]) -> G::Group {
    assert_eq!(bases.len(), scalars.len());

    let mut acc = G::Group::zero();
    for (base, &bit) in bases.iter().zip(scalars) {
        if bit {
            acc += base.into_group();
        }
    }
    acc
}

fn scalar_to_bits_le<E: Pairing>(x: &E::ScalarField) -> Vec<bool> {
    let bigint: <E::ScalarField as ark_ff::PrimeField>::BigInt = x.into_bigint();
    ark_ff::BitIteratorLE::new(&bigint).collect()
}

fn differentiate_in_place<F: Field>(coeffs: &mut Vec<F>) {
    let degree = coeffs.len() - 1;
    for i in 0..degree {
        coeffs[i] = coeffs[i + 1].mul(F::from((i + 1) as u64));
    }

    coeffs.truncate(degree);
}

#[allow(non_snake_case)]
pub fn batch_prove<E: Pairing, R>(
    rng: &mut R,
    pp: &PublicParameters<E>,
    zz: &[E::ScalarField],
    cc: &Commitment<E>,
    rr: &E::ScalarField,
    fs_transcript: &mut merlin::Transcript,
) -> Proof<E>
where
    R: RngCore + CryptoRng,
{
    let zz = pad_to_pow2_len_minus_one::<E>(zz.to_vec());

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
            scalar_to_bits_le::<E>(z_val)
                .into_iter()
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

    let r = correlated_randomness(rng, 2, pp.ell, rr);

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
    let f_evals_without_r: Vec<Vec<bool>> = (0..pp.ell)
        .map(|j| bits.iter().map(|row| row[j]).collect())
        .collect(); // This is just transposing the bits matrix
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
    // let bits_flattened: Vec<bool> = bits.into_iter().flatten().collect();
    let c: Vec<E::G1> = (0..pp.ell)
        // Note on blstrs: Using a multiexp will be 10-20% slower than manually multiplying.
        // .map(|j|
        //     g1_multi_exp(&pp.lagrange_basis, &f_evals[j]))
        // TODO: Whereas has msm's for specific scalar chunk sizes........
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
            let mut c_j: <E as Pairing>::G1 = msm_bool(&pp.lagr_g1[..pp.n], &f_evals_without_r[j]);
            c_j.add_assign(pp.lagr_g1[pp.n].mul(&r[j]));
            c_j
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
    let c_hat: Vec<E::G2> = (0..pp.ell)
        // Note: Using a multiexp will be 10-20% slower than manually multiplying.
        // .map(|j| g2_multi_exp(&pp.lagrange_basis_g2, &f_evals[j]))
        .map(|j| {
            let mut c_hat_j: <E as Pairing>::G2 =
                msm_bool(&pp.lagr_g2[..pp.n], &f_evals_without_r[j]);
            c_hat_j.add_assign(pp.lagr_g2[pp.n].mul(&r[j]));
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
    //  2. Compute f'_j(X) via a differentiation.
    //  3. Evaluate f'_j at all (n+1)th roots of unity via a size-(n+1) FFT.
    //  4. \forall i \in [0,n), compute N_j'(\omega^i) = (\omega^i - \omega^n) f_j'(\omega^i)(2f_j(\omega^i) - 1)
    //  5. for i = n, compute N_j'(\omega^n) = r_j(r_j - 1)
    //  6. \forall i \in [0,n], compute h_j(\omega^i) = N_j'(\omega^i) / ( (n+1)\omega^{i n} )
    #[cfg(feature = "range_proof_timing")]
    let start = Instant::now();
    let omega_n = pp.eval_dom.element(pp.n); // let omega_n = pp.all_roots_of_unity(pp.n)
    let n1_inv = E::ScalarField::from(pp.n as u64 + 1).inverse().unwrap();
    let mut omega_i_minus_n = Vec::with_capacity(pp.n);
    for i in 0..pp.n {
        let omega_i = pp.roots_of_unity_in_eval_dom[i];
        omega_i_minus_n.push(omega_i - omega_n);
    }

    let f_evals: Vec<Vec<E::ScalarField>> = f_evals_without_r
        .iter()
        .enumerate()
        .map(|(j, col)| {
            col.iter()
                .map(|&b| E::ScalarField::from(b))
                .chain(once(r[j]))
                .collect()
        })
        .collect();

    let h: Vec<Vec<E::ScalarField>> = (0..pp.ell)
        .map(|j| {
            // Interpolate f_j coeffs
            let mut f_j = f_evals[j].clone();
            pp.eval_dom.ifft_in_place(&mut f_j);
            assert_eq!(f_j.len(), pp.n + 1);

            // Compute f'_j derivative
            let mut diff_f_j = f_j.clone();
            differentiate_in_place(&mut diff_f_j);
            assert_eq!(diff_f_j.len(), pp.n);

            // Evaluate f'_j at all (n+1)th roots of unity
            let mut diff_f_j_evals = diff_f_j.clone();
            pp.eval_dom.fft_in_place(&mut diff_f_j_evals);
            assert_eq!(diff_f_j_evals.len(), pp.n + 1);

            // \forall i \in [0,n), N'_j(\omega^i) = (\omega^i - \omega^n) f_j'(\omega^i)(2f_j(\omega^i) - 1)
            let mut diff_n_j_evals = Vec::with_capacity(num_omegas);
            for i in 0..pp.n {
                diff_n_j_evals.push(
                    (omega_i_minus_n[i])
                        * diff_f_j_evals[i]
                        * (f_evals[j][i].double() - E::ScalarField::ONE),
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
                        .mul(pp.roots_of_unity_in_eval_dom[i])
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
        &pp.taus.t1[0],
        &pp.taus.t2[0],
        &pp.taus.t2[1],
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
    );
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
    let mut hh: Vec<E::ScalarField> = vec![E::ScalarField::ZERO; pp.n + 1];
    for (h_j, &beta_j) in h.iter().zip(&betas) {
        for (hh_coeff, &h_coeff) in hh.iter_mut().zip(h_j) {
            *hh_coeff += h_coeff * beta_j;
        }
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
    let d = VariableBaseMSM::msm(&pp.lagr_g1[0..num_omegas], &hh).expect("Failed computing msm");
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
pub fn batch_verify<E: Pairing>(
    pp: &PublicParameters<E>,
    c: &Commitment<E>,
    proof: &Proof<E>,
    fs_transcript: &mut merlin::Transcript,
) -> anyhow::Result<()> {
    let commitment_decomp_affine: Vec<E::G1Affine> = E::G1::normalize_batch(&proof.c); // proof.c.iter().map(|p| p.into_affine()).collect();
        
    let commitment_recomputed: E::G1 =
        VariableBaseMSM::msm(&commitment_decomp_affine, &pp.powers_of_two)
            .expect("Failed to compute msm");
    ensure!(c.0 == commitment_recomputed);

    let vk = (
        &pp.taus.t1[0],
        &pp.taus.t2[0],
        &pp.taus.t2[1],
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
    let h_check = E::multi_pairing(
        (0..pp.ell)
            .map(|j| proof.c[j] * betas[j]) // E::G1
            .chain(once(-proof.d)) // add -d
            .collect::<Vec<_>>(), // collect into Vec<E::G1>
        (0..pp.ell)
            .map(|j| proof.c_hat[j] - pp.taus.t2[0]) // E::G2
            .chain(once(pp.vanishing_com)) // add vanishing commitment
            .collect::<Vec<_>>(), // collect into Vec<E::G2>
    );
    ensure!(E::TargetField::ONE == h_check.0);

    // Ensure duality: c[j] matches c_hat[j].

    // Compute MSM in G1: sum_j (alphas[j] * proof.c[j])
    let g1_comb = VariableBaseMSM::msm(
        &proof
            .c
            .iter()
            .map(|p| p.into_affine())
            .collect::<Vec<E::G1Affine>>(),
        &alphas, // <-- keep them as Fr
    )
    .unwrap();

    // Compute MSM in G2: sum_j (alphas[j] * proof.c_hat[j])
    let g2_comb = VariableBaseMSM::msm(
        &proof
            .c_hat
            .iter()
            .map(|p| p.into_affine())
            .collect::<Vec<E::G2Affine>>(),
        &alphas,
    )
    .unwrap();
    let c_check = E::multi_pairing(
        vec![
            g1_comb,        // from MSM in G1
            -pp.taus.t1[0], // subtract tau_1
        ],
        vec![
            pp.taus.t2[0], // tau_2
            g2_comb,       // from MSM in G2
        ],
    );
    ensure!(E::TargetField::ONE == c_check.0);

    Ok(())
}

/// Compute alpha, beta.
fn fiat_shamir_challenges<E: Pairing>(
    vk: &(&E::G1, &E::G2, &E::G2, &E::G2),
    public_statement: &(usize, &Commitment<E>),
    bit_commitments: &(&[E::G1], &[E::G2]),
    num_scalars: usize,
    fs_transcript: &mut merlin::Transcript,
) -> (Vec<E::ScalarField>, Vec<E::ScalarField>) {
    <merlin::Transcript as fiat_shamir::RangeProof<E>>::append_sep(fs_transcript, DST);

    <merlin::Transcript as fiat_shamir::RangeProof<E>>::append_vk(fs_transcript, vk);

    <merlin::Transcript as fiat_shamir::RangeProof<E>>::append_public_statement(
        fs_transcript,
        public_statement,
    );

    <merlin::Transcript as fiat_shamir::RangeProof<E>>::append_bit_commitments(
        fs_transcript,
        bit_commitments,
    );

    // Generate the Fiat–Shamir challenges from the updated transcript
    let beta_vals =
        <merlin::Transcript as fiat_shamir::RangeProof<E>>::challenge_linear_combination_128bit(
            fs_transcript,
            num_scalars,
        );

    let alpha_vals =
        <merlin::Transcript as fiat_shamir::RangeProof<E>>::challenge_linear_combination_128bit(
            fs_transcript,
            num_scalars,
        );

    (alpha_vals, beta_vals)
}

/// Generate correlated random values whose weighted sum equals `target_sum`.
///
/// Returns `num_chunks` field elements `[r_0, ..., r_{num_chunks-1}]` such that:
/// `r_0 + r_1 * radix + r_2 * radix^2 + ... + r_{num_chunks-1} * radix^{num_chunks-1} = target_sum`.
pub fn correlated_randomness<F, R>(
    rng: &mut R,
    radix: u64,
    num_chunks: usize,
    target_sum: &F,
) -> Vec<F>
where
    F: Field + UniformRand,
    R: RngCore + CryptoRng,
{
    let mut r_vals = vec![F::zero(); num_chunks];
    let mut remaining = *target_sum;
    let radix_f = F::from(radix);
    let mut cur_base = radix_f;

    for i in 1..num_chunks {
        r_vals[i] = F::rand(rng);
        remaining -= r_vals[i] * cur_base;
        cur_base *= radix_f;
    }
    r_vals[0] = remaining;

    r_vals
}

#[cfg(test)]
mod tests {
    use crate::range_proofs::univariate_range_proof::correlated_randomness;
    use ark_ff::Field;
    use ark_std::rand::thread_rng;

    #[cfg(test)]
    fn test_correlated_randomness_generic<F: Field>() {
        let mut rng = thread_rng();
        let target_sum = F::one();
        let radix: u64 = 4;
        let num_chunks: usize = 8;

        let coefs = correlated_randomness(&mut rng, radix, num_chunks, &target_sum);

        // Compute actual sum: Σ coef[i] * radix^i
        let actual_sum: F = (0..num_chunks)
            .map(|i| coefs[i] * F::from(radix.pow(i as u32)))
            .sum();

        assert_eq!(target_sum, actual_sum);
    }

    #[test]
    fn test_correlated_randomness_bn254() {
        use ark_bn254::Fr;
        test_correlated_randomness_generic::<Fr>();
    }
}
