// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::anchor_election::TChainHealthBackoff;
use crate::dag::{
    observability::tracing::{observe_round, RoundStage},
    types::NodeCertificate,
};
use aptos_consensus_types::common::Round;
use aptos_infallible::duration_since_epoch;
use aptos_types::epoch_state::EpochState;
use async_trait::async_trait;
use std::{cmp::Ordering, sync::Arc, time::Duration};
use tokio::task::JoinHandle;

pub struct RoundState {
    current_round: Round,
    event_sender: tokio::sync::mpsc::Sender<Round>,
    responsive_check: Box<dyn ResponsiveCheck>,
}

impl RoundState {
    pub fn new(
        event_sender: tokio::sync::mpsc::Sender<Round>,
        responsive_check: Box<dyn ResponsiveCheck>,
    ) -> Self {
        Self {
            current_round: 0,
            event_sender,
            responsive_check,
        }
    }

    pub async fn check_for_new_round(
        &mut self,
        highest_strong_links_round: Round,
        strong_links: Vec<NodeCertificate>,
    ) {
        match self.current_round.cmp(&highest_strong_links_round) {
            // we're behind, move forward immediately
            Ordering::Less => {
                // the receiver can be dropped if we move to a new epoch
                let _ = self.event_sender.send(highest_strong_links_round + 1).await;
            },
            Ordering::Equal => {
                self.responsive_check
                    .check_for_new_round(highest_strong_links_round, strong_links)
                    .await
            },
            Ordering::Greater => (),
        }
    }

    pub fn current_round(&self) -> Round {
        self.current_round
    }

    pub fn set_current_round(&mut self, new_round: Round) {
        self.current_round = new_round;
        self.responsive_check.reset();
    }
}

/// Interface to decide if we should move forward to a new round
#[async_trait]
pub trait ResponsiveCheck: Send {
    async fn check_for_new_round(
        &mut self,
        highest_strong_links_round: Round,
        strong_links: Vec<NodeCertificate>,
    );

    fn reset(&mut self);
}

/// Move as fast as 2f+1
pub struct OptimisticResponsive {
    event_sender: tokio::sync::mpsc::Sender<Round>,
}

impl OptimisticResponsive {
    pub fn new(event_sender: tokio::sync::mpsc::Sender<Round>) -> Self {
        Self { event_sender }
    }
}

#[async_trait]
impl ResponsiveCheck for OptimisticResponsive {
    async fn check_for_new_round(
        &mut self,
        highest_strong_links_round: Round,
        _strong_links: Vec<NodeCertificate>,
    ) {
        let new_round = highest_strong_links_round + 1;
        let _ = self.event_sender.send(new_round).await;
    }

    fn reset(&mut self) {}
}

enum State {
    Initial,
    Scheduled(JoinHandle<()>),
    Sent,
}

/// More sophisticated strategy to move round forward given 2f+1 strong links
/// Delay if backpressure is triggered. (TODO)
/// Move as soon as 3f+1 is ready. (TODO: make it configurable)
/// Move if minimal wait time is reached. (TODO: make it configurable)
pub struct AdaptiveResponsive {
    epoch_state: Arc<EpochState>,
    start_time: Duration,
    minimal_wait_time: Duration,
    event_sender: tokio::sync::mpsc::Sender<Round>,
    state: State,
    chain_backoff: Arc<dyn TChainHealthBackoff>,
}

impl AdaptiveResponsive {
    pub fn new(
        event_sender: tokio::sync::mpsc::Sender<Round>,
        epoch_state: Arc<EpochState>,
        minimal_wait_time: Duration,
        chain_backoff: Arc<dyn TChainHealthBackoff>,
    ) -> Self {
        Self {
            epoch_state,
            start_time: duration_since_epoch(),
            minimal_wait_time,
            event_sender,
            state: State::Initial,
            chain_backoff,
        }
    }
}

#[async_trait]
impl ResponsiveCheck for AdaptiveResponsive {
    async fn check_for_new_round(
        &mut self,
        highest_strong_links_round: Round,
        strong_links: Vec<NodeCertificate>,
    ) {
        if matches!(self.state, State::Sent) {
            return;
        }
        let new_round = highest_strong_links_round + 1;
        observe_round(
            self.start_time.as_micros() as u64,
            RoundStage::StrongLinkReceived,
        );
        let voting_power = self
            .epoch_state
            .verifier
            .sum_voting_power(strong_links.iter().map(|cert| cert.metadata().author()))
            .expect("Unable to sum voting power from strong links");

        let (_, backoff_duration) = self.chain_backoff.get_round_backoff(new_round);
        let wait_time = if let Some(duration) = backoff_duration {
            duration.max(self.minimal_wait_time)
        } else {
            self.minimal_wait_time
        };

        // voting power == 3f+1 or pass minimal wait time
        let duration_since_start = duration_since_epoch().saturating_sub(self.start_time);
        if voting_power == self.epoch_state.verifier.total_voting_power()
            || duration_since_start >= wait_time
        {
            let _ = self.event_sender.send(new_round).await;
            if let State::Scheduled(handle) = std::mem::replace(&mut self.state, State::Sent) {
                handle.abort();
            }
        } else if matches!(self.state, State::Initial) {
            // wait until minimal time reaches before sending
            let sender = self.event_sender.clone();
            let wait_time = wait_time.saturating_sub(duration_since_start);
            let handle = tokio::spawn(async move {
                tokio::time::sleep(wait_time).await;
                let _ = sender.send(new_round).await;
            });
            self.state = State::Scheduled(handle);
        }
    }

    fn reset(&mut self) {
        self.start_time = duration_since_epoch();
        self.state = State::Initial;
    }
}
