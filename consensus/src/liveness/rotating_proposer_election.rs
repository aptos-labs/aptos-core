// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::liveness::proposer_election::ProposerElection;
use aptos_consensus_types::common::{Author, Round};
use aptos_types::account_config::NewBlockEvent;
use std::collections::HashMap;

/// The rotating proposer maps a round to an author according to a round-robin rotation.
/// A fixed proposer strategy loses liveness when the fixed proposer is down. Rotating proposers
/// won't gather quorum certificates to machine loss/byzantine behavior on f/n rounds.
pub struct RotatingProposer {
    // Ordering of proposers to rotate through (all honest replicas must agree on this)
    proposers: Vec<Author>,
    // Number of contiguous rounds (i.e. round numbers increase by 1) a proposer is active
    // in a row
    contiguous_rounds: u32,
}

/// Choose a proposer that is going to be the single leader (relevant for a mock fixed proposer
/// election only).
pub fn choose_leader(peers: Vec<Author>) -> Author {
    // As it is just a tmp hack function, pick the min PeerId to be a proposer.
    peers.into_iter().min().expect("No trusted peers found!")
}

/// Choose the proposer with the lowest median round time based on recent block history.
///
/// "Round time" for a block is the difference between its timestamp and the previous block's
/// timestamp. A proposer with a lower median round time is closer to quorum — it broadcasts
/// proposals and collects 2f+1 votes faster than other proposers.
///
/// Returns `None` if fewer than `MIN_SAMPLES` rounds of history are available for every
/// proposer (caller should fall back to `choose_leader`).
pub fn choose_latency_optimal_leader(
    proposers: &[Author],
    history: &[NewBlockEvent],
) -> Option<Author> {
    if history.len() < 2 {
        return None;
    }

    // Compute per-proposer round durations from consecutive block timestamps.
    // history is sorted newest-first (decreasing epoch/round).
    let mut round_times: HashMap<Author, Vec<u64>> = HashMap::new();
    for window in history.windows(2) {
        let newer = &window[0];
        let older = &window[1];
        // Skip across epoch boundaries to avoid mixing timing characteristics.
        if newer.epoch() != older.epoch() {
            continue;
        }
        if newer.proposed_time() > older.proposed_time() {
            round_times
                .entry(newer.proposer())
                .or_default()
                .push(newer.proposed_time() - older.proposed_time());
        }
    }

    const MIN_SAMPLES: usize = 3;

    proposers
        .iter()
        .filter_map(|p| {
            let times = round_times.get(p)?;
            if times.len() < MIN_SAMPLES {
                return None;
            }
            let mut sorted = times.clone();
            sorted.sort_unstable();
            Some((*p, sorted[sorted.len() / 2]))
        })
        .min_by_key(|(_, median_us)| *median_us)
        .map(|(p, _)| p)
}

impl RotatingProposer {
    /// With only one proposer in the vector, it behaves the same as a fixed proposer strategy.
    pub fn new(proposers: Vec<Author>, contiguous_rounds: u32) -> Self {
        Self {
            proposers,
            contiguous_rounds,
        }
    }
}

impl ProposerElection for RotatingProposer {
    fn get_valid_proposer(&self, round: Round) -> Author {
        self.proposers
            [((round / u64::from(self.contiguous_rounds)) % self.proposers.len() as u64) as usize]
    }
}
