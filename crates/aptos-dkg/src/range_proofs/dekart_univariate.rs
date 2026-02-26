// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    algebra::polynomials, fiat_shamir, pcs::univariate_kzg,
    pvss::chunky::chunked_elgamal::correlated_randomness, range_proofs::traits,
    sigma_protocol::homomorphism::Trait, utils, Scalar,
};
use anyhow::ensure;
use aptos_crypto::arkworks::{powers_of_two, random::sample_field_element, GroupGenerators};
use ark_ec::{
    pairing::{Pairing, PairingOutput},
    CurveGroup, PrimeGroup, VariableBaseMSM,
};
use ark_ff::{AdditiveGroup, Field};
use ark_poly::{self, EvaluationDomain, Radix2EvaluationDomain};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize, Compress, SerializationError};
#[cfg(feature = "range_proof_timing")]
use ff::derive::bitvec::macros::internal::funty::Fundamental;
use rand::{CryptoRng, RngCore};
#[cfg(feature = "range_proof_timing")]
use std::time::{Duration, Instant};
use std::{
    io::Write,
    iter::once,
    ops::{AddAssign, Mul},
};

// WARNING: This scheme is deprecated, probably not ZK, do not use

pub const DST: &[u8; 42] = b"APTOS_UNIVARIATE_DEKART_V1_RANGE_PROOF_DST";

#[derive(CanonicalSerialize, CanonicalDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct Proof<E: Pairing> {
    d: E::G1,                // commitment to h(X) = \sum_{j=0}^{\ell-1} beta_j h_j(X)
    c: Vec<E::G1Affine>,     // of size \ell
    c_hat: Vec<E::G2Affine>, // of size \ell
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PowersOfTau<E: Pairing> {
    t1: Vec<E::G1>, // g_1, g_1^{tau}, g_1^{tau^2}, ..., g_1^{tau^max_n}, where `max_n` is the maximum batch size
    t2: Vec<E::G2>, // TODO: Should probably use E::G1Affine instead?
}

pub fn powers_of_tau<E: Pairing, R>(
    group_generators: GroupGenerators<E>,
    rng: &mut R,
    n: usize,
) -> PowersOfTau<E>
where
    R: RngCore + CryptoRng,
{
    let tau: E::ScalarField = sample_field_element(rng);
    let mut t1 = vec![group_generators.g1.into()];
    let mut t2 = vec![group_generators.g2.into()];
    for i in 0..n {
        t1.push(t1[i] * tau);
        t2.push(t2[i] * tau);
    }
    PowersOfTau { t1, t2 }
}

#[derive(CanonicalSerialize, CanonicalDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct Commitment<E: Pairing>(E::G1);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProverKey<E: Pairing> {
    max_n: usize,
    max_ell: u8,
    taus: PowersOfTau<E>,      // g_1, g_1^{tau}, g_1^{tau^2}, ..., g_1^{tau^n},
    lagr_g1: Vec<E::G1Affine>, // of size n + 1
    lagr_g2: Vec<E::G2Affine>, // of size n + 1
    eval_dom: Radix2EvaluationDomain<E::ScalarField>,
    roots_of_unity_in_eval_dom: Vec<E::ScalarField>,
    roots_of_unity_minus_one: Vec<E::ScalarField>, // [omega - 1, ..., omega^n - 1]
    vk: VerificationKey<E>,                        // Needed for Fiat-Shamir
}

#[derive(CanonicalSerialize)]
pub struct PublicStatement<E: Pairing> {
    n: usize,
    ell: u8,
    comm: Commitment<E>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationKey<E: Pairing> {
    max_ell: u8,
    tau_1: E::G1, // TODO: should make this stuff affine
    tau_2: E::G2,
    vanishing_com: E::G2, // commitment to deg-n vanishing polynomial (X^{n+1} - 1) / (X - 1) used to test h(X)
    powers_of_two: Vec<E::ScalarField>, // [1, 2, 4, ..., 2^{max_ell - 1}]
}

impl<E: Pairing> CanonicalSerialize for VerificationKey<E> {
    fn serialize_with_mode<W: Write>(
        &self,
        mut writer: W,
        compress: Compress,
    ) -> Result<(), SerializationError> {
        self.max_ell.serialize_with_mode(&mut writer, compress)?;
        self.tau_1.serialize_with_mode(&mut writer, compress)?;
        self.tau_2.serialize_with_mode(&mut writer, compress)?;
        self.vanishing_com
            .serialize_with_mode(&mut writer, compress)?;
        // NOTE: powers_of_two is intentionally not serialized
        Ok(())
    }

    fn serialized_size(&self, compress: Compress) -> usize {
        let mut size = 0;
        size += self.max_ell.serialized_size(compress);
        size += self.tau_1.serialized_size(compress);
        size += self.tau_2.serialized_size(compress);
        size += self.vanishing_com.serialized_size(compress);
        size
    }
}

impl<E: Pairing> traits::BatchedRangeProof<E> for Proof<E> {
    type Commitment = Commitment<E>;
    type CommitmentKey = ProverKey<E>;
    type CommitmentRandomness = Scalar<E::ScalarField>;
    type Input = E::ScalarField;
    type ProverKey = ProverKey<E>;
    type PublicStatement = PublicStatement<E>;
    type VerificationKey = VerificationKey<E>;

    const DST: &[u8] = DST;

    fn commitment_key_from_prover_key(pk: &Self::ProverKey) -> Self::CommitmentKey {
        pk.clone()
    }

    // The main bottlenecks are `powers_of_tau` and the IFFT steps.
    fn setup<R: RngCore + CryptoRng>(
        max_n: usize,
        max_ell: u8,
        group_generators: GroupGenerators<E>,
        rng: &mut R,
    ) -> (ProverKey<E>, VerificationKey<E>) {
        let max_n = (max_n + 1).next_power_of_two() - 1;
        let num_omegas = max_n + 1;
        debug_assert!(num_omegas.is_power_of_two());

        let taus = powers_of_tau(group_generators, rng, max_n); // The taus have length `max_n+1`

        let eval_dom = Radix2EvaluationDomain::<E::ScalarField>::new(num_omegas)
            .expect("Could not construct evaluation domain");
        let roots_of_unity_in_eval_dom: Vec<E::ScalarField> = eval_dom.elements().collect();
        let roots_of_unity_minus_one: Vec<_> = roots_of_unity_in_eval_dom
            .iter()
            .skip(1) // skip index 0
            .map(|&omega| omega - E::ScalarField::ONE)
            .collect();

        // Lagrange bases
        let lagr_g1_proj = eval_dom.ifft(&taus.t1);
        let lagr_g2_proj = eval_dom.ifft(&taus.t2);

        let lagr_g1 = E::G1::normalize_batch(&lagr_g1_proj);
        let lagr_g2 = E::G2::normalize_batch(&lagr_g2_proj);

        // Vanishing polynomial that we test h(X) with is (X^{n+1} - 1) / (X - 1)
        //
        // Zhoujun's faster algorithm in Lagrange basis:
        // Let $V(X) = \frac{X^{n+1} - 1}{X - 1}$ denote the vanishing polynomial.

        // Note that the $0$-th Lagrange polynomial (w.r.t. our $(n+1)$-sized FFT evaluation domain) is $\ell_0(X) = \frac{V(X)}{ \prod_{i > 0} (1 - \omega^i) }$.

        // Therefore, we can commit to $V(X)$ by simply scaling it down by $\prod_{i > 0} (1 - \omega^i)$!

        // Notice that $\prod_{i > 0} (1 - \omega^i)$ is the evaluation of (X^{n+1} - 1) / (X - 1) = 1 + X + ... + X^n at X = 1, which is just n + 1.
        let vanishing_com = { lagr_g2_proj[0] * E::ScalarField::from((max_n + 1) as u64) };

        let powers_of_two = powers_of_two(max_ell as usize);

        let vk = VerificationKey {
            max_ell,
            tau_1: taus.t1[0],
            tau_2: taus.t2[0],
            vanishing_com,
            powers_of_two,
        };

        let pk = ProverKey {
            max_n,
            max_ell,
            taus,
            lagr_g1,
            lagr_g2,
            eval_dom,
            roots_of_unity_in_eval_dom,
            roots_of_unity_minus_one,
            vk: vk.clone(),
        };

        (pk, vk)
    }

    fn commit_with_randomness(
        pk: &Self::ProverKey,
        values: &[Self::Input],
        r: &Self::CommitmentRandomness,
    ) -> Commitment<E> {
        let kzg_commit_hom: univariate_kzg::Homomorphism<'_, E> = univariate_kzg::Homomorphism {
            lagr_g1: &pk.lagr_g1,
        };

        let input = (r.0, values.to_vec());

        Commitment(kzg_commit_hom.apply(&input).0)
    }

    #[allow(non_snake_case)]
    fn prove<R>(
        pk: &ProverKey<E>,
        values: &[Self::Input],
        ell: u8,
        comm: &Self::Commitment,
        r: &Self::CommitmentRandomness,
        rng: &mut R,
    ) -> Proof<E>
    where
        R: rand_core::RngCore + rand_core::CryptoRng,
    {
        let mut fs_transcript = merlin::Transcript::new(Self::DST);

        let n = values.len();
        assert!(
            n <= pk.max_n,
            "n (got {}) must be ≤ max_n (which is {})",
            n,
            pk.max_n
        );
        assert!(
            ell <= pk.max_ell,
            "ell (got {}) must be ≤ max_ell (which is {})",
            ell,
            pk.max_ell
        );

        let mut zz = values.to_vec();
        zz.resize(pk.max_n, E::ScalarField::ZERO);

        debug_assert_eq!(zz.len(), pk.max_n);
        assert_eq!(pk.taus.t1.len(), pk.max_n + 1); // g_1, g_1^{tau}, g_1^{tau^2}, ..., g_1^{tau^max_n}
        assert_eq!(pk.taus.t2.len(), pk.max_n + 1);

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
                utils::scalar_to_bits_le::<E>(z_val)
                    .into_iter()
                    .take(ell as usize)
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

        assert_eq!(pk.max_n, bits.len());
        assert_eq!(ell as usize, bits[0].len());

        // Step 2: Sample correlated randomness r_j for each f_j polynomial commitment.
        #[cfg(feature = "range_proof_timing")]
        let start = Instant::now();

        let r = correlated_randomness(rng, 2, ell.into(), &r.0);

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

        assert_eq!(ell as usize, r.len());

        // Step 3: Compute f_j(X) = \sum_{i=0}^{n-1} z_i[j] \ell_i(X) + r[j] \ell_n(X),
        // where \ell_i(X) is the ith Lagrange polynomial for the (n+1)th roots-of-unity evaluation domain.
        #[cfg(feature = "range_proof_timing")]
        let start = Instant::now();
        // f_evals[j] = the evaluations of f_j(x) at all the (n+1)-th roots of unity.
        //            = (r[j], z_0[j], ..., z_{n-1}[j]), where z_i[j] is the j-th bit of z_i.
        let f_evals_without_r: Vec<Vec<bool>> = (0..ell as usize)
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
        let c: Vec<E::G1> = (0..ell as usize)
            // Note on blstrs: Using a multiexp will be 10-20% slower than manually multiplying.
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
                let mut c_j: <E as Pairing>::G1 = pk.lagr_g1[0].mul(&r[j]); // start with r[j] * lagr_g1[0]
                c_j.add_assign(&utils::msm_bool(
                    &pk.lagr_g1[1..=pk.max_n], // TODO: why are we padding?
                    &f_evals_without_r[j],
                ));
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
        let c_hat: Vec<E::G2> = (0..ell as usize)
            // Note: Using a multiexp will be 10-20% slower than manually multiplying.
            // .map(|j| g2_multi_exp(&pp.lagrange_basis_g2, &f_evals[j]))
            .map(|j| {
                let mut c_hat_j: <E as Pairing>::G2 = pk.lagr_g2[0].mul(&r[j]);
                c_hat_j.add_assign(&utils::msm_bool(
                    &pk.lagr_g2[1..=pk.max_n],
                    &f_evals_without_r[j],
                ));
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

        let num_omegas = pk.max_n + 1;

        // Step 6:
        //  1. Compute each f_j(X) in coefficient form via a size-(n+1) FFT on f_j(X)
        //  2. Compute f'_j(X) via a differentiation.
        //  3. Evaluate f'_j at all (n+1)th roots of unity via a size-(n+1) FFT.
        //  5. for i = 0, compute N_j'(\omega^i) = r_j(r_j - 1)
        //  4. \forall i > 0, compute N_j'(\omega^i) = (\omega^i - 1) f_j'(\omega^i)(2f_j(\omega^i) - 1)
        //  6. \forall i \in [0,n], compute h_j(\omega^i) = N_j'(\omega^i) / ( (n+1)\omega^{i n} )
        #[cfg(feature = "range_proof_timing")]
        let start = Instant::now();
        // let omega_n = pp.roots_of_unity_in_eval_dom[pp.n];
        let n1_inv = E::ScalarField::from((pk.max_n + 1) as u64)
            .inverse()
            .unwrap();

        let f_evals: Vec<Vec<E::ScalarField>> = f_evals_without_r
            .iter()
            .enumerate()
            .map(|(j, col)| {
                once(r[j])
                    .chain(col.iter().map(|&b| E::ScalarField::from(b)))
                    .collect()
            })
            .collect();

        let h: Vec<Vec<E::ScalarField>> = (0..ell as usize)
            .map(|j| {
                // Interpolate f_j coeffs
                let mut f_j = f_evals[j].clone();
                pk.eval_dom.ifft_in_place(&mut f_j);
                assert_eq!(f_j.len(), pk.max_n + 1);

                // Compute f'_j derivative
                let mut diff_f_j = f_j.clone();
                polynomials::differentiate_in_place(&mut diff_f_j);
                assert_eq!(diff_f_j.len(), pk.max_n);

                // Evaluate f'_j at all (n+1)th roots of unity
                let mut diff_f_j_evals = diff_f_j.clone();
                pk.eval_dom.fft_in_place(&mut diff_f_j_evals);
                assert_eq!(diff_f_j_evals.len(), pk.max_n + 1);

                // N'_j(\omega^0) = r_j(r_j - 1)
                let mut diff_n_j_evals = Vec::with_capacity(num_omegas);
                diff_n_j_evals.push(r[j].square() - r[j]);

                // \forall i > 0, N'_j(\omega^i) = (\omega^i - 1) f_j'(\omega^i)(2f_j(\omega^i) - 1)
                for i in 1..(pk.max_n + 1) {
                    diff_n_j_evals.push(
                        (pk.roots_of_unity_minus_one[i - 1])
                            * diff_f_j_evals[i]
                            * (f_evals[j][i].double() - E::ScalarField::ONE),
                    );
                }
                assert_eq!(diff_n_j_evals.len(), num_omegas);

                // \forall i \in [0,n], h_j(\omega^i)
                //  = N_j'(\omega^i) / ( (n+1)\omega^{i n} )
                //  = N_j'(\omega^i) * (\omega^i / (n+1))
                let mut h_j = Vec::with_capacity(num_omegas);
                for i in 0..pk.max_n + 1 {
                    h_j.push(
                        diff_n_j_evals[i]
                            .mul(pk.roots_of_unity_in_eval_dom[i])
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
        let public_statement = PublicStatement {
            n,
            ell,
            comm: comm.clone(),
        };
        let c_aff = E::G1::normalize_batch(&c);
        let c_hat_aff = E::G2::normalize_batch(&c_hat);
        let bit_commitments = (c_aff.as_slice(), c_hat_aff.as_slice());
        let (_, betas) = fiat_shamir_challenges(
            &pk.vk,
            public_statement,
            &bit_commitments,
            c.as_slice().len(),
            &mut fs_transcript,
        );
        assert_eq!(ell as usize, betas.len());
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
        let mut hh: Vec<E::ScalarField> = vec![E::ScalarField::ZERO; pk.max_n + 1];
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
        let d =
            VariableBaseMSM::msm(&pk.lagr_g1[0..num_omegas], &hh).expect("Failed computing msm"); // TODO: Not very "variable base"...
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

        Proof {
            d,
            c: c_aff,
            c_hat: c_hat_aff,
        }
    }

    fn pairing_for_verify<R: RngCore + CryptoRng>(
        &self,
        vk: &Self::VerificationKey,
        n: usize,
        ell: u8,
        comm: &Self::Commitment,
        _rng: &mut R,
    ) -> anyhow::Result<(Vec<E::G1Affine>, Vec<E::G2Affine>)> {
        let mut fs_t = merlin::Transcript::new(Self::DST);

        assert!(
            ell <= vk.max_ell,
            "ell (got {}) must be ≤ max_ell (which is {})",
            ell,
            vk.max_ell
        );

        let commitment_recomputed: E::G1 =
            VariableBaseMSM::msm(&self.c, &vk.powers_of_two[..ell as usize])
                .expect("Failed to compute msm");
        ensure!(comm.0 == commitment_recomputed);

        let public_statement = PublicStatement {
            n,
            ell,
            comm: comm.clone(),
        };
        let bit_commitments = (&self.c[..], &self.c_hat[..]);
        let (alphas, betas) = fiat_shamir_challenges(
            &vk,
            public_statement,
            &bit_commitments,
            self.c.len(),
            &mut fs_t,
        );

        // Verify h(\tau)
        // TODO: uhh I should also move this multi-pairing to the output... oh well this code is dead anyway
        let h_check = E::multi_pairing(
            (0..ell as usize)
                .map(|j| self.c[j] * betas[j]) // E::G1
                .chain(once(-self.d)) // add -d
                .collect::<Vec<_>>(), // collect into Vec<E::G1>
            (0..ell as usize)
                .map(|j| self.c_hat[j] - vk.tau_2) // E::G2
                .chain(once(vk.vanishing_com)) // add vanishing commitment
                .collect::<Vec<_>>(), // collect into Vec<E::G2>
        );
        ensure!(PairingOutput::<E>::ZERO == h_check);

        // Ensure duality: c[j] matches c_hat[j].

        // Compute MSM in G1: sum_j (alphas[j] * proof.c[j])
        let g1_comb = VariableBaseMSM::msm(&self.c, &alphas).unwrap();

        // Compute MSM in G2: sum_j (alphas[j] * proof.c_hat[j])
        let g2_comb = VariableBaseMSM::msm(&self.c_hat, &alphas).unwrap();


        Ok((E::G1::normalize_batch(&vec![
            g1_comb,   // from MSM in G1
            -vk.tau_1, // subtract tau_1
        ]), E::G2::normalize_batch(&vec![
            vk.tau_2, // tau_2
            g2_comb,  // from MSM in G2
        ])))

        // let c_check = E::multi_pairing(
        //     vec![
        //         g1_comb,   // from MSM in G1
        //         -vk.tau_1, // subtract tau_1
        //     ],
        //     vec![
        //         vk.tau_2, // tau_2
        //         g2_comb,  // from MSM in G2
        //     ],
        // );
        // ensure!(PairingOutput::<E>::ZERO == c_check);

        // Ok(())
    }

    fn maul(&mut self) {
        self.c[0] = (self.c[0] + E::G1::generator()).into_affine();
    }
}

/// Compute alpha, beta.
fn fiat_shamir_challenges<E: Pairing>(
    vk: &VerificationKey<E>,
    public_statement: PublicStatement<E>,
    bit_commitments: &(&[E::G1Affine], &[E::G2Affine]), // TODO: make this generic over B?
    num_scalars: usize,
    fs_transcript: &mut merlin::Transcript,
) -> (Vec<E::ScalarField>, Vec<E::ScalarField>) {
    <merlin::Transcript as fiat_shamir::RangeProof<E, Proof<E>>>::append_sep(fs_transcript, DST);

    <merlin::Transcript as fiat_shamir::RangeProof<E, Proof<E>>>::append_vk(fs_transcript, vk);

    <merlin::Transcript as fiat_shamir::RangeProof<E, Proof<E>>>::append_public_statement(
        fs_transcript,
        public_statement,
    );

    <merlin::Transcript as fiat_shamir::RangeProof<E, Proof<E>>>::append_f_j_commitments(
        fs_transcript,
        bit_commitments,
    );

    // Generate the Fiat–Shamir challenges from the updated transcript
    let beta_vals =
        <merlin::Transcript as fiat_shamir::RangeProof<E, Proof<E>>>::challenges_for_linear_combination(
            fs_transcript,
            num_scalars,
        );

    let alpha_vals =
        <merlin::Transcript as fiat_shamir::RangeProof<E, Proof<E>>>::challenges_for_linear_combination(
            fs_transcript,
            num_scalars,
        );

    (alpha_vals, beta_vals)
}
