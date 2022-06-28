// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters::{
        COMMITTED_PROPOSALS_IN_WINDOW, COMMITTED_VOTES_IN_WINDOW, FAILED_PROPOSALS_IN_WINDOW,
        LEADER_REPUTATION_HISTORY_SIZE,
    },
    liveness::proposer_election::{next, ProposerElection},
};
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_types::block_metadata::{new_block_event_key, NewBlockEvent};
use consensus_types::common::{Author, Round};
use std::{cmp::Ordering, collections::HashMap, convert::TryFrom, sync::Arc};
use storage_interface::{DbReader, Order};

/// Interface to query committed BlockMetadata.
pub trait MetadataBackend: Send + Sync {
    /// Return a contiguous BlockMetadata window in which last one is at target_round or
    /// latest committed, return all previous one if not enough.
    fn get_block_metadata(&self, target_round: Round) -> Vec<NewBlockEvent>;
}

pub struct AptosDBBackend {
    epoch: u64,
    window_size: usize,
    seek_len: u64,
    aptos_db: Arc<dyn DbReader>,
    window: Mutex<Vec<(u64, NewBlockEvent)>>,
}

impl AptosDBBackend {
    pub fn new(epoch: u64, window_size: usize, seek_len: u64, aptos_db: Arc<dyn DbReader>) -> Self {
        Self {
            epoch,
            window_size,
            seek_len,
            aptos_db,
            window: Mutex::new(vec![]),
        }
    }

    fn refresh_window(&self, target_round: Round) -> anyhow::Result<()> {
        // assumes target round is not too far from latest commit
        let events = self.aptos_db.get_events(
            &new_block_event_key(),
            u64::max_value(),
            Order::Descending,
            self.window_size as u64 + self.seek_len,
        )?;
        let mut result = vec![];
        for event in events {
            let e = bcs::from_bytes::<NewBlockEvent>(event.event.event_data())?;
            if e.epoch() == self.epoch
                && e.round() <= target_round
                && result.len() < self.window_size
            {
                result.push((event.transaction_version, e));
            }
        }
        *self.window.lock() = result;
        Ok(())
    }
}

impl MetadataBackend for AptosDBBackend {
    // assume the target_round only increases
    fn get_block_metadata(&self, target_round: Round) -> Vec<NewBlockEvent> {
        let (known_version, known_round) = self
            .window
            .lock()
            .first()
            .map(|(v, e)| (*v, e.round()))
            .unwrap_or((0, 0));
        if !(known_round == target_round
            || known_version == self.aptos_db.get_latest_version().unwrap_or(0))
        {
            if let Err(e) = self.refresh_window(target_round) {
                error!(
                    error = ?e, "[leader reputation] Fail to refresh window",
                );
                return vec![];
            }
        }
        self.window
            .lock()
            .clone()
            .into_iter()
            .map(|(_, e)| e)
            .collect()
    }
}

/// Interface to calculate weights for proposers based on history.
pub trait ReputationHeuristic: Send + Sync {
    /// Return the weights of all candidates based on the history.
    fn get_weights(&self, epoch: u64, candidates: &[Author], history: &[NewBlockEvent])
        -> Vec<u64>;
}

pub struct NewBlockEventAggregation {
    // Window sizes are in number of succesfull blocks, not number of rounds.
    // i.e. we can be looking at different number of rounds for the same window,
    // dependig on how many failures we have.
    voter_window_size: usize,
    proposer_window_size: usize,
}

impl NewBlockEventAggregation {
    pub fn new(voter_window_size: usize, proposer_window_size: usize) -> Self {
        Self {
            voter_window_size,
            proposer_window_size,
        }
    }

    pub fn bitmap_to_voters<'a>(
        validators: &'a [Author],
        bitmap: &[bool],
    ) -> Result<Vec<&'a Author>, String> {
        if validators.len() != bitmap.len() {
            return Err(format!(
                "bitmap {} does not match validators {}",
                bitmap.len(),
                validators.len()
            ));
        }

        Ok(validators
            .iter()
            .zip(bitmap.iter())
            .filter_map(|(validator, &voted)| if voted { Some(validator) } else { None })
            .collect())
    }

    pub fn indices_to_validators<'a>(
        validators: &'a [Author],
        indices: &[u64],
    ) -> Result<Vec<&'a Author>, String> {
        indices
            .iter()
            .map(|index| {
                usize::try_from(*index)
                    .map_err(|_err| format!("index {} out of bounds", index))
                    .and_then(|index| {
                        validators.get(index).ok_or(format!(
                            "index {} is larger than number of validators {}",
                            index,
                            validators.len()
                        ))
                    })
            })
            .collect()
    }

    fn history_iter(
        history: &[NewBlockEvent],
        epoch: u64,
        window_size: usize,
    ) -> impl Iterator<Item = &NewBlockEvent> {
        let start = if history.len() > window_size {
            history.len() - window_size
        } else {
            0
        };

        (&history[start..])
            .iter()
            .filter(move |&meta| meta.epoch() == epoch)
    }

    pub fn count_votes(
        &self,
        epoch: u64,
        candidates: &[Author],
        history: &[NewBlockEvent],
    ) -> HashMap<Author, u32> {
        Self::history_iter(history, epoch, self.voter_window_size).fold(
            HashMap::new(),
            |mut map, meta| {
                match Self::bitmap_to_voters(candidates, meta.previous_block_votes()) {
                    Ok(voters) => {
                        for &voter in voters {
                            let count = map.entry(voter).or_insert(0);
                            *count += 1;
                        }
                    }
                    Err(msg) => {
                        error!(
                            "Voter conversion from bitmap failed at epoch {}, round {}: {}",
                            meta.epoch(),
                            meta.round(),
                            msg
                        )
                    }
                }
                map
            },
        )
    }

    pub fn count_proposals(&self, epoch: u64, history: &[NewBlockEvent]) -> HashMap<Author, u32> {
        Self::history_iter(history, epoch, self.proposer_window_size).fold(
            HashMap::new(),
            |mut map, meta| {
                let count = map.entry(meta.proposer()).or_insert(0);
                *count += 1;
                map
            },
        )
    }

    pub fn count_failed_proposals(
        &self,
        epoch: u64,
        candidates: &[Author],
        history: &[NewBlockEvent],
    ) -> HashMap<Author, u32> {
        Self::history_iter(history, epoch, self.proposer_window_size).fold(
            HashMap::new(),
            |mut map, meta| {
                match Self::indices_to_validators(candidates, meta.failed_proposer_indices()) {
                    Ok(failed_proposers) => {
                        for &failed_proposer in failed_proposers {
                            let count = map.entry(failed_proposer).or_insert(0);
                            *count += 1;
                        }
                    }
                    Err(msg) => {
                        error!(
                            "Failed proposer conversion from indices failed at epoch {}, round {}: {}",
                            meta.epoch(),
                            meta.round(),
                            msg
                        )
                    }
                }
                map
            },
        )
    }
}

/// If candidate appear in the history, it's assigned active_weight otherwise inactive weight.
pub struct ActiveInactiveHeuristic {
    author: Author,
    active_weight: u64,
    inactive_weight: u64,
    aggregation: NewBlockEventAggregation,
}

impl ActiveInactiveHeuristic {
    pub fn new(
        author: Author,
        active_weight: u64,
        inactive_weight: u64,
        window_size: usize,
    ) -> Self {
        Self {
            author,
            active_weight,
            inactive_weight,
            aggregation: NewBlockEventAggregation::new(window_size, window_size),
        }
    }
}

impl ReputationHeuristic for ActiveInactiveHeuristic {
    fn get_weights(
        &self,
        epoch: u64,
        candidates: &[Author],
        history: &[NewBlockEvent],
    ) -> Vec<u64> {
        let votes = self.aggregation.count_votes(epoch, candidates, history);
        let proposals = self.aggregation.count_proposals(epoch, history);

        COMMITTED_PROPOSALS_IN_WINDOW.set(*proposals.get(&self.author).unwrap_or(&0) as i64);
        COMMITTED_VOTES_IN_WINDOW.set(*votes.get(&self.author).unwrap_or(&0) as i64);
        LEADER_REPUTATION_HISTORY_SIZE.set(proposals.values().sum::<u32>() as i64);

        candidates
            .iter()
            .map(|author| {
                if votes.contains_key(author) || proposals.contains_key(author) {
                    self.active_weight
                } else {
                    self.inactive_weight
                }
            })
            .collect()
    }
}

/// Heuristic that looks at successful and failed proposals, as well as voting history,
/// to define node reputation, used for leader selection.
///
/// We want to optimize leader selection to primarily maximize network's throughput,
/// but we also, in combinatoin with staking rewards logic, need to be reasonably fair.
///
/// Logic is:
///  * if proposer round failure rate within the proposer window is strictly above threshold, use failed_weight (default 1).
///  * otherwise, if node had no proposal rounds and no successful votes, use inactive_weight (default 10).
///  * otherwise, use the default active_weight (default 100).
///
/// We primarily want to avoid failed rounds, as they have a largest negative effect on the network.
/// So if we see a node having failures to propose, when it was the leader, we want to avoid that node.
/// We add a threshold (instead of penalizing on a single failure), so that transient issues in the network,
/// or malicious behaviour of the next leader is avoided. In general, we expect there to be
/// proposer_window_size/num_validators opportunities for a node to be a leader, so a single failure, or a
/// subset of following leaders being malicious will not be enough to exclude a node.
/// On the other hand, single failure, without any successes before will exclude the note.
/// Threshold probably makes the most sense to be between:
///  * 10% (aggressive exclusion with 1 failure in 10 proposals being enough for exclusion)
///  * and 33% (much less aggressive exclusion, with 1 failure for every 2 successes, should still reduce failed
///    rounds by at least 66%, and is enough to avoid byzantine attacks as well as the rest of the protocol)
pub struct ProposerAndVoterHeuristic {
    author: Author,
    active_weight: u64,
    inactive_weight: u64,
    failed_weight: u64,
    failure_threshold_percent: u32,
    aggregation: NewBlockEventAggregation,
}

impl ProposerAndVoterHeuristic {
    pub fn new(
        author: Author,
        active_weight: u64,
        inactive_weight: u64,
        failed_weight: u64,
        failure_threshold_percent: u32,
        voter_window_size: usize,
        proposer_window_size: usize,
    ) -> Self {
        Self {
            author,
            active_weight,
            inactive_weight,
            failed_weight,
            failure_threshold_percent,
            aggregation: NewBlockEventAggregation::new(voter_window_size, proposer_window_size),
        }
    }
}

impl ReputationHeuristic for ProposerAndVoterHeuristic {
    fn get_weights(
        &self,
        epoch: u64,
        candidates: &[Author],
        history: &[NewBlockEvent],
    ) -> Vec<u64> {
        let votes = self.aggregation.count_votes(epoch, candidates, history);
        let proposals = self.aggregation.count_proposals(epoch, history);
        let failed_proposals = self
            .aggregation
            .count_failed_proposals(epoch, candidates, history);

        COMMITTED_PROPOSALS_IN_WINDOW.set(*proposals.get(&self.author).unwrap_or(&0) as i64);
        FAILED_PROPOSALS_IN_WINDOW.set(*proposals.get(&self.author).unwrap_or(&0) as i64);
        COMMITTED_VOTES_IN_WINDOW.set(*votes.get(&self.author).unwrap_or(&0) as i64);
        LEADER_REPUTATION_HISTORY_SIZE.set(proposals.values().sum::<u32>() as i64);

        candidates
            .iter()
            .map(|author| {
                let cur_votes = *votes.get(author).unwrap_or(&0);
                let cur_proposals = *proposals.get(author).unwrap_or(&0);
                let cur_failed_proposals = *failed_proposals.get(author).unwrap_or(&0);

                if cur_failed_proposals * 100
                    > (cur_proposals + cur_failed_proposals) * self.failure_threshold_percent
                {
                    self.failed_weight
                } else if cur_proposals > 0 || cur_votes > 0 {
                    self.active_weight
                } else {
                    self.inactive_weight
                }
            })
            .collect()
    }
}

/// Committed history based proposer election implementation that could help bias towards
/// successful leaders to help improve performance.
pub struct LeaderReputation {
    epoch: u64,
    proposers: Vec<Author>,
    backend: Box<dyn MetadataBackend>,
    heuristic: Box<dyn ReputationHeuristic>,
    exclude_round: u64,
}

impl LeaderReputation {
    pub fn new(
        epoch: u64,
        proposers: Vec<Author>,
        backend: Box<dyn MetadataBackend>,
        heuristic: Box<dyn ReputationHeuristic>,
        exclude_round: u64,
    ) -> Self {
        // assert!(proposers.is_sorted()) implementation from new api
        assert!(proposers.windows(2).all(|w| {
            PartialOrd::partial_cmp(&&w[0], &&w[1])
                .map(|o| o != Ordering::Greater)
                .unwrap_or(false)
        }));

        Self {
            epoch,
            proposers,
            backend,
            heuristic,
            exclude_round,
        }
    }
}

impl ProposerElection for LeaderReputation {
    fn get_valid_proposer(&self, round: Round) -> Author {
        let target_round = round.saturating_sub(self.exclude_round);
        let sliding_window = self.backend.get_block_metadata(target_round);
        let mut weights = self
            .heuristic
            .get_weights(self.epoch, &self.proposers, &sliding_window);
        assert_eq!(weights.len(), self.proposers.len());
        let mut total_weight = 0;
        for w in &mut weights {
            total_weight += *w;
            *w = total_weight;
        }
        let mut state = round.to_le_bytes().to_vec();
        let chosen_weight = next(&mut state) % total_weight;
        let chosen_index = weights
            .binary_search_by(|w| {
                if *w <= chosen_weight {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            })
            .unwrap_err();
        self.proposers[chosen_index]
    }
}
