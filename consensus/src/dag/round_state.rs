// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::dag::{
    observability::{
        counters,
        tracing::{observe_round, RoundStage},
    },
    types::NodeCertificate,
};
use anyhow::ensure;
use aptos_consensus_types::common::Round;
use aptos_infallible::{duration_since_epoch, Mutex};
use aptos_logger::debug;
use aptos_types::epoch_state::EpochState;
use std::{cmp::Ordering, sync::Arc, time::Duration};
use tokio::task::JoinHandle;

pub struct RoundState {
    current_round: Mutex<Round>,
    event_sender: tokio::sync::mpsc::UnboundedSender<Round>,
    responsive_check: Box<dyn ResponsiveCheck>,
}

impl RoundState {
    pub fn new(
        event_sender: tokio::sync::mpsc::UnboundedSender<Round>,
        responsive_check: Box<dyn ResponsiveCheck>,
    ) -> Self {
        Self {
            current_round: Mutex::new(0),
            event_sender,
            responsive_check,
        }
    }

    pub fn check_for_new_round(
        &self,
        highest_strong_links_round: Round,
        strong_links: Vec<NodeCertificate>,
        minimum_delay: Duration,
    ) {
        let current_round = self.current_round.lock();

        debug!(
            round = *current_round,
            highest = highest_strong_links_round,
            "check for new round"
        );

        match current_round.cmp(&highest_strong_links_round) {
            // we're behind, move forward immediately
            Ordering::Less => {
                debug!(
                    round = *current_round,
                    highest = highest_strong_links_round,
                    "current round too low"
                );
                // the receiver can be dropped if we move to a new epoch
                let _ = self.event_sender.send(*current_round + 1);
            },
            Ordering::Equal => self.responsive_check.check_for_new_round(
                highest_strong_links_round,
                strong_links,
                minimum_delay,
            ),
            Ordering::Greater => (),
        }
    }

    pub fn set_current_round(&self, new_round: Round) -> anyhow::Result<()> {
        let mut current_round = self.current_round.lock();
        ensure!(
            *current_round < new_round,
            "current round {} is newer than new round {}",
            current_round,
            new_round
        );
        *current_round = new_round;
        debug!(round = new_round, "round state: reset");
        self.responsive_check.reset();
        Ok(())
    }

    pub fn current_round(&self) -> Round {
        *self.current_round.lock()
    }
}

/// Interface to decide if we should move forward to a new round
pub trait ResponsiveCheck: Send + Sync {
    fn check_for_new_round(
        &self,
        highest_strong_links_round: Round,
        strong_links: Vec<NodeCertificate>,
        health_backoff_delay: Duration,
    );

    fn reset(&self);
}

/// Move as fast as 2f+1
pub struct OptimisticResponsive {
    event_sender: tokio::sync::mpsc::UnboundedSender<Round>,
}

impl OptimisticResponsive {
    pub fn new(event_sender: tokio::sync::mpsc::UnboundedSender<Round>) -> Self {
        Self { event_sender }
    }
}

impl ResponsiveCheck for OptimisticResponsive {
    fn check_for_new_round(
        &self,
        highest_strong_links_round: Round,
        _strong_links: Vec<NodeCertificate>,
        _health_backoff_delay: Duration,
    ) {
        let new_round = highest_strong_links_round + 1;
        let _ = self.event_sender.send(new_round);
    }

    fn reset(&self) {}
}

enum State {
    Initial,
    Scheduled(JoinHandle<()>),
    Sent,
}

struct AdaptiveResponsiveInner {
    start_time: Duration,
    state: State,
}

/// More sophisticated strategy to move round forward given 2f+1 strong links
/// Delay if backpressure is triggered.
/// Move as soon as 3f+1 is ready. (TODO: make it configurable)
/// Move if minimal wait time is reached.
pub struct AdaptiveResponsive {
    dag_id: u8,
    inner: Mutex<AdaptiveResponsiveInner>,
    epoch_state: Arc<EpochState>,
    wait_voting_power: u128,
    minimal_wait_time: Duration,
    event_sender: tokio::sync::mpsc::UnboundedSender<Round>,
}

impl AdaptiveResponsive {
    pub fn new(
        dag_id: u8,
        event_sender: tokio::sync::mpsc::UnboundedSender<Round>,
        epoch_state: Arc<EpochState>,
        minimal_wait_time: Duration,
        wait_voting_power_pct: usize,
    ) -> Self {
        let wait_voting_power = epoch_state
            .verifier
            .total_voting_power()
            .saturating_mul(wait_voting_power_pct as u128)
            .saturating_add(50)
            .saturating_div(100);
        Self {
            dag_id,
            inner: Mutex::new(AdaptiveResponsiveInner {
                start_time: duration_since_epoch(),
                state: State::Initial,
            }),
            epoch_state,
            wait_voting_power,
            minimal_wait_time,
            event_sender,
        }
    }
}

impl ResponsiveCheck for AdaptiveResponsive {
    fn check_for_new_round(
        &self,
        highest_strong_links_round: Round,
        strong_links: Vec<NodeCertificate>,
        health_backoff_delay: Duration,
    ) {
        let mut inner = self.inner.lock();
        if matches!(inner.state, State::Sent) {
            debug!(
                round = highest_strong_links_round,
                "adaptive responsive: already sent"
            );
            return;
        }
        let new_round = highest_strong_links_round + 1;
        if matches!(inner.state, State::Initial) {
            observe_round(
                self.dag_id,
                inner.start_time.as_micros() as u64,
                RoundStage::StrongLinkReceived,
            );
        }
        let voting_power = self
            .epoch_state
            .verifier
            .sum_voting_power(strong_links.iter().map(|cert| cert.metadata().author()))
            .expect("Unable to sum voting power from strong links");

        let (wait_time, is_health_backoff) = if self.minimal_wait_time < health_backoff_delay {
            (health_backoff_delay, true)
        } else {
            (self.minimal_wait_time, false)
        };

        debug!(
            voting_power = voting_power,
            wait_power = self.wait_voting_power,
            round = highest_strong_links_round,
            is_health_backoff = is_health_backoff,
            "adaptive responsive: check for new round"
        );

        // voting power >= 90% and pass wait time if health backoff
        let duration_since_start = duration_since_epoch().saturating_sub(inner.start_time);
        if voting_power >= self.wait_voting_power
            && (duration_since_start >= wait_time || !is_health_backoff)
        {
            if voting_power >= self.wait_voting_power {
                debug!(round = highest_strong_links_round, "voting power met");
                observe_round(
                    self.dag_id,
                    inner.start_time.as_micros() as u64,
                    RoundStage::VotingPowerMet,
                );
            }
            debug!(
                round = highest_strong_links_round,
                "sending new round notification"
            );
            let _ = self.event_sender.send(new_round);
            if let State::Scheduled(handle) = std::mem::replace(&mut inner.state, State::Sent) {
                handle.abort();
            }
        } else if matches!(inner.state, State::Initial) {
            // wait until minimal time reaches before sending
            let sender = self.event_sender.clone();
            let wait_time = wait_time.saturating_sub(duration_since_start);
            debug!(
                round = highest_strong_links_round,
                "waiting timeout for round"
            );
            let handle = tokio::spawn(async move {
                tokio::time::sleep(wait_time).await;
                counters::TIMEOUT_WAIT_VOTING_POWER_COUNT.inc();
                let _ = sender.send(new_round);
            });
            inner.state = State::Scheduled(handle);
        }
    }

    fn reset(&self) {
        let mut inner = self.inner.lock();

        inner.start_time = duration_since_epoch();
        inner.state = State::Initial;
    }
}
