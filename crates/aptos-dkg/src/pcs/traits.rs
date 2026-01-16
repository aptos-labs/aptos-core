// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Some of this is derived from: https://www.ietf.org/archive/id/draft-zkproof-polycommit-00.html
// TODO: This trait is still very much a work in progress

use rand::{CryptoRng, RngCore};

pub trait PolynomialCommitmentScheme {
    type CommitmentKey: Clone;
    type VerificationKey: Clone;
    type Polynomial: Clone;
    type WitnessField: Clone + From<u64>; // So the domain of a polynomial is a Vec<WitnessField>
    type Commitment: Clone;
    type Proof: Clone;

    fn setup<R: rand_core::RngCore + rand_core::CryptoRng>(
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
        com: Self::Commitment,
        challenge: Vec<Self::WitnessField>,
        eval: Self::WitnessField,
        proof: Self::Proof,
    ) -> anyhow::Result<()>;

    fn random_witness<R: rand_core::RngCore + rand_core::CryptoRng>(
        rng: &mut R,
    ) -> Self::WitnessField;

    fn polynomial_from_vec(vec: Vec<Self::WitnessField>) -> Self::Polynomial;

    fn evaluate_point(
        poly: &Self::Polynomial,
        point: &Vec<Self::WitnessField>,
    ) -> Self::WitnessField;

    fn scheme_name() -> &'static [u8];
}

/// Generate a random polynomial from a set of size `len` consisting of values of bit-length `ell`.
///
/// - `len` controls the number of values used to generate the polynomial.
/// - `ell` controls the bit-length of each value (should be at most 64).
pub fn random_poly<CS: PolynomialCommitmentScheme, R: rand_core::RngCore + rand_core::CryptoRng>(
    rng: &mut R,
    len: u32, // limited to u32 only because higher wouldn't be too slow for most commitment schemes
    ell: u8,
) -> CS::Polynomial {
    // Sample `len` field elements, each constructed from an `ell`-bit integer
    let ell_bit_values: Vec<CS::WitnessField> = (0..len)
        .map(|_| {
            // Mask to `ell` bits by shifting away higher bits
            let val = rng.next_u64() >> (64 - ell);
            CS::WitnessField::from(val)
        })
        .collect();

    // Convert the value vector into a polynomial representation
    CS::polynomial_from_vec(ell_bit_values)
}

/// Generate a random evaluation point in FF^n.
///
/// This corresponds to sampling a point at which the polynomial will be opened.
/// The dimension `num_vars` should be log2 of the polynomial length.
pub fn random_point<
    CS: PolynomialCommitmentScheme,
    R: rand_core::RngCore + rand_core::CryptoRng,
>(
    rng: &mut R,
    num_vars: u32, // i.e. this is `n` if the point lies in `FF^n`
) -> Vec<CS::WitnessField> {
    (0..num_vars).map(|_| CS::random_witness(rng)).collect()
}
