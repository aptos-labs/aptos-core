// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::round_state::NewRoundReason;
use crate::counters;
use aptos_collections::BoundedVecDeque;
use aptos_consensus_types::{
    common::Author, payload_pull_params::OptQSPayloadPullParams, round_timeout::RoundTimeoutReason,
};
use aptos_infallible::Mutex;
use aptos_logger::warn;
use aptos_short_hex_str::AsShortHexStr;
use raptr::raptr::{types::RoundEntryReason, TRaptrFailureTracker};
use std::{collections::HashSet, ops::Deref, sync::Arc};

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

pub struct LockedExponentialWindowFailureTracker(Mutex<ExponentialWindowFailureTracker>);

impl From<Mutex<ExponentialWindowFailureTracker>> for LockedExponentialWindowFailureTracker {
    fn from(mutex: Mutex<ExponentialWindowFailureTracker>) -> Self {
        Self(mutex)
    }
}

impl Deref for LockedExponentialWindowFailureTracker {
    type Target = Mutex<ExponentialWindowFailureTracker>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TRaptrFailureTracker for LockedExponentialWindowFailureTracker {
    fn push_reason(&self, reason: RoundEntryReason) {
        let reason = match reason {
            RoundEntryReason::ThisRoundQC(_) => NewRoundReason::QCReady,
            RoundEntryReason::FullPrefixQC(_) => NewRoundReason::QCReady,
            RoundEntryReason::CC(_, _) => NewRoundReason::QCReady,
            RoundEntryReason::TC(tc, _) => NewRoundReason::Timeout(tc.reason()),
        };
        self.0.lock().push(reason)
    }
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
        self.last_consecutive_success_count = self.last_consecutive_statuses_matching(|reason| {
            !matches!(
                reason,
                NewRoundReason::Timeout(RoundTimeoutReason::PayloadUnavailable { .. })
            )
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

impl TPastProposalStatusTracker for LockedExponentialWindowFailureTracker {
    fn push(&self, status: NewRoundReason) {
        self.lock().push(status)
    }
}

pub struct OptQSPullParamsProvider {
    enable_opt_qs: bool,
    minimum_batch_age_usecs: u64,
    failure_tracker: Arc<LockedExponentialWindowFailureTracker>,
}

impl OptQSPullParamsProvider {
    pub fn new(
        enable_opt_qs: bool,
        minimum_batch_age_usecs: u64,
        failure_tracker: Arc<LockedExponentialWindowFailureTracker>,
    ) -> Self {
        Self {
            enable_opt_qs,
            minimum_batch_age_usecs,
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

        counters::OPTQS_LAST_CONSECUTIVE_SUCCESS_COUNT
            .observe(tracker.last_consecutive_success_count as f64);
        if tracker.last_consecutive_success_count < tracker.window {
            warn!(
                "Skipping OptQS: (last_consecutive_successes) {} < {} (window)",
                tracker.last_consecutive_success_count, tracker.window
            );
            return Some(OptQSPayloadPullParams {
                exclude_authors: HashSet::new(),
                minimum_batch_age_usecs: self.minimum_batch_age_usecs,
            });
        }

        let exclude_authors = tracker.get_exclude_authors();
        if !exclude_authors.is_empty() {
            let exclude_authors_str: Vec<_> =
                exclude_authors.iter().map(|a| a.short_str()).collect();
            for author in &exclude_authors_str {
                counters::OPTQS_EXCLUDE_AUTHORS_COUNT
                    .with_label_values(&[author.as_str()])
                    .inc();
            }
            warn!("OptQS exclude authors: {:?}", exclude_authors_str);
        }
        Some(OptQSPayloadPullParams {
            exclude_authors,
            minimum_batch_age_usecs: self.minimum_batch_age_usecs,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::ExponentialWindowFailureTracker;
    use crate::liveness::round_state::NewRoundReason;
    use aptos_bitvec::BitVec;
    use aptos_consensus_types::round_timeout::RoundTimeoutReason;
    use aptos_types::validator_verifier::random_validator_verifier;

    #[test]
    fn test_exponential_window_failure_tracker() {
        let (_signers, verifier) = random_validator_verifier(4, None, false);
        let mut tracker =
            ExponentialWindowFailureTracker::new(100, verifier.get_ordered_account_addresses());
        assert_eq!(tracker.max_window, 100);

        tracker.push(NewRoundReason::QCReady);
        assert_eq!(tracker.window, 2);
        assert_eq!(tracker.last_consecutive_success_count, 1);

        tracker.push(NewRoundReason::QCReady);
        assert_eq!(tracker.window, 2);
        assert_eq!(tracker.last_consecutive_success_count, 2);

        tracker.push(NewRoundReason::QCReady);
        assert_eq!(tracker.window, 2);
        assert_eq!(tracker.last_consecutive_success_count, 3);

        tracker.push(NewRoundReason::Timeout(
            RoundTimeoutReason::ProposalNotReceived,
        ));
        assert_eq!(tracker.window, 2);
        assert_eq!(tracker.last_consecutive_success_count, 4);

        tracker.push(NewRoundReason::Timeout(RoundTimeoutReason::NoQC));
        assert_eq!(tracker.window, 2);
        assert_eq!(tracker.last_consecutive_success_count, 5);

        tracker.push(NewRoundReason::Timeout(RoundTimeoutReason::Unknown));
        assert_eq!(tracker.window, 2);
        assert_eq!(tracker.last_consecutive_success_count, 6);

        tracker.push(NewRoundReason::Timeout(
            RoundTimeoutReason::PayloadUnavailable {
                missing_authors: BitVec::with_num_bits(4),
            },
        ));
        assert_eq!(tracker.window, 4);
        assert_eq!(tracker.last_consecutive_success_count, 0);

        tracker.push(NewRoundReason::QCReady);
        assert_eq!(tracker.window, 4);
        assert_eq!(tracker.last_consecutive_success_count, 1);

        // Check that the window does not grow beyond max_window
        for _ in 0..10 {
            tracker.push(NewRoundReason::Timeout(
                RoundTimeoutReason::PayloadUnavailable {
                    missing_authors: BitVec::with_num_bits(4),
                },
            ));
        }
        assert_eq!(tracker.window, tracker.max_window);

        for _ in 0..tracker.max_window {
            tracker.push(NewRoundReason::QCReady);
        }
        assert_eq!(tracker.window, 2);
        assert_eq!(tracker.last_consecutive_success_count, tracker.max_window);
    }
}
