// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::fiat_shamir::SerializeForFiatShamirTranscript;
use aptos_crypto::arkworks::{random::UniformRand, GroupGenerators};
use ark_ec::pairing::{Pairing, PairingOutput};
use ark_ff::AdditiveGroup;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use rand::{CryptoRng, RngCore};
use std::fmt::Debug;

// TODO: split this into `BatchedRangeProof` and `PairingBatchedRangeProof: BatchedRangeProof`? Or only do PairingBatchedRangeProof for now?
pub trait BatchedRangeProof<E: Pairing>: Clone + CanonicalSerialize + CanonicalDeserialize {
    type PublicStatement: CanonicalSerialize; // Serialization is needed because this is often appended to a Fiat-Shamir transcript
    type ProverKey: CanonicalSerialize + CanonicalDeserialize + Eq + Debug;
    type VerificationKey: CanonicalSerialize
        + CanonicalDeserialize
        + Eq
        + Debug
        + Clone
        + SerializeForFiatShamirTranscript; // This is often appended to a Fiat-Shamir transcript
    type Input: From<u64>; // Slightly hacky. It's used in `range_proof_random_instance()` to generate (chunks of) inputs that have a certain bit size
    type Commitment: Clone + Into<Self::CommitmentNormalised>;
    type CommitmentNormalised: Clone;
    type CommitmentRandomness: UniformRand;
    type CommitmentKey;
    type ProofProjective: Into<Self>; // TODO: Might want to expand this by making it return its projective elements, and building Self from affinisations of those. But not needed atm

    const DST: &[u8];

    fn commitment_key_from_prover_key(pk: &Self::ProverKey) -> Self::CommitmentKey;

    /// Setup generates the prover and verifier keys used in the batched range proof.
    fn setup<R: RngCore + CryptoRng>(
        max_n: usize,
        max_ell: usize,
        group_generators: GroupGenerators<E>,
        rng: &mut R,
    ) -> (Self::ProverKey, Self::VerificationKey);

    fn commit<R: RngCore + CryptoRng>(
        ck: &Self::CommitmentKey,
        values: &[Self::Input],
        rng: &mut R,
    ) -> (Self::Commitment, Self::CommitmentRandomness) {
        let r = Self::CommitmentRandomness::rand(rng);
        let comm = Self::commit_with_randomness(ck, values, &r);
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
        comm: &Self::CommitmentNormalised,
        r: &Self::CommitmentRandomness,
        rng: &mut R,
    ) -> Self::ProofProjective;

    fn verify<R: RngCore + CryptoRng>(
        &self,
        vk: &Self::VerificationKey,
        n: usize,
        ell: usize,
        comm: &Self::CommitmentNormalised,
        rng: &mut R,
    ) -> anyhow::Result<()> {
        let (g1_terms, g2_terms) = self.pairing_for_verify(vk, n, ell, comm, rng)?;
        let check = E::multi_pairing(g1_terms, g2_terms);
        anyhow::ensure!(PairingOutput::<E>::ZERO == check);

        Ok(())
    }

    fn pairing_for_verify<R: RngCore + CryptoRng>(
        &self,
        vk: &Self::VerificationKey,
        n: usize,
        ell: usize,
        comm: &Self::CommitmentNormalised,
        rng: &mut R,
    ) -> anyhow::Result<(Vec<E::G1Affine>, Vec<E::G2Affine>)>;

    fn maul(&mut self);
}
