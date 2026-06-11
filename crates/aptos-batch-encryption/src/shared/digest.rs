// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
//! This module deals with computing and generating opening proofs for "digests",
//! which are KZG polynomial commitments which commit to a set of IDs.
use super::ids::{ComputedCoeffs, Id, IdSet};
use crate::{
    errors::{BatchEncryptionError, DigestKeyInitError},
    group::{Fr, G1Affine, G1Projective, G2Affine, G2Projective, PairingSetting},
    shared::{
        algebra::fk_algorithm::{FKDomain, FKDomainParams, PreparedInput},
        ids::UncomputedCoeffs,
    },
};
use anyhow::{anyhow, Result};
use aptos_crypto::arkworks::serialization::{ark_de, ark_se};
use ark_ec::{pairing::Pairing, AffineRepr, ScalarMul, VariableBaseMSM};
use ark_std::{
    rand::{CryptoRng, RngCore},
    UniformRand,
};
use num_traits::{One, Zero};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::Arc,
};

/// Round-independent setup data: tau_g2 plus the FK algorithm domain params (`toeplitz_domain`,
/// `fft_domain`). This is what every consumer that only needs to derive an encryption key reads.
///
/// Carries `batch_size` and `num_rounds` so consumers don't need to touch a `RoundData` to learn
/// the shape of the setup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigestKeyHeader {
    pub tau_g2: G2Affine,
    pub batch_size: usize,
    pub num_rounds: usize,
    pub fk_params: FKDomainParams<Fr>,
}

/// Per-round setup data. One of these per round in the trusted setup file; the streaming
/// `DigestKeyStore` only keeps a bounded window of these in memory at once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundData {
    pub tau_powers_g1: Vec<G1Affine>,
    pub prepared_toeplitz_input: PreparedInput<Fr, G1Projective>,
}

/// Read-only view over the trusted setup. Both the eagerly-loaded [`DigestKey`] and the
/// streaming `DigestKeyStore` implement this so consumers can be written once against either.
pub trait DigestKeyView: Send + Sync {
    fn header(&self) -> &DigestKeyHeader;
    fn round(&self, r: usize) -> Arc<RoundData>;

    fn max_batch_size(&self) -> usize {
        self.header().batch_size
    }

    fn num_rounds(&self) -> usize {
        self.header().num_rounds
    }

    fn tau_g2(&self) -> G2Affine {
        self.header().tau_g2
    }

    fn digest(
        &self,
        ids: &mut IdSet<UncomputedCoeffs>,
        round: u64,
    ) -> Result<(Digest, EvalProofsPromise)> {
        let round_usize: usize = round as usize;
        if round_usize >= self.num_rounds() {
            return Err(anyhow!(
                "Tried to compute digest with round greater than setup length."
            ));
        }
        let round_data = self.round(round_usize);
        let tau_powers = &round_data.tau_powers_g1;
        if ids.capacity() > tau_powers.len() - 1 {
            return Err(anyhow!(
                "Tried to compute a batch digest with size {}, where setup supports up to size {}",
                ids.capacity(),
                tau_powers.len() - 1
            ));
        }
        let ids = ids.compute_poly_coeffs();
        let mut coeffs = ids.poly_coeffs();
        coeffs.resize(tau_powers.len(), Fr::zero());

        let digest = Digest {
            digest_g1: G1Projective::msm(tau_powers, &coeffs)
                .expect("Sizes should always match up b/c of check above")
                .into(),
            round: round_usize,
        };

        Ok((digest.clone(), EvalProofsPromise::new(digest, ids)))
    }

    fn verify_pf(&self, digest: &Digest, id: Id, pf: G1Affine) -> Result<()> {
        Ok(PairingSetting::multi_pairing([pf, -digest.as_g1()], [
            G2Affine::from(
                self.header().tau_g2 - G2Projective::from(G2Affine::generator() * id.x()),
            ),
            G2Affine::generator(),
        ])
        .is_zero()
        .then_some(())
        .ok_or(BatchEncryptionError::EvalProofVerifyError)?)
    }

    fn verify(&self, digest: &Digest, pfs: &EvalProofs, id: Id) -> Result<()> {
        let pf = pfs.computed_proofs[&id];
        self.verify_pf(digest, id, pf)
    }

    fn verify_all(&self, digest: &Digest, pfs: &EvalProofs) -> Result<()> {
        pfs.computed_proofs
            .iter()
            .try_for_each(|(id, pf)| self.verify_pf(digest, *id, *pf))
    }
}

/// The digest public parameters.
///
/// Internally split into a round-independent [`DigestKeyHeader`] and a vector of per-round
/// [`RoundData`] blobs (each wrapped in `Arc` so they can be shared with — or migrated into —
/// the streaming `DigestKeyStore` without copying). Callers that only need `tau_g2` go through
/// [`DigestKey::tau_g2`]; callers that need per-round data go through [`DigestKey::round`].
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DigestKey {
    pub header: Arc<DigestKeyHeader>,
    pub rounds: Vec<Arc<RoundData>>,
}

impl DigestKey {
    /// Construct a `DigestKey` directly from its split-out pieces. Used by the file reader and
    /// tests to assemble a key without going through the round-generating `new` path.
    pub fn from_parts(header: Arc<DigestKeyHeader>, rounds: Vec<Arc<RoundData>>) -> Self {
        Self { header, rounds }
    }

    pub fn tau_g2(&self) -> G2Affine {
        self.header.tau_g2
    }

    pub fn tau_powers_g1_for_round(&self, round: usize) -> &[G1Affine] {
        &self.rounds[round].tau_powers_g1
    }

    /// Borrow the round-data slot. Cheap clone (Arc).
    pub fn round_data(&self, round: usize) -> Arc<RoundData> {
        Arc::clone(&self.rounds[round])
    }
}

impl DigestKeyView for DigestKey {
    fn header(&self) -> &DigestKeyHeader {
        &self.header
    }

    fn round(&self, r: usize) -> Arc<RoundData> {
        Arc::clone(&self.rounds[r])
    }
}

/// A succinct commitment to a set of IDs. Internally, a KZG commitment over a shifted
/// powers-of-tau.
#[derive(Clone, Serialize, Deserialize, Debug, Default, PartialEq, Eq, Hash)]
pub struct Digest {
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    digest_g1: G1Affine,
    round: usize,
}

impl Digest {
    pub fn as_g1(&self) -> G1Affine {
        self.digest_g1
    }

    #[allow(unused)]
    pub(super) fn new_for_testing<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        Self {
            digest_g1: G1Affine::rand(rng),
            round: 0,
        }
    }
}

impl DigestKey {
    pub fn new(rng: &mut impl RngCore, batch_size: usize, num_rounds: usize) -> Result<Self> {
        let mut i = batch_size;
        while i > 1 {
            i.is_multiple_of(2)
                .then_some(())
                .ok_or(BatchEncryptionError::DigestInitError(
                    DigestKeyInitError::BatchSizeMustBePowerOfTwo,
                ))?;
            i >>= 1;
        }

        let tau = Fr::rand(rng);

        let mut tau_powers_fr = vec![Fr::one()];
        let mut cur = tau;
        for _ in 0..batch_size {
            tau_powers_fr.push(cur);
            cur *= &tau;
        }

        let rs: Vec<Fr> = (0..num_rounds).map(|_| Fr::rand(rng)).collect();

        let tau_powers_randomized_fr = rs
            .into_iter()
            .map(|r| {
                tau_powers_fr
                    .iter()
                    .map(|tau_power| r * tau_power)
                    .collect::<Vec<Fr>>()
            })
            .collect::<Vec<Vec<Fr>>>();

        let tau_powers_g1: Vec<Vec<G1Affine>> = tau_powers_randomized_fr
            .into_iter()
            .map(|powers_for_r| G1Projective::from(G1Affine::generator()).batch_mul(&powers_for_r))
            .collect();

        let tau_powers_g1_projective: Vec<Vec<G1Projective>> = tau_powers_g1
            .iter()
            .map(|gs| gs.iter().map(|g| G1Projective::from(*g)).collect())
            .collect();

        debug_assert_eq!(tau_powers_g1[0].len(), batch_size + 1);
        debug_assert_eq!(tau_powers_g1_projective[0].len(), batch_size + 1);

        let tau_g2: G2Affine = (G2Affine::generator() * tau).into();

        let fk_domain = FKDomain::new(batch_size, batch_size, tau_powers_g1_projective).ok_or(
            BatchEncryptionError::DigestInitError(DigestKeyInitError::FKDomainInitFailure),
        )?;

        Ok(Self::from_fk_domain(tau_g2, tau_powers_g1, fk_domain))
    }

    /// Internal helper: assemble a `DigestKey` from the legacy split (tau_g2, per-round powers,
    /// fully-materialized `FKDomain`). Splits the FKDomain into params + per-round prepared
    /// inputs and packages each round.
    fn from_fk_domain(
        tau_g2: G2Affine,
        tau_powers_g1: Vec<Vec<G1Affine>>,
        fk_domain: FKDomain<Fr, G1Projective>,
    ) -> Self {
        let batch_size = tau_powers_g1[0].len() - 1;
        let num_rounds = tau_powers_g1.len();
        let header = Arc::new(DigestKeyHeader {
            tau_g2,
            batch_size,
            num_rounds,
            fk_params: fk_domain.params(),
        });
        let rounds: Vec<Arc<RoundData>> = tau_powers_g1
            .into_iter()
            .zip(fk_domain.prepared_toeplitz_inputs)
            .map(|(tau_powers_g1, prepared_toeplitz_input)| {
                Arc::new(RoundData {
                    tau_powers_g1,
                    prepared_toeplitz_input,
                })
            })
            .collect();
        Self { header, rounds }
    }

    pub fn with_randomized_powers_of_tau(
        randomized_tau_powers_g1: Vec<Vec<G1Affine>>,
        tau_g2: G2Affine,
    ) -> Result<Self> {
        if randomized_tau_powers_g1.is_empty() {
            Err(BatchEncryptionError::DigestInitError(
                DigestKeyInitError::NumRoundsMustBeNonzero,
            ))?;
        }

        let batch_size = randomized_tau_powers_g1[0].len() - 1;

        let mut i = batch_size;
        while i > 1 {
            i.is_multiple_of(2)
                .then_some(())
                .ok_or(BatchEncryptionError::DigestInitError(
                    DigestKeyInitError::BatchSizeMustBePowerOfTwo,
                ))?;
            i >>= 1;
        }

        for powers in &randomized_tau_powers_g1 {
            if powers.len() != batch_size + 1 {
                Err(BatchEncryptionError::DigestInitError(
                    DigestKeyInitError::RandomizedTauPowersMalformedShape,
                ))?;
            }
        }

        let randomized_tau_powers_g1_projective: Vec<Vec<G1Projective>> = randomized_tau_powers_g1
            .iter()
            .map(|gs| gs.iter().map(|g| G1Projective::from(*g)).collect())
            .collect();

        let fk_domain = FKDomain::new(batch_size, batch_size, randomized_tau_powers_g1_projective)
            .ok_or(BatchEncryptionError::DigestInitError(
                DigestKeyInitError::FKDomainInitFailure,
            ))?;

        Ok(Self::from_fk_domain(
            tau_g2,
            randomized_tau_powers_g1,
            fk_domain,
        ))
    }

    pub fn max_batch_size(&self) -> usize {
        self.header.batch_size
    }

    pub fn num_rounds(&self) -> usize {
        self.header.num_rounds
    }
}

#[derive(Clone, Debug)]
pub struct EvalProofsPromise {
    pub digest: Digest,
    pub ids: IdSet<ComputedCoeffs>,
}

impl EvalProofsPromise {
    pub fn new(digest: Digest, ids: IdSet<ComputedCoeffs>) -> Self {
        Self { digest, ids }
    }

    pub fn compute_all(&self, digest_key: &dyn DigestKeyView) -> EvalProofs {
        EvalProofs {
            computed_proofs: self
                .ids
                .compute_all_eval_proofs_with_setup(digest_key, self.digest.round),
        }
    }

    pub fn compute_all_vzgg_multi_point_eval(&self, digest_key: &dyn DigestKeyView) -> EvalProofs {
        EvalProofs {
            computed_proofs: self
                .ids
                .compute_all_eval_proofs_with_setup_vzgg_multi_point_eval(
                    digest_key,
                    self.digest.round,
                ),
        }
    }
}

#[derive(Clone, Debug)]
pub struct EvalProofs {
    pub computed_proofs: HashMap<Id, G1Affine>,
}

impl EvalProofs {
    pub fn get(&self, i: &Id) -> Option<EvalProof> {
        // TODO(ibalajiarun): No need to copy here
        Some(EvalProof(self.computed_proofs.get(i).copied()?))
    }
}

/// Wrapper struct to allow for easy use of serde
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvalProof(#[serde(serialize_with = "ark_se", deserialize_with = "ark_de")] G1Affine);

impl Deref for EvalProof {
    type Target = G1Affine;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for EvalProof {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<G1Affine> for EvalProof {
    fn from(value: G1Affine) -> Self {
        Self(value)
    }
}

impl EvalProof {
    pub fn random() -> Self {
        Self(G1Affine::generator())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::shared::ids::{Id, IdSet};
    use ark_std::rand::thread_rng;

    #[allow(unused)]
    pub(crate) fn digest_and_pfs_for_testing(dk: &DigestKey) -> (Digest, EvalProofsPromise) {
        let mut ids = IdSet::with_capacity(dk.max_batch_size());
        let mut counter = Fr::zero();

        for _ in 0..dk.max_batch_size() {
            ids.add(&Id::new(counter));
            counter += Fr::one();
        }

        ids.compute_poly_coeffs();
        dk.digest(&mut ids, 0).unwrap()
    }

    #[test]
    fn compute_and_verify_all_opening_proofs() {
        let batch_capacity = 8;
        let num_rounds = 4;
        let mut rng = thread_rng();
        let setup = DigestKey::new(&mut rng, batch_capacity, num_rounds * batch_capacity).unwrap();

        for current_batch_size in 1..=batch_capacity {
            let mut ids = IdSet::with_capacity(batch_capacity);
            let mut counter = Fr::zero();

            for _ in 0..current_batch_size {
                ids.add(&Id::new(counter));
                counter += Fr::one();
            }

            ids.compute_poly_coeffs();

            for round in 0..num_rounds {
                let (d, pfs_promise) = setup.digest(&mut ids, round as u64).unwrap();
                let pfs = pfs_promise.compute_all(&setup);
                setup.verify_all(&d, &pfs).unwrap();
            }
        }
    }

    #[test]
    fn test_digest_key_capacity() {
        let mut rng = thread_rng();
        let dk = DigestKey::new(&mut rng, 8, 1).unwrap();
        assert_eq!(dk.max_batch_size(), 8);
    }

    #[test]
    fn test_with_randomized_powers_of_tau() {
        let mut rng = thread_rng();
        let dk = DigestKey::new(&mut rng, 8, 2).unwrap();
        let tau_powers_g1: Vec<Vec<G1Affine>> =
            dk.rounds.iter().map(|r| r.tau_powers_g1.clone()).collect();
        let dk2 =
            DigestKey::with_randomized_powers_of_tau(tau_powers_g1, dk.header.tau_g2).unwrap();
        assert_eq!(dk, dk2);
    }
}
