// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_crypto::arkworks::{random::UniformRand, GroupGenerators};
use ark_ec::pairing::Pairing;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use rand::{CryptoRng, RngCore};
use ark_ec::pairing::PairingOutput;
use ark_ff::AdditiveGroup;

// TODO: split this into `BatchedRangeProof` and `PairingBatchedRangeProof: BatchedRangeProof`? Or only do PairingBatchedRangeProof for now?
pub trait BatchedRangeProof<E: Pairing>: Clone + CanonicalSerialize + CanonicalDeserialize {
    type PublicStatement: CanonicalSerialize; // Serialization is needed because this is often appended to a Fiat-Shamir transcript
    type ProverKey;
    type VerificationKey: Clone + CanonicalSerialize; // Serialization is needed because this is often appended to a Fiat-Shamir transcript
    type Input: From<u64>; // Slightly hacky. It's used in `range_proof_random_instance()` to generate (chunks of) inputs that have a certain bit size
    type Commitment;
    type CommitmentRandomness: UniformRand;
    type CommitmentKey;

    const DST: &[u8];

    fn commitment_key_from_prover_key(pk: &Self::ProverKey) -> Self::CommitmentKey;

    /// Setup generates the prover and verifier keys used in the batched range proof.
    fn setup<R: RngCore + CryptoRng>(
        max_n: usize,
        max_ell: u8,
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
        ell: u8,
        comm: &Self::Commitment,
        r: &Self::CommitmentRandomness,
        rng: &mut R,
    ) -> Self;

    fn verify<R: RngCore + CryptoRng>(
        &self,
        vk: &Self::VerificationKey,
        n: usize,
        ell: u8,
        comm: &Self::Commitment,
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
        ell: u8,
        comm: &Self::Commitment,
        rng: &mut R,
    ) -> anyhow::Result<(Vec<E::G1Affine>, Vec<E::G2Affine>)>;

    fn maul(&mut self);
}
