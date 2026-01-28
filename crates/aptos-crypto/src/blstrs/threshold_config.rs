// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Threshold secret sharing configuration for BLSTRS-based PVSS.

use crate::{
    blstrs::evaluation_domain::{BatchEvaluationDomain, EvaluationDomain},
    player::Player,
    traits::{self, ThresholdConfig as _},
};
use anyhow::anyhow;
use rand::{seq::IteratorRandom, Rng};
use rand_core::{CryptoRng, RngCore};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt::{Display, Formatter};

/// Encodes the *threshold configuration* for a normal/unweighted PVSS: i.e., the threshold $t$ and
/// the number of players $n$ such that any $t$ or more players can reconstruct a dealt secret given
/// a PVSS transcript. Due to the last fields, this struct should only be used in the context of `blstrs`
#[derive(Clone, PartialEq, Serialize, Debug, Eq)]
pub struct ThresholdConfigBlstrs {
    /// The reconstruction threshold $t$ that must be exceeded in order to reconstruct the dealt
    /// secret; i.e., $t$ or more shares are needed
    pub t: usize,
    /// The total number of players involved in the PVSS protocol
    pub n: usize,
    /// Evaluation domain consisting of the $N$th root of unity and other auxiliary information
    /// needed to compute an FFT of size $N$.
    #[serde(skip)]
    dom: EvaluationDomain,
    /// Batch evaluation domain, consisting of all the $N$th roots of unity (in the scalar field),
    /// where N is the smallest power of two such that n <= N.
    #[serde(skip)]
    batch_dom: BatchEvaluationDomain,
}

impl<'de> Deserialize<'de> for ThresholdConfigBlstrs {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Deserialize only the serializable fields (t, n)
        #[derive(Deserialize)]
        struct SerializedFields {
            t: usize,
            n: usize,
        }

        let serialized = SerializedFields::deserialize(deserializer)?;

        // Rebuild the skipped fields using `new`
        ThresholdConfigBlstrs::new(serialized.t, serialized.n).map_err(serde::de::Error::custom)
    }
}

impl ThresholdConfigBlstrs {
    /// Returns a reference to the precomputed batch evaluation domain.
    pub fn get_batch_evaluation_domain(&self) -> &BatchEvaluationDomain {
        &self.batch_dom
    }

    /// Returns a reference to the primary evaluation domain.
    pub fn get_evaluation_domain(&self) -> &EvaluationDomain {
        &self.dom
    }
}

impl Display for ThresholdConfigBlstrs {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "threshold/{}-out-of-{}", self.t, self.n)
    }
}

impl traits::TSecretSharingConfig for ThresholdConfigBlstrs {
    /// For testing only.
    fn get_random_player<R>(&self, rng: &mut R) -> Player
    where
        R: RngCore + CryptoRng,
    {
        Player {
            id: rng.gen_range(0, self.n),
        }
    }

    /// For testing only.
    fn get_random_eligible_subset_of_players<R>(&self, mut rng: &mut R) -> Vec<Player>
    where
        R: RngCore,
    {
        (0..self.get_total_num_shares())
            .choose_multiple(&mut rng, self.t)
            .into_iter()
            .map(|i| self.get_player(i))
            .collect::<Vec<Player>>()
    }

    fn get_total_num_players(&self) -> usize {
        self.n
    }

    fn get_total_num_shares(&self) -> usize {
        self.n
    }
}

impl traits::ThresholdConfig for ThresholdConfigBlstrs {
    /// Creates a new $t$ out of $n$ secret sharing configuration where any subset of $t$ or more
    /// players can reconstruct the secret.
    fn new(t: usize, n: usize) -> anyhow::Result<Self> {
        if t == 0 {
            return Err(anyhow!("expected the reconstruction threshold to be > 0"));
        }

        if n == 0 {
            return Err(anyhow!("expected the number of shares to be > 0"));
        }

        if t > n {
            return Err(anyhow!(
                "expected the reconstruction threshold {t} to be < than the number of shares {n}"
            ));
        }

        let batch_dom = BatchEvaluationDomain::new(n);
        let dom = batch_dom.get_subdomain(n);
        Ok(ThresholdConfigBlstrs {
            t,
            n,
            dom,
            batch_dom,
        })
    }

    /// Returns the threshold $t$. Recall that $\ge t$ shares are needed to reconstruct.
    fn get_threshold(&self) -> usize {
        self.t
    }
}

#[cfg(test)]
mod test {
    use crate::{blstrs::threshold_config::ThresholdConfigBlstrs, traits::ThresholdConfig as _};

    #[test]
    fn create_many_configs() {
        let mut _tcs = vec![];

        for t in 1..100 {
            for n in t..100 {
                _tcs.push(ThresholdConfigBlstrs::new(t, n).unwrap())
            }
        }
    }
}
