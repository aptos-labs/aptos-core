// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::fiat_shamir::PolynomialCommitmentScheme;
use ark_ff::AdditiveGroup;
use crate::{
    Scalar,
    pcs::{
        shplonked::{Srs, ZkPcsOpeningProof, zk_pcs_commit, zk_pcs_open, zk_pcs_verify},
        univariate_hiding_kzg,
    },
    range_proofs::{dekart_univariate_v2::two_term_msm, traits},
    sigma_protocol::{homomorphism::Trait as _, Trait as _},
    sumcheck::{ml_sumcheck::protocol::verifier::VerifierMsg, rng::TranscriptRng},
    utils,
};
use aptos_crypto::arkworks::srs::SrsBasis;
use aptos_crypto::arkworks::{
    msm::MsmInput,
    random::{sample_field_element, sample_field_elements},
    srs::SrsType,
    GroupGenerators,
};
use crate::sigma_protocol::homomorphism::TrivialShape;
use crate::sumcheck::ml_sumcheck::protocol::prover::ProverMsg;
use ark_poly::univariate::DensePolynomial;
use ark_poly::DenseUVPolynomial;
use ark_ec::{pairing::Pairing, AffineRepr, CurveGroup, PrimeGroup, VariableBaseMSM};
use ark_ff::Field;
use ark_poly::Polynomial;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use std::iter::once;
use rand::{CryptoRng, RngCore};
use std::fmt::Debug;
use ark_poly::DenseMultilinearExtension;
use std::iter::successors;
#[cfg(feature = "range_proof_timing_multivariate")]
use std::time::{Duration, Instant};

#[allow(non_snake_case)]
#[derive(CanonicalSerialize, CanonicalDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct ProverKey<E: Pairing> {
    pub(crate) vk: VerificationKey<E>,
    pub(crate) ck: univariate_hiding_kzg::CommitmentKey<E>,
    pub(crate) max_n: usize,
    //pub(crate) prover_precomputed: ProverPrecomputed<E>,
}

#[derive(CanonicalSerialize, CanonicalDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct VerificationKey<E: Pairing> {
    xi_1: E::G1Affine,
    last_tau: E::G1Affine,
    vk_hkzg: univariate_hiding_kzg::VerificationKey<E>,
    //verifier_precomputed: VerifierPrecomputed<E>,
    poly_info: crate::sumcheck::ml_sumcheck::protocol::PolynomialInfo,
    srs: Srs<E>,
}

// This is a copy-paste
#[derive(CanonicalSerialize)]
pub struct PublicStatement<E: Pairing> {
    n: usize,
    ell: usize,
    comm: univariate_hiding_kzg::Commitment<E>,
}

#[derive(CanonicalSerialize, Clone, CanonicalDeserialize)]
pub struct Proof<E: Pairing> {
    /// Blinding commitment C_β (None if blinding was not used)
    pub blinding_poly_comm: Option<E::G1Affine>,
    /// Proof that C_β is of the form β·eq_0 (None if blinding was not used)
    pub blinding_poly_proof: Option<two_term_msm::Proof<E::G1>>,
    pub asserted_sum: E::ScalarField,
    pub sumcheck_proof: Vec<ProverMsg<E::ScalarField>>,
    /// Single batched opening proof for f̂ and g_1..g_m (per spec Step 7: one uPCS.BatchVerify)
    pub zk_pcs_opening_proof: ZkPcsOpeningProof<E>,
    pub commitments: Vec<E::G1Affine>,
    pub g_commitments: Vec<E::G1Affine>,
    pub h_g: E::ScalarField,
    /// y_g = sum_i g_i(x_i), used in Step 5 check
    pub y_g: E::ScalarField,
    /// Batched evaluation f̂(z) at the opening point z (so verifier can check y_sum == y_batched_at_z + y_g)
    pub y_batched_at_z: E::ScalarField,
    pub evals: Vec<E::ScalarField>,
}

impl<E: Pairing> traits::BatchedRangeProof<E> for Proof<E> {
    type Commitment = univariate_hiding_kzg::Commitment<E>;
    type CommitmentKey = univariate_hiding_kzg::CommitmentKey<E>;
    type CommitmentRandomness = univariate_hiding_kzg::CommitmentRandomness<E::ScalarField>;
    type Input = E::ScalarField;
    type ProverKey = ProverKey<E>;
    type PublicStatement = PublicStatement<E>;
    type VerificationKey = VerificationKey<E>;

    /// Domain-separation tag (DST) used to ensure that all cryptographic hashes and
    /// transcript operations within the protocol are uniquely namespaced
    const DST: &[u8] = b"MULTIVARIATE_DEKART_RANGE_PROOF_DST";

    fn maul(&mut self) {
        if let Some(c) = self.commitments.first_mut() {
            *c = (c.into_group() + E::G1::generator()).into_affine();
        }
    }

    fn commitment_key_from_prover_key(pk: &Self::ProverKey) -> Self::CommitmentKey {
        pk.ck.clone()
    }

    #[allow(non_snake_case)]
    fn setup<R: RngCore + CryptoRng>(
        max_n: usize,
        _max_ell: u8,
        group_generators: GroupGenerators<E>,
        rng: &mut R,
    ) -> (ProverKey<E>, VerificationKey<E>) {
        let size = (max_n + 1).next_power_of_two();
        let trapdoor = univariate_hiding_kzg::Trapdoor::<E>::rand(rng);
        let (vk_hkzg, ck) = univariate_hiding_kzg::setup(
            size,
            SrsType::PowersOfTau,
            group_generators.clone(),
            trapdoor,
        );
        let tau_powers = match &ck.msm_basis {
            SrsBasis::PowersOfTau { tau_powers } => tau_powers.clone(),
            _ => panic!("Expected PowersOfTau SRS"),
        };
        let last_tau = *tau_powers
            .last()
            .expect("PowersOfTau SRS has at least one element");
        let num_vars = size.ilog2() as usize;
        let poly_info = crate::sumcheck::ml_sumcheck::data_structures::PolynomialInfo {
            num_variables: num_vars,
            max_multiplicands: 2,
        };
        let srs = Srs {
            taus_1: tau_powers,
            xi_1: ck.xi_1,
            g_2: vk_hkzg.group_generators.g2,
            tau_2: vk_hkzg.tau_2,
            xi_2: vk_hkzg.xi_2,
        };
        let vk = VerificationKey {
            xi_1: ck.xi_1,
            last_tau,
            vk_hkzg,
            poly_info,
            srs,
        };
        let pk = ProverKey {
            vk: vk.clone(),
            ck,
            max_n,
        };
        (pk, vk)
    }

    #[allow(non_snake_case)]
    fn commit_with_randomness(
        ck: &Self::CommitmentKey,
        values: &[Self::Input],
        rho: &Self::CommitmentRandomness,
    ) -> Self::Commitment {
        // Match prover layout: coeffs[0] = 0 (β when no blinding), coeffs[1..=n] = values, rest zero. // TODO!!!
        let size = (values.len() + 1).next_power_of_two();
        let mut coeffs = Vec::with_capacity(size);
        coeffs.push(E::ScalarField::ZERO);
        coeffs.extend_from_slice(values);
        coeffs.resize(size, E::ScalarField::ZERO);
        univariate_hiding_kzg::commit_with_randomness(ck, &coeffs, rho)
    }

    fn verify<R: RngCore + CryptoRng>(
        &self,
        vk: &Self::VerificationKey,
        n: usize,
        ell: u8,
        comm: &Self::Commitment,
        rng: &mut R,
    ) -> anyhow::Result<()> {
        #[cfg(feature = "range_proof_timing_multivariate")]
        let mut cumulative = Duration::ZERO;
        #[cfg(feature = "range_proof_timing_multivariate")]
        let mut print_cumulative = |name: &str, duration: Duration| {
            cumulative += duration;
            println!(
                "{:>10.2} ms  ({:>10.2} ms cum.)  {}",
                duration.as_secs_f64() * 1000.0,
                cumulative.as_secs_f64() * 1000.0,
                name
            );
        };

        // Number of variables for this instance (must match prover; can be less than vk max)
        let num_vars = (n + 1).next_power_of_two().ilog2() as usize;
        if num_vars > vk.poly_info.num_variables {
            anyhow::bail!(
                "instance n={} requires num_vars={} but setup supports at most {}",
                n,
                num_vars,
                vk.poly_info.num_variables
            );
        }

        #[cfg(feature = "range_proof_timing_multivariate")]
        let start = Instant::now();
        let mut trs = merlin::Transcript::new(b"dekart");

        // Replay transcript in same order as prover: C_beta (if any), C_fj, C_gi, H_g, then c, alpha, t
        if let Some(ref c) = self.blinding_poly_comm {
            trs.append_point(c);
        }
        for c in &self.commitments {
            trs.append_point(c);
        }
        for g in &self.g_commitments {
            trs.append_point(g);
        }
        let mut buf = Vec::new();
        self.h_g.serialize_compressed(&mut buf).expect("serialize h_g");
        trs.append_message(b"H_g", &buf);

        let c: E::ScalarField = trs.challenge_scalar::<E::ScalarField>();
        let alpha: E::ScalarField = trs.challenge_scalar::<E::ScalarField>();
        // eq_point t (same order as prover: one challenge per sumcheck variable)
        let t: Vec<E::ScalarField> = (0..num_vars)
            .map(|_| trs.challenge_scalar::<E::ScalarField>())
            .collect();
        #[cfg(feature = "range_proof_timing_multivariate")]
        print_cumulative("transcript + challenges (c, alpha, t)", start.elapsed());

        #[cfg(feature = "range_proof_timing_multivariate")]
        let start = Instant::now();
        if let (Some(blinding_comm), Some(blinding_proof)) =
            (&self.blinding_poly_comm, &self.blinding_poly_proof)
        {
            let hom = two_term_msm::Homomorphism {
                base_1: vk.last_tau,
                base_2: vk.xi_1,
            };
            hom.verify(
                &TrivialShape((*blinding_comm).into()),
                blinding_proof,
                &(),
                Some(1),
                rng,
            )?;
        }
        #[cfg(feature = "range_proof_timing_multivariate")]
        print_cumulative("blinding two_term_msm verify", start.elapsed());

        #[cfg(feature = "range_proof_timing_multivariate")]
        let start = Instant::now();
        let poly_info_instance = crate::sumcheck::ml_sumcheck::data_structures::PolynomialInfo {
            num_variables: num_vars,
            max_multiplicands: vk.poly_info.max_multiplicands,
        };
        let mut trng = TranscriptRng::<E::ScalarField>::new(&mut trs);
        let subclaim = crate::sumcheck::ml_sumcheck::MLSumcheck::verify_as_subprotocol(
            &mut trng,
            &poly_info_instance,
            self.asserted_sum,
            &self.sumcheck_proof,
        )
        .map_err(|e| anyhow::anyhow!("sumcheck verify: {:?}", e))?;
        #[cfg(feature = "range_proof_timing_multivariate")]
        print_cumulative("sumcheck verify", start.elapsed());

        // Advance transcript to match prover (prover drew last_rho here, then appended y_f, y_j, hat_c, z, then single batched open)
        let _last_rho: E::ScalarField = trs.challenge_scalar::<E::ScalarField>();

        #[cfg(feature = "range_proof_timing_multivariate")]
        let start = Instant::now();
        // Step 5: (y_f - sum 2^{j-1} y_j + sum c^j y_j(1-y_j)) * eq_t(x) * vnsh_0(x) + alpha * y_g == h_m(x_m)
        // BinaryConstraintPolynomial uses (1 - eq_zero) with eq_zero = ∏ᵢ(1-xᵢ); vnsh_0 = 1 - eq_zero (vanishes at (0,...,0))
        let x = &subclaim.point;
        // Variable order: sumcheck folds variable 0 first; point[0] = first round challenge.
        // DenseMultilinearExtension in ark_poly uses index = sum_i b_i * 2^i with b_0 LSB, so var 0 = LSB. Match that.
        let eq_t_x: E::ScalarField = (0..x.len())
            .map(|i| {
                let ti = t[i];
                let xi = x[i];
                (E::ScalarField::ONE - ti) + xi * (ti + ti - E::ScalarField::ONE)
            })
            .product();
        let eq_zero_x: E::ScalarField = x.iter().map(|&xi| E::ScalarField::ONE - xi).product();
        let vnsh_0_x = E::ScalarField::ONE - eq_zero_x;

        let two = E::ScalarField::from(2u64);
        let mut pow2 = E::ScalarField::ONE;
        let mut sum_weighted_y = self.evals[0]; // y_f
        for (_, &y_j) in self.evals.iter().enumerate().skip(1).take(ell as usize) {
            sum_weighted_y -= pow2 * y_j;
            pow2 *= two;
        }
        let mut c_pow = c;
        for &y_j in self.evals.iter().skip(1).take(ell as usize) {
            sum_weighted_y += c_pow * y_j * (E::ScalarField::ONE - y_j); // f_j(1-f_j) = y_j(1-y_j)
            c_pow *= c;
        }
        let lhs = sum_weighted_y * eq_t_x * vnsh_0_x + alpha * self.y_g;
        if lhs != subclaim.expected_evaluation {
            return Err(anyhow::anyhow!(
                "Step 5 check failed: lhs != h_m(x_m)"
            ));
        }
        #[cfg(feature = "range_proof_timing_multivariate")]
        print_cumulative("step5 scalar check (eq_t, vnsh_0, lhs)", start.elapsed());

        // Evals (y_f, y_1, ..., y_ell)
        for (i, y) in self.evals.iter().enumerate() {
            buf.clear();
            y.serialize_compressed(&mut buf).expect("serialize eval");
            trs.append_message(if i == 0 { b"y_f" } else { b"y_j" }, &buf);
        }
        let hat_c: E::ScalarField = trs.challenge_scalar();

        let hat_c_powers: Vec<E::ScalarField> =
            successors(Some(E::ScalarField::ONE), |p| Some(*p * hat_c))
                .take(ell as usize + 1)
                .collect();

        // Prover drew the opening point z before zk_pcs_open; consume it so transcript matches.
        let z: E::ScalarField = trs.challenge_scalar();
        anyhow::ensure!(
            z == self.zk_pcs_opening_proof.eval_points[0],
            "Batched opening: transcript opening point z does not match proof.eval_points[0]"
        );

        #[cfg(feature = "range_proof_timing_multivariate")]
        let start = Instant::now();
        let combined_comm = {
            let mut bases = vec![comm.0.into_affine()];
            let mut scalars = vec![E::ScalarField::ONE];
            if let Some(ref bc) = self.blinding_poly_comm {
                bases.push(*bc);
                scalars.push(E::ScalarField::ONE);
            }
            bases.extend(self.commitments.iter().copied());
            scalars.extend(hat_c_powers.iter().skip(1).copied());
            E::G1::msm(&bases, &scalars).expect("combined commitment MSM")
        };
        #[cfg(feature = "range_proof_timing_multivariate")]
        print_cumulative("combined_comm MSM", start.elapsed());

        // Step 4d (spec): single uPCS.BatchVerify for f̂ and g_1..g_m (one zk_pcs_verify call)
        let g_commitment_msms: Vec<MsmInput<E::G1Affine, E::ScalarField>> = self
            .g_commitments
            .iter()
            .map(|&affine| MsmInput::new(vec![affine], vec![E::ScalarField::ONE]).expect("single term"))
            .collect();
        let commitment_msms: Vec<MsmInput<E::G1Affine, E::ScalarField>> = once(
            MsmInput::new(
                vec![combined_comm.into_affine()],
                vec![E::ScalarField::ONE],
            )
            .expect("single term"),
        )
        .chain(g_commitment_msms)
        .collect();

        #[cfg(feature = "range_proof_timing_multivariate")]
        let start = Instant::now();
        zk_pcs_verify(
            &self.zk_pcs_opening_proof,
            &commitment_msms,
            &vk.srs,
            &mut trs,
            rng,
        )?;
        #[cfg(feature = "range_proof_timing_multivariate")]
        print_cumulative("zk_pcs_verify (batched f̂ + g)", start.elapsed());

        // y_sum in the opening proof must equal y_batched_at_z (f̂(z)) + y_g (sum of g_i at rho_i)
        anyhow::ensure!(
            self.zk_pcs_opening_proof.sigma_proof_statement.y_sum == self.y_batched_at_z + self.y_g,
            "Batched opening y_sum != y_batched_at_z + y_g"
        );

        Ok(())
    }

    #[allow(non_snake_case)]
    fn prove<R: RngCore + CryptoRng>(
        pk: &ProverKey<E>,
        values: &[Self::Input],
        ell: u8,
        comm: &Self::Commitment,
        rho: &Self::CommitmentRandomness,
        rng: &mut R,
    ) -> Proof<E> {
        // Use blinding=false: with blinding, combined_comm = comm + comm_blinding_poly + ... mixes
        // a two-term MSM (beta*tau^{n-1}+rho*xi) with KZG commitments; that sum is not a KZG
        // commitment to batched_coeffs, so the batched opening verification fails.
        prove_impl(pk, values, ell, comm, rho, rng, false)
    }
}

/// Prover with optional blinding. When `use_blinding` is false, β=0 and no C_β is produced.
#[allow(non_snake_case)]
pub fn prove_impl<E: Pairing, R: RngCore + CryptoRng>(
    pk: &ProverKey<E>,
    values: &[E::ScalarField],
    ell: u8,
    comm: &univariate_hiding_kzg::Commitment<E>,
    rho: &univariate_hiding_kzg::CommitmentRandomness<E::ScalarField>,
    rng: &mut R,
    use_blinding: bool,
) -> Proof<E>
where
    E: Pairing,
{
    let mut trs = merlin::Transcript::new(b"dekart");
    let tau_powers = match &pk.ck.msm_basis {
        SrsBasis::PowersOfTau { tau_powers } => tau_powers,
        _ => panic!("Expected PowersOfTau SRS"),
    };

    let (beta, comm_blinding_poly, comm_blinding_poly_rand, beta_sigma_proof) = if use_blinding {
        let last_msm_elt = tau_powers.last().expect("PowersOfTau SRS has no elements");
        let (b, c, r, proof): (
            E::ScalarField,
            E::G1,
            E::ScalarField,
            two_term_msm::Proof<E::G1>,
        ) = zksc_blind::<E, _>(*last_msm_elt, pk.ck.xi_1, rng);
        trs.append_point(&c.into_affine());
        (b, Some(c), Some(r), Some(proof))
    } else {
        (
            E::ScalarField::ZERO,
            None,
            None,
            None::<two_term_msm::Proof<E::G1>>,
        )
    };

    // Step 3: Sample masks β_j independently (no correlated randomness)
    let betas: Vec<E::ScalarField> = sample_field_elements(ell as usize, rng);

    // Step 4: construct the hat_f_js
    let bits: Vec<Vec<bool>> = values
            .iter()
            .map(|z_val| {
                utils::scalar_to_bits_le::<E>(z_val)
                    .into_iter()
                    .take(ell as usize)
                    .collect::<Vec<_>>()
            })
            .collect();
        // This is copy-paste:
        let hat_f_j_evals_without_r: Vec<Vec<bool>> = (0..ell as usize)
            .map(|j| bits.iter().map(|row| row[j]).collect())
            .collect(); // This is just transposing the bits matrix
        let num_vars = (values.len() + 1).next_power_of_two().ilog2() as u8;
        let size = 1 << num_vars;
        let hat_f_j_evals: Vec<Vec<E::ScalarField>> = hat_f_j_evals_without_r
            .iter()
            .enumerate()
            .map(|(j, col)| {
                let mut evals: Vec<E::ScalarField> = once(betas[j])
                    .chain(col.iter().map(|&b| E::ScalarField::from(b)))
                    .collect();
                evals.resize(size, E::ScalarField::ZERO);
                evals
            })
            .collect();

        let hat_f_js: Vec<DenseMultilinearExtension::<E::ScalarField>> = hat_f_j_evals
            .iter()
            .map(|hat_f_j_eval| DenseMultilinearExtension::from_evaluations_vec(num_vars.into(), hat_f_j_eval.clone()))
            .collect();   

        // Step 5: Commit to the hat_f_js using Zeromorph
        let hat_f_j_comms_randomness = sample_field_elements(hat_f_js.len(), rng);
        let hom: univariate_hiding_kzg::CommitmentHomomorphism<'_, E> =
            univariate_hiding_kzg::CommitmentHomomorphism {
                msm_basis: &tau_powers,
                xi_1: pk.ck.xi_1,
            };
        let hat_f_j_comms: Vec<_> = hat_f_j_evals
            .iter()
            .zip(hat_f_j_comms_randomness.iter())
            .map(|(hat_f_j_eval, r_i)| {
                hom.apply(&univariate_hiding_kzg::Witness {
                    hiding_randomness: Scalar(*r_i),
                    values: Scalar::vec_from_inner(hat_f_j_eval.clone()),
                })
                .0
            })
            .collect();
        hat_f_j_comms
            .iter()
            .for_each(|hat_f_j_comm: &E::G1| {
                trs.append_point(&hat_f_j_comm.into_affine());
            });

        // Step 7: 
        let srs = Srs {
            taus_1: tau_powers.clone(),
            xi_1: pk.ck.xi_1,
            g_2: pk.vk.vk_hkzg.group_generators.g2,
            tau_2: pk.vk.vk_hkzg.tau_2,
            xi_2: pk.vk.vk_hkzg.xi_2,
        };
        let (g_is, g_comm, g_comm_randomnesses, _G): (
            Vec<Vec<E::ScalarField>>,
            Vec<E::G1>,
            Vec<E::ScalarField>,
            E::ScalarField,
        ) = zksc_send_mask(&srs, 4, num_vars, rng);
        g_comm
            .iter()
            .for_each(|g_i_comm: &E::G1| {
                trs.append_point(&g_i_comm.into_affine());
            });
        {
            let mut buf = Vec::new();
            _G.serialize_compressed(&mut buf).expect("serialize H_g");
            trs.append_message(b"H_g", &buf);
        }

    let size = 1 << num_vars;
    let mut f_evals = vec![E::ScalarField::ZERO; size];
    f_evals[0] = beta;
    for (i, &v) in values.iter().enumerate() {
        f_evals[i + 1] = v;
    }

    // Step 5a–5c: Verifier challenges c, alpha; eq_point t; run sumcheck on transcript with linear term (f - sum 2^{j-1} f_j) + sum c^j f_j(f_j-1)
    let c: E::ScalarField = trs.challenge_scalar::<E::ScalarField>();
    let alpha: E::ScalarField = trs.challenge_scalar::<E::ScalarField>();
    let sumcheck_proof = zkzc_send_polys::<E>(
        &mut trs,
        g_is.clone(),
        num_vars,
        ell as usize,
        c,
        alpha,
        &f_evals,
        &hat_f_j_evals,
    );
    // Sumcheck proves the hypercube sum of h = [L + Σ c^j f_j(1-f_j)]*eq_t*(1-eq_zero) + α*g (eq_zero = ∏(1-xᵢ)).
    // The verifier checks proof[0].evaluations[0]+proof[0].evaluations[1] == asserted_sum, so we must
    // send the actual sum (extract_sum), not α*H_g.
    let asserted_sum =
        crate::sumcheck::ml_sumcheck::MLSumcheck::<E::ScalarField>::extract_sum(&sumcheck_proof.0);

    // Sumcheck point: use all round challenges from verifier_messages (see sumcheck fix: we now
    // include the final round's challenge so the point matches the verifier's subclaim.point).
    let rhos: Vec<E::ScalarField> = sumcheck_proof
        .1
        .into_iter()
        .map(|msg| msg.randomness)
        .collect();
    // Advance transcript to match verifier (verifier draws last_rho after sumcheck)
    let _last_rho: E::ScalarField = trs.challenge_scalar();
    let g_evals: Vec<E::ScalarField> = g_is
        .iter()
        .zip(rhos.iter())
        .map(|(g_i_coeffs, rho)| {
            let poly = DensePolynomial::from_coefficients_vec(g_i_coeffs.clone());
            poly.evaluate(rho)
        })
        .collect();
    let y_g: E::ScalarField = g_evals.iter().sum();

    // Step 6: Evaluations y_f = f(x), y_j = f_j(x) at sumcheck point x = (rho_1,...,rho_n)
    let sumcheck_point: Vec<E::ScalarField> = rhos[0..num_vars as usize].to_vec();
    let f_poly = DenseMultilinearExtension::from_evaluations_vec(
        num_vars.into(),
        f_evals.clone(),
    );
    let y_f = f_poly.evaluate(&sumcheck_point);
    let y_evals: Vec<E::ScalarField> = (0..ell as usize)
        .map(|j| hat_f_js[j].evaluate(&sumcheck_point))
        .collect();

    {
        let mut buf = Vec::new();
        y_f.serialize_compressed(&mut buf).expect("serialize y_f");
        trs.append_message(b"y_f", &buf);
    }
    for (j, y_j) in y_evals.iter().enumerate() {
        let mut buf = Vec::new();
        y_j.serialize_compressed(&mut buf).expect("serialize y_j");
        trs.append_message(b"y_j", &buf);
        let _ = j;
    }
    let hat_c: E::ScalarField = trs.challenge_scalar();

    // Batched polynomial f̂ = f + sum_j hat_c^j f_j (coefficient form for univariate opening)
    let mut base_coeffs = vec![E::ScalarField::ZERO; size];
    base_coeffs[0] = beta;
    for (i, &z_i) in values.iter().enumerate() {
        base_coeffs[i + 1] = z_i;
    }
    let hat_c_powers: Vec<E::ScalarField> =
        successors(Some(E::ScalarField::ONE), |p| Some(*p * hat_c))
            .take(ell as usize + 1)
            .collect();
    let mut batched_coeffs = base_coeffs.clone();
    for j in 0..ell as usize {
        let cj = hat_c_powers[j + 1];
        for (i, b) in batched_coeffs.iter_mut().enumerate() {
            *b += cj * hat_f_j_evals[j][i];
        }
    }

    let z: E::ScalarField = trs.challenge_scalar();
    let y: E::ScalarField = batched_coeffs
        .iter()
        .enumerate()
        .fold(E::ScalarField::ZERO, |acc, (i, &coeff)| {
            acc + coeff * z.pow([i as u64])
        });

    // Verifier will recompute this for the single batched zk_pcs_verify
    let _combined_comm = {
        let mut bases = vec![comm.0.into_affine()];
        let mut scalars = vec![E::ScalarField::ONE];
        if let Some(ref cp) = comm_blinding_poly {
            bases.push((*cp).into_affine());
            scalars.push(E::ScalarField::ONE);
        }
        for (j, &cf) in hat_f_j_comms.iter().enumerate() {
            bases.push(cf.into_affine());
            scalars.push(hat_c_powers[j + 1]);
        }
        E::G1::msm(&bases, &scalars).expect("batched commitment MSM")
    };

    let mut batched_randomness = rho.0 + comm_blinding_poly_rand.unwrap_or(E::ScalarField::ZERO);
    for (j, &r_j) in hat_f_j_comms_randomness.iter().enumerate() {
        batched_randomness += hat_c_powers[j + 1] * r_j;
    }

    // Step 7 (spec): single batched opening proof for f̂ at z and g_i at rho_i (uPCS.BatchOpen)
    let mut all_f_is = vec![batched_coeffs];
    all_f_is.extend(g_is);
    let eval_points: Vec<E::ScalarField> = once(z)
        .chain(rhos.iter().copied())
        .collect();
    let mut all_evals = vec![y];
    all_evals.extend(g_evals.iter().copied());
    let mut all_rs = vec![batched_randomness];
    all_rs.extend(g_comm_randomnesses.iter().copied());
    let zk_pcs_opening_proof = zk_pcs_open(
        &srs,
        (size - 1).max(4) as u8,
        all_f_is,
        vec![], // commitments not used in open
        eval_points,
        all_evals,
        all_rs,
        &mut trs,
        rng,
    );

    let commitments: Vec<E::G1Affine> = hat_f_j_comms.iter().map(|g| g.into_affine()).collect();
    let g_commitments: Vec<E::G1Affine> = g_comm.iter().map(|g| g.into_affine()).collect();
    let evals = once(y_f).chain(y_evals).collect();

    Proof {
        blinding_poly_comm: comm_blinding_poly.map(|c| c.into_affine()),
        blinding_poly_proof: beta_sigma_proof,
        asserted_sum,
        sumcheck_proof: sumcheck_proof.0,
        zk_pcs_opening_proof,
        commitments,
        g_commitments,
        h_g: _G,
        y_g,
        y_batched_at_z: y,
        evals,
    }
}

/// Run sumcheck on the main transcript with linear term (f - sum 2^{j-1} f_j) and constraints c^j f_j(f_j-1).
/// Draws eq_point t from trs, then runs MLSumcheck::prove_as_subprotocol with TranscriptRng.
fn zkzc_send_polys<E: Pairing>(
    trs: &mut merlin::Transcript,
    g_is: Vec<Vec<E::ScalarField>>,
    num_vars: u8,
    ell: usize,
    c: E::ScalarField,
    alpha: E::ScalarField,
    f_evals: &[E::ScalarField],
    hat_f_j_evals: &[Vec<E::ScalarField>],
) -> (Vec<ProverMsg<E::ScalarField>>, Vec<VerifierMsg<E::ScalarField>>) {
    let t: Vec<E::ScalarField> = (0..num_vars)
        .map(|_| trs.challenge_scalar::<E::ScalarField>())
        .collect();
    let nv = num_vars as usize;
    let size = 1 << nv;

    // Linear term L = f - sum_{j=0..ell-1} 2^j hat_f_j (indices: hat_f_j_evals[j] is f_{j+1})
    let two = E::ScalarField::from(2u64);
    let mut pow2 = E::ScalarField::ONE;
    let mut linear_evals = f_evals.to_vec();
    for j in 0..ell {
        for (i, l) in linear_evals.iter_mut().enumerate().take(size) {
            *l -= pow2 * hat_f_j_evals[j][i];
        }
        pow2 *= two;
    }
    let linear_term = DenseMultilinearExtension::from_evaluations_vec(nv, linear_evals);

    let mut poly = crate::sumcheck::ml_sumcheck::data_structures::BinaryConstraintPolynomial::new(
        nv,
        t,
        alpha,
        g_is,
    );
    poly.set_linear_term(linear_term);
    let mut c_j = c;
    for j in 0..ell {
        let f_hat_j = DenseMultilinearExtension::from_evaluations_vec(
            nv,
            hat_f_j_evals[j].clone(),
        );
        poly.add_constraint(c_j, f_hat_j);
        c_j *= c;
    }

    let mut trng = TranscriptRng::<E::ScalarField>::new(trs);
    let (prover_msgs, _state, verifier_msgs) = crate::sumcheck::ml_sumcheck::MLSumcheck::prove_as_subprotocol(&mut trng, &poly).expect("sumcheck prove failed");
    (prover_msgs, verifier_msgs)
}

// #[allow(non_snake_case)]
// fn prove<R: RngCore + CryptoRng>(
//     pk: &ProverKey<E>,
//     values: &[Self::Input],
//     ell: usize,
//     comm: &Self::Commitment,
//     rho: &Self::CommitmentRandomness,
//     rng: &mut R,
// ) -> Proof<E>
// {
//     // Step 1(a): Sample beta
//     let beta = sample_field_element(rng);

//     // Step 1(b): Commit to `beta \cdot eq_(1,..., 1)`, and prove knowledge of `beta`
//     let hom = two_term_msm::Homomorphism {
//             base_1: pk.lagr_g1.last().unwrap(),
//             base_2: pk.xi_1,
//         };
//     let rho = sample_field_element(rng);
//     let witness = two_term_msm::Witness {
//                 poly_randomness: Scalar(beta),
//                 hiding_kzg_randomness: Scalar(rho),
//             };
//     let blinding_poly_comm = hom.apply(&witness);
//     let sigma_proof = hom.prove(&witness, &blinding_poly_comm, &(), rng);

//     // // Step 1(b): commit to beta \cdot eq_(1,..., 1)
//     // let num_vars = (values.len() + 1).next_power_of_two().ilog2() as usize;
//     // let size = 1 << num_vars;
//     // let mut blinding_poly_values = vec![E::ScalarField::ZERO; size];
//     // blinding_poly_values[size - 1] = beta;
//     // let blinding_poly = DenseMultilinearExtension::from_evaluations_vec(num_vars, blinding_poly_values);
//     // let blinding_poly_comm = Zeromorph::commit(ck, &blinding_poly, rng);

//     // Step 3: Produce correlated randomness
//     let betas = correlated_randomness(rng, 2, ell.try_into().unwrap(), beta);

//     // Step 4: construct the hat_f_js
//     // This is copy-paste:
//     let bits: Vec<Vec<bool>> = values
//         .iter()
//         .map(|z_val| {
//             utils::scalar_to_bits_le::<E>(z_val)
//                 .into_iter()
//                 .take(ell)
//                 .collect::<Vec<_>>()
//         })
//         .collect();
//     // This is copy-paste:
//     let hat_f_j_evals_without_r: Vec<Vec<bool>> = (0..ell)
//         .map(|j| bits.iter().map(|row| row[j]).collect())
//         .collect(); // This is just transposing the bits matrix
//     let hat_f_j_evals: Vec<Vec<E::ScalarField>> = hat_f_j_evals_without_r
//         .iter()
//         .enumerate()
//         .map(|(j, col)| {
//             once(betas[j])
//                 .chain(col.iter().map(|&b| E::ScalarField::from(b)))
//                 .collect()
//         })
//         .collect();

//     let num_vars = (values.len() + 1).next_power_of_two().ilog2() as usize;
//     let hat_f_js: Vec<DenseMultilinearExtension::<E::ScalarField>> = hat_f_j_evals
//         .iter()
//         .map(|hat_f_j_eval| DenseMultilinearExtension::from_evaluations_vec(num_vars, hat_f_j_eval.clone()))
//         .collect();

//     // Step 5: Commit to the hat_f_j
//     let hat_f_j_comms: Vec<_> = hat_f_js
//         .iter()
//         .map(|hat_f_j| Zeromorph::commit(ck, hat_f_j, rng))
//         .collect();

//     // Step 6
//     let gammas = sample_field_elements(ell, rng);
//     // TODO: replace this with Fiat-Shamir?

//     // // Step 2(a):
//     // let poly = SparsePolynomial::from_coefficients_vec(
//     //     num_vars,
//     //     vec![
//     //         (sample_field_element(rng), SparseTerm::new(vec![])),
//     //         (sample_field_element(rng), SparseTerm::new(vec![(i, 1)])),
//     //         (sample_field_element(rng), SparseTerm::new(vec![(i, 2)])),
//     //     ],
//     // );

//     let g_is: Vec<_> = (0..num_vars)
//         .map(|i| {
//             SparsePolynomial::from_coefficients_vec(
//                 num_vars,
//                 vec![
//                     (sample_field_element(rng), SparseTerm::new(vec![])),
//                     (sample_field_element(rng), SparseTerm::new(vec![(i, 1)])),
//                     (sample_field_element(rng), SparseTerm::new(vec![(i, 2)])),
//                 ],
//             )
//         })
//         .collect();

//     // Step 2(b):
//     let g = g_is.iter().cloned().sum();

//     // // Step 2(c):
//     // let g_comm = Zeromorph::commit(ck, &g, rng);

//     // let mut G = E::ScalarField::ZERO;
//     // for i in 0..(1 << num_vars) {
//     //     // build the Boolean vector corresponding to i
//     //     let point: Vec<E::ScalarField> = (0..num_vars)
//     //         .map(|j| if (i >> j) & 1 == 1 {
//     //             E::ScalarField::ONE
//     //         } else {
//     //             E::ScalarField::ZERO
//     //         })
//     //         .collect();

//     //     G += g.evaluate(&point);
//     // }

// }

fn zksc_blind<E: Pairing, R: RngCore + CryptoRng>(
    last_msm_elt: E::G1Affine,
    xi_1: E::G1Affine,
    rng: &mut R,    
) -> (E::ScalarField, E::G1, E::ScalarField, two_term_msm::Proof<E::G1>) {
    // Step 1: Sample `beta`
    let beta = sample_field_element(rng);

    // Step 2: Commit to `beta \cdot eq_(1,..., 1)` using a simplified version of Zeromorph
    let hom = two_term_msm::Homomorphism {
            base_1: last_msm_elt,
            base_2: xi_1,
        };
    let rho = sample_field_element(rng);
    let witness = two_term_msm::Witness {
                poly_randomness: Scalar(beta),
                hiding_kzg_randomness: Scalar(rho),
            };
    let blinding_poly_comm = hom.apply(&witness);

    // Step 3: Prove knowledge of `beta`
    let (sigma_proof, _) = hom.prove(&witness, blinding_poly_comm.clone(), &(), rng);

    (beta, blinding_poly_comm.0, rho, sigma_proof)
}

fn zksc_send_mask<E: Pairing, R: RngCore + CryptoRng>(
    srs: &Srs<E>,
    d: u8,
    num_vars: u8,
    rng: &mut R,
) -> (Vec<Vec<E::ScalarField>>, Vec<E::G1>, Vec<E::ScalarField>, E::ScalarField) {
    // Step (1): Sample the g_i
    let g_is: Vec<_> = (0..num_vars)
        .map(|_| sample_field_elements((d + 1).into(), rng))
        .collect();

    // Step (2): Commit
    let r_is = sample_field_elements(num_vars.into(), rng);
    let g_comm: Vec<E::G1> = zk_pcs_commit(srs, g_is.clone(), r_is.clone());

    let mut sum_c = E::ScalarField::ZERO;
    let mut sum_b = E::ScalarField::ZERO;

    for g_i in g_is.clone() {
        sum_c += g_i[0];
        sum_b += g_i[1..].iter().copied().sum::<E::ScalarField>();
    }

    let two = E::ScalarField::from(2u64);
    let total_sum =
        two.pow([num_vars as u64]) * sum_c
        + two.pow([(num_vars - 1) as u64]) * sum_b;

    (g_is, g_comm, r_is, total_sum)
}


// /// Samples a specific kind of random polynomial `g`, then evaluates it at all points in {0,1}^num_vars and returns the polynomial, this sum and a commitment
// fn send_mask<E: Pairing, R: RngCore + CryptoRng>(ck: ZeromorphProverKey<E>, d: u8, num_vars: u8, rng: &mut R) -> (SparsePolynomial<E::ScalarField, SparseTerm>, E::G1, E::ScalarField) {

//     // Step (a): Sample the g_i
//     let g_is: Vec<_> = (0..num_vars)
//         .map(|i| {
//             SparsePolynomial::from_coefficients_vec(
//                 num_vars.into(),
//                 (0..=d)
//                     .map(|k| {
//                         let term = if k == 0 {
//                             // constant term
//                             SparseTerm::new(vec![])
//                         } else {
//                             SparseTerm::new(vec![(i.into(), k as usize)])
//                         };

//                         (sample_field_element(rng), term)
//                     })
//                     .collect())
//             })
//         .collect();

//     // Step (b): Sum them into one polynomial
//     let g = g_is.iter().cloned().sum();

//     // Step (c): Commit and compute the sum
//     let g_comm = univariate_hiding_kzg::commit(&ck, g, rng);

//     let mut sum = E::ScalarField::ZERO;

//     for i in 0..(1 << num_vars) {
//         // build the Boolean vector corresponding to i
//         let point: Vec<E::ScalarField> = (0..num_vars)
//             .map(|j| if (i >> j) & 1 == 1 { E::ScalarField::ONE } else { E::ScalarField::ZERO })
//             .collect();

//         sum += g.evaluate(&point);
//     }

//     (g, sum, comm)
// }


// pub mod blinding_check {
//     // TODO: maybe fixed_base_msms should become a folder and put its code inside mod.rs? Then put this mod inside of that folder?
//     use super::*;
//     use crate::sigma_protocol::{homomorphism::fixed_base_msms, traits::FirstProofItem};
//     use aptos_crypto::arkworks::{msm::IsMsmInput, random::UniformRand};
//     use aptos_crypto_derive::SigmaProtocolWitness;
//     use ark_ec::AffineRepr;
//     pub use sigma_protocol::homomorphism::TrivialShape as CodomainShape;
//     pub type Proof<C> = sigma_protocol::Proof<
//         <<C as CurveGroup>::Affine as AffineRepr>::ScalarField,
//         Homomorphism<C>,
//     >;

//     /// Represents a homomorphism with two base points over an elliptic curve group.
//     ///
//     /// This structure defines a map from two scalars to one group element:
//     /// `f(x1, x2) = base_1 * x1 + base_2 * x2`.
//     #[derive(CanonicalSerialize, Clone, Debug, PartialEq, Eq)]
//     pub struct Homomorphism<C: CurveGroup> {
//         pub base_1: C::Affine,
//         pub base_2: C::Affine,
//     }

//     #[derive(
//         SigmaProtocolWitness, CanonicalSerialize, CanonicalDeserialize, Clone, Debug, PartialEq, Eq,
//     )]
//     pub struct Witness<F: PrimeField> {
//         pub poly_randomness: Scalar<F>,
//         pub hiding_kzg_randomness: Scalar<F>,
//     }

//     impl<C: CurveGroup> homomorphism::Trait for Homomorphism<C> {
//         type Codomain = CodomainShape<C>;
//         type Domain = Witness<C::ScalarField>;

//         fn apply(&self, input: &Self::Domain) -> Self::Codomain {
//             // Not doing `self.apply_msm(self.msm_terms(input))` because E::G1::msm is slower!
//             // `msm_terms()` is still useful for verification though: there the code will use it to produce an MSM
//             //  of size 2+2 (the latter two are for the first prover message A and the statement P)
//             CodomainShape(
//                 self.base_1 * input.poly_randomness.0 + self.base_2 * input.hiding_kzg_randomness.0,
//             )
//         }
//     }

//     impl<C: CurveGroup> fixed_base_msms::Trait for Homomorphism<C> {
//         type Base = C::Affine;
//         type CodomainShape<T>
//             = CodomainShape<T>
//         where
//             T: CanonicalSerialize + CanonicalDeserialize + Clone + Eq + Debug;
//         type MsmInput = MsmInput<C::Affine, C::ScalarField>;
//         type MsmOutput = C;
//         type Scalar = C::ScalarField;

//         fn msm_terms(&self, input: &Self::Domain) -> Self::CodomainShape<Self::MsmInput> {
//             let mut scalars = Vec::with_capacity(2);
//             scalars.push(input.poly_randomness.0);
//             scalars.push(input.hiding_kzg_randomness.0);

//             let mut bases = Vec::with_capacity(2);
//             bases.push(self.base_1);
//             bases.push(self.base_2);

//             CodomainShape(MsmInput { bases, scalars })
//         }

//         fn msm_eval(input: Self::MsmInput) -> Self::MsmOutput {
//             C::msm(input.bases(), input.scalars()).expect("MSM failed in TwoTermMSM")
//         }

//         fn batch_normalize(msm_output: Vec<Self::MsmOutput>) -> Vec<Self::Base> {
//             C::normalize_batch(&msm_output)
//         }
//     }

//     impl<C: CurveGroup> sigma_protocol::Trait<C> for Homomorphism<C> {
//         fn dst(&self) -> Vec<u8> {
//             b"DEKART_V2_SIGMA_PROTOCOL".to_vec()
//         }
//     }
// }

// mod ml_sumcheck {
//     /// Prover Message
//     #[derive(Clone, CanonicalSerialize)]
//     pub struct ProverMsg<F: Field> {
//         /// evaluations on P(0), P(1), P(2), ... 
//         pub(crate) evaluations: Vec<F>,
//     }

//     /// Prover State for binary constraints with eq_t masking and g polynomial
//     pub struct ProverState<F: Field> {
//         /// sampled randomness given by the verifier
//         pub randomness: Vec<F>,
//         /// List of (coefficient, polynomial) pairs
//         pub constraints: Vec<(F, DenseMultilinearExtension<F>)>,
//         /// The eq_t point (original, never modified)
//         pub eq_point_original: Vec<F>,
//         /// Coefficient α for g term
//         pub alpha: F,
//         /// Random univariate polynomials g₁, ..., gₙ (coefficients)
//         pub g_polys: Vec<Vec<F>>,
//         /// Number of variables
//         pub num_vars: usize,
//         /// The current round number
//         pub round: usize,
//     }



// }

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_crypto::arkworks::GroupGenerators;
    use ark_bn254::Bn254;
    use rand::thread_rng;

    #[test]
    fn test_prove_verify_simple() {
        type E = Bn254;
        let mut rng = thread_rng();
        let group_generators = GroupGenerators::default();

        // Setup: max_n = 4 so size = 8 (SRS supports degree-4 g), num_vars = 3; max_ell = 8.
        let max_n = 4;
        let max_ell = 8u8;
        let (pk, vk) =
            <Proof<E> as traits::BatchedRangeProof<E>>::setup(max_n, max_ell, group_generators, &mut rng);

        // Four values so (4+1).next_power_of_two() = 8, num_vars = 3, matching vk.
        let values: Vec<ark_bn254::Fr> = vec![
            ark_bn254::Fr::from(0u64),
            ark_bn254::Fr::from(42u64),
            ark_bn254::Fr::from(1u64),
            ark_bn254::Fr::from(100u64),
        ];
        let n = values.len();

        let ck = <Proof<E> as traits::BatchedRangeProof<E>>::commitment_key_from_prover_key(&pk);
        let (comm, r) = <Proof<E> as traits::BatchedRangeProof<E>>::commit(&ck, &values, &mut rng);

        let proof = <Proof<E> as traits::BatchedRangeProof<E>>::prove(
            &pk, &values, max_ell, &comm, &r, &mut rng,
        );

        // Assert proof structure
        assert_eq!(proof.sumcheck_proof.len(), 3, "sumcheck rounds = num_vars");
        assert_eq!(proof.commitments.len(), max_ell as usize);
        assert_eq!(proof.evals.len(), 1 + max_ell as usize, "y_f + y_1..y_ell");

        traits::BatchedRangeProof::<E>::verify(&proof, &vk, n, max_ell, &comm, &mut rng)
            .expect("verification should succeed");
    }
}