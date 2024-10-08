use super::round_state::NewRoundReason;
use aptos_collections::BoundedVecDeque;
use aptos_consensus_types::{
    common::Author, payload_pull_params::OptQSPayloadPullParams, round_timeout::RoundTimeoutReason,
};
use aptos_infallible::Mutex;
use std::{collections::HashSet, sync::Arc};

pub trait TPastProposalStatusTracker: Send + Sync {
    fn push(&self, status: NewRoundReason);
}

pub trait TOptQSPullParamsProvider: Send + Sync {
    fn get_params(&self) -> Option<OptQSPayloadPullParams>;
}

/// A exponential window based algorithm to decide whether to go optimistic or not, based on
/// configurable number of past proposal statuses
///
/// Initialize the window at 2.
/// - For each proposal failure, double the window up to a MAX size
/// - If there are no failures within the window, then propose optimistic batch
/// - If there are no failures up to MAX proposals, reset the window to 2.
pub struct ExponentialWindowFailureTracker {
    window: usize,
    max_window: usize,
    past_round_statuses: BoundedVecDeque<NewRoundReason>,
    last_consecutive_success_count: usize,
    ordered_authors: Vec<Author>,
}

impl ExponentialWindowFailureTracker {
    pub(crate) fn new(max_window: usize, ordered_authors: Vec<Author>) -> Self {
        Self {
            window: 2,
            max_window,
            past_round_statuses: BoundedVecDeque::new(max_window),
            last_consecutive_success_count: 0,
            ordered_authors,
        }
    }

    pub(crate) fn push(&mut self, status: NewRoundReason) {
        self.past_round_statuses.push_back(status);
        self.compute_failure_window();
    }

    fn last_consecutive_statuses_matching<F>(&self, matcher: F) -> usize
    where
        F: Fn(&NewRoundReason) -> bool,
    {
        self.past_round_statuses
            .iter()
            .rev()
            .take_while(|reason| matcher(reason))
            .count()
    }

    fn compute_failure_window(&mut self) {
        self.last_consecutive_success_count =
            self.last_consecutive_statuses_matching(|reason| match reason {
                NewRoundReason::Timeout(RoundTimeoutReason::PayloadUnavailable { .. }) => false,
                _ => true,
            });
        if self.last_consecutive_success_count == 0 {
            self.window *= 2;
            self.window = self.window.min(self.max_window);
        } else if self.last_consecutive_success_count == self.past_round_statuses.len() {
            self.window = 2;
        }
    }

    fn get_exclude_authors(&self) -> HashSet<Author> {
        let mut exclude_authors = HashSet::new();

        let limit = self.window;
        for round_reason in self.past_round_statuses.iter().rev().take(limit) {
            if let NewRoundReason::Timeout(RoundTimeoutReason::PayloadUnavailable {
                missing_authors,
            }) = round_reason
            {
                for author_idx in missing_authors.iter_ones() {
                    if let Some(author) = self.ordered_authors.get(author_idx) {
                        exclude_authors.insert(*author);
                    }
                }
            }
        }

        exclude_authors
    }
}

impl TPastProposalStatusTracker for Mutex<ExponentialWindowFailureTracker> {
    fn push(&self, status: NewRoundReason) {
        self.lock().push(status)
    }
}

pub struct OptQSPullParamsProvider {
    enable_opt_qs: bool,
    failure_tracker: Arc<Mutex<ExponentialWindowFailureTracker>>,
}

impl OptQSPullParamsProvider {
    pub fn new(
        enable_opt_qs: bool,
        failure_tracker: Arc<Mutex<ExponentialWindowFailureTracker>>,
    ) -> Self {
        Self {
            enable_opt_qs,
            failure_tracker,
        }
    }
}

impl TOptQSPullParamsProvider for OptQSPullParamsProvider {
    fn get_params(&self) -> Option<OptQSPayloadPullParams> {
        if !self.enable_opt_qs {
            return None;
        }

        let tracker = self.failure_tracker.lock();

        let opt_batch_txns_pct = if tracker.last_consecutive_success_count < tracker.window {
            0
        } else {
            50
        };
        let exclude_authors = tracker.get_exclude_authors();

        Some(OptQSPayloadPullParams {
            opt_batch_txns_pct,
            exclude_authors,
            minimum_batch_age_usecs: 50_000_000,
        })
    }
}
