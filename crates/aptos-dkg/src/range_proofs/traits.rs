// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use ark_ec::pairing::Pairing;
use ark_ff::UniformRand;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::rand::{CryptoRng, RngCore};

pub trait BatchedRangeProof<E: Pairing>: Clone + CanonicalSerialize + CanonicalDeserialize {
    type PublicStatement: CanonicalSerialize;
    type ProverKey;
    type VerificationKey: Clone + CanonicalSerialize; // Serialization is needed because this is often appended to a Fiat-Shamir transcript
    type Input: From<u64>; // TODO: slightly hacky
    type Commitment;
    type CommitmentRandomness: Clone + ark_ff::UniformRand;
    type CommitmentKey;

    const DST: &[u8];

    fn commitment_key_from_prover_key(pk: &Self::ProverKey) -> Self::CommitmentKey;

    /// Setup generates the prover and verifier keys used in the batched range proof.
    fn setup<R: RngCore + CryptoRng>(
        max_n: usize,
        max_ell: usize,
        rng: &mut R,
    ) -> (Self::ProverKey, Self::VerificationKey);

    fn commit<R: RngCore + CryptoRng>(
        ck: &Self::CommitmentKey,
        values: &[Self::Input],
        rng: &mut R,
    ) -> (Self::Commitment, Self::CommitmentRandomness) {
        let r = Self::CommitmentRandomness::rand(rng);
        let comm = Self::commit_with_randomness(ck, values, &r.clone());
        (comm, r)
    }

    fn commit_with_randomness(
        ck: &Self::CommitmentKey,
        values: &[Self::Input],
        r: &Self::CommitmentRandomness,
    ) -> Self::Commitment;

    fn prove<R: RngCore + CryptoRng>(
        pk: &Self::ProverKey,
        values: &[Self::Input],
        ell: usize,
        comm: &Self::Commitment,
        r: &Self::CommitmentRandomness,
        fs_transcript: &mut merlin::Transcript,
        rng: &mut R,
    ) -> Self;

    fn verify(
        &self,
        vk: &Self::VerificationKey,
        n: usize,
        ell: usize,
        comm: &Self::Commitment,
        fs_transcript: &mut merlin::Transcript,
    ) -> anyhow::Result<()>;

    fn maul(&mut self);
}
