// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Some of this is derived from: https://www.ietf.org/archive/id/draft-zkproof-polycommit-00.html

// TODO: This trait is still very much a work in progress

use crate::fiat_shamir::PolynomialCommitmentScheme as _;
use rand_core::{CryptoRng, RngCore};
use std::fmt::Debug;

pub trait PolynomialCommitmentScheme {
    type CommitmentKey: Clone;
    type VerificationKey: Clone;
    type Polynomial: Clone;
    type WitnessField: Copy + From<u64> + Debug + Eq; // So the domain of a polynomial is a Vec<WitnessField>
                                                      // For small fields, add ChallengeField here, which should probably have a from-WitnessField-property
    /// Commitment produced by the prover (e.g. a group element).
    type Commitment: Clone + Into<Self::VerifierCommitment>;
    /// Commitment representation accepted by the verifier (e.g. `MsmInput` so it can be merged into one MSM).
    type VerifierCommitment: Clone;
    type Proof: Clone;

    fn setup<R: RngCore + CryptoRng>(
        // security_bits: usize, // make this an Option<usize> ??
        degree_bounds: Vec<usize>,
        rng: &mut R,
    ) -> (Self::CommitmentKey, Self::VerificationKey);

    fn commit(
        ck: &Self::CommitmentKey,
        poly: Self::Polynomial,
        r: Option<Self::WitnessField>,
    ) -> Self::Commitment;

    fn open<R: RngCore + CryptoRng>(
        ck: &Self::CommitmentKey,
        poly: Self::Polynomial,
        // com: Self::Commitment,
        //com_state: CommitmentState,
        challenge: Vec<Self::WitnessField>,
        // Might want to put `eval` here
        r: Option<Self::WitnessField>,
        rng: &mut R,
        trs: &mut merlin::Transcript,
    ) -> Self::Proof;

    fn batch_open<R: RngCore + CryptoRng>(
        ck: Self::CommitmentKey,
        polys: Vec<Self::Polynomial>,
        //   coms: Vec<Commitment>,
        challenge: Vec<Self::WitnessField>,
        rs: Option<Vec<Self::WitnessField>>,
        rng: &mut R,
        trs: &mut merlin::Transcript,
    ) -> Self::Proof;

    fn verify(
        vk: &Self::VerificationKey,
        com: impl Into<Self::VerifierCommitment>,
        challenge: Vec<Self::WitnessField>,
        eval: Self::WitnessField,
        proof: Self::Proof,
        trs: &mut merlin::Transcript,
        batch: bool,
    ) -> anyhow::Result<()>;

    fn random_witness<R: RngCore + CryptoRng>(rng: &mut R) -> Self::WitnessField;

    fn polynomial_from_vec(vec: Vec<Self::WitnessField>) -> Self::Polynomial;

    fn evaluate_point(
        poly: &Self::Polynomial,
        point: &Vec<Self::WitnessField>,
    ) -> Self::WitnessField;

    fn scheme_name() -> &'static [u8];

    /// Transcript domain separator for single open/verify. Used when `verify(..., batch: false)`.
    /// Prover and verifier must use the same DST so the verifier can reconstruct the same
    /// Fiat–Shamir challenges from the proof.
    fn transcript_dst_for_single_open() -> &'static [u8] {
        b"pcs_single_open_test"
    }

    /// Transcript domain separator for batch open/verify. Used when `verify(..., batch: true)`.
    fn transcript_dst_for_batch_open() -> &'static [u8] {
        b"pcs_batch_open_test"
    }
}

/// Generate a random polynomial from a set of size `len` consisting of values of bit-length `ell`.
///
/// - `len` controls the number of values used to generate the polynomial.
/// - `ell`: if `Some(ell)`, each value is masked to `ell` bits (should be at most 64); if `None`, each coefficient is a full random element in the witness field via `random_witness`.
pub fn random_poly<PCS: PolynomialCommitmentScheme, R: RngCore + CryptoRng>(
    rng: &mut R,
    len: u32, // limited to u32 only because higher wouldn't be too slow for most commitment schemes
    ell: Option<u8>,
) -> PCS::Polynomial {
    let coeffs: Vec<PCS::WitnessField> = (0..len)
        .map(|_| match ell {
            None => PCS::random_witness(rng),
            Some(ell_bits) => {
                let val = rng.next_u64() >> (64 - ell_bits);
                PCS::WitnessField::from(val)
            },
        })
        .collect();

    PCS::polynomial_from_vec(coeffs)
}

/// Generate a random evaluation point in FF^n.
///
/// This corresponds to sampling a point at which the polynomial will be opened.
/// The dimension `num_vars` should be log2 of the polynomial length.
pub fn random_point<CS: PolynomialCommitmentScheme, R: RngCore + CryptoRng>(
    rng: &mut R,
    num_vars: u32, // i.e. this is `n` if the point lies in `FF^n`
) -> Vec<CS::WitnessField> {
    (0..num_vars).map(|_| CS::random_witness(rng)).collect()
}

/// Returns the first Fiat–Shamir challenge from a fresh transcript with the given DST.
///
/// **Test / batch tests only.** In a real protocol the verifier’s challenge depends on
/// the prover’s first message (and other transcript contents), not just the DST.
/// This helper does not model that; it is only for tests that need a deterministic
/// scalar from a transcript (e.g. batch tests that reconstruct the combined commitment).
pub fn first_transcript_challenge<F: ark_ff::PrimeField>(dst: &[u8]) -> F {
    let mut t = merlin::Transcript::new(dst);
    t.challenge_scalar()
}
