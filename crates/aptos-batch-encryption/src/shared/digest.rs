// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
//! This module deals with computing and generating opening proofs for "digests",
//! which are KZG polynomial commitments which commit to a set of IDs.
use super::ids::{ComputedCoeffs, Id, IdSet};
use crate::{
    errors::{BatchEncryptionError, DigestKeyInitError},
    group::{Fr, G1Affine, G1Projective, G2Affine, G2Projective, PairingSetting},
    shared::{algebra::fk_algorithm::FKDomain, ids::UncomputedCoeffs},
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
};

/// The digest public parameters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DigestKey {
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub tau_g2: G2Affine,
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub tau_powers_g1: Vec<Vec<G1Affine>>,
    pub fk_domain: FKDomain<Fr, G1Projective>,
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
            (i % 2 == 0)
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

        let tau_g2: G2Affine = (G2Affine::generator() * tau).into();

        let fk_domain = FKDomain::new(batch_size, batch_size, tau_powers_g1_projective).ok_or(
            BatchEncryptionError::DigestInitError(DigestKeyInitError::FKDomainInitFailure),
        )?;

        Ok(DigestKey {
            tau_g2,
            tau_powers_g1,
            fk_domain,
        })
    }

    pub fn capacity(&self) -> usize {
        self.tau_powers_g1[0].len() - 1
    }

    pub fn digest(
        &self,
        ids: &mut IdSet<UncomputedCoeffs>,
        round: u64,
    ) -> Result<(Digest, EvalProofsPromise)> {
        let round: usize = round as usize;
        if round >= self.tau_powers_g1.len() {
            Err(anyhow!(
                "Tried to compute digest with round greater than setup length."
            ))
        } else if ids.capacity() > self.tau_powers_g1[round].len() - 1 {
            Err(anyhow!(
                "Tried to compute a batch digest with size {}, where setup supports up to size {}",
                ids.capacity(),
                self.tau_powers_g1[round].len() - 1
            ))?
        } else {
            let ids = ids.compute_poly_coeffs();
            let mut coeffs = ids.poly_coeffs();
            coeffs.resize(self.tau_powers_g1[round].len(), Fr::zero());

            let digest = Digest {
                digest_g1: G1Projective::msm(&self.tau_powers_g1[round], &coeffs)
                    .unwrap()
                    .into(),
                round,
            };

            Ok((digest.clone(), EvalProofsPromise::new(digest, ids)))
        }
    }

    fn verify_pf(&self, digest: &Digest, id: Id, pf: G1Affine) -> Result<()> {
        // TODO use multipairing here?
        Ok((PairingSetting::pairing(
            pf,
            self.tau_g2 - G2Projective::from(G2Affine::generator() * id.x()),
        ) == PairingSetting::pairing(digest.as_g1(), G2Affine::generator()))
        .then_some(())
        .ok_or(BatchEncryptionError::EvalProofVerifyError)?)
    }

    pub fn verify(&self, digest: &Digest, pfs: &EvalProofs, id: Id) -> Result<()> {
        let pf = pfs.computed_proofs[&id];
        self.verify_pf(digest, id, pf)
    }

    pub fn verify_all(&self, digest: &Digest, pfs: &EvalProofs) -> Result<()> {
        pfs.computed_proofs
            .iter()
            .try_for_each(|(id, pf)| self.verify_pf(digest, *id, *pf))
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

    pub fn compute_all(&self, digest_key: &DigestKey) -> EvalProofs {
        EvalProofs {
            computed_proofs: self
                .ids
                .compute_all_eval_proofs_with_setup(digest_key, self.digest.round),
        }
    }

    pub fn compute_all_vgzz_multi_point_eval(&self, digest_key: &DigestKey) -> EvalProofs {
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
        let mut ids = IdSet::with_capacity(dk.capacity()).unwrap();
        let mut counter = Fr::zero();

        for _ in 0..dk.capacity() {
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
            let mut ids = IdSet::with_capacity(batch_capacity).unwrap();
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
        assert_eq!(dk.capacity(), 8);
    }
}
