// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    pending_votes::{PendingVotes, VoteReceptionResult},
    util::time_service::TimeService,
};
use aptos_config::config::{DelayedQcAggregatorConfig, QcAggregatorType};
use aptos_consensus_types::{delayed_qc_msg::DelayedQcMsg, vote::Vote};
use aptos_logger::{error, info};
use aptos_types::{
    ledger_info::LedgerInfoWithPartialSignatures, validator_verifier::ValidatorVerifier,
};
use futures::SinkExt;
use futures_channel::mpsc::UnboundedSender;
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;

pub trait QcAggregator: Send + Sync {
    fn handle_aggregated_qc(
        &mut self,
        validator_verifier: &ValidatorVerifier,
        aggregated_voting_power: u128,
        vote: &Vote,
        li_with_sig: &LedgerInfoWithPartialSignatures,
    ) -> VoteReceptionResult;
}

struct NoDelayQcAggregator {}

pub fn create_qc_aggregator(
    qc_aggregator_type: QcAggregatorType,
    time_service: Arc<dyn TimeService>,
    delayed_qc_tx: UnboundedSender<DelayedQcMsg>,
) -> Box<dyn QcAggregator> {
    match qc_aggregator_type {
        QcAggregatorType::NoDelay => Box::new(NoDelayQcAggregator {}),
        QcAggregatorType::Delayed(delay_config) => {
            let DelayedQcAggregatorConfig {
                max_delay_after_round_start_ms,
                aggregated_voting_power_pct_to_wait,
                pct_delay_after_qc_aggregated,
            } = delay_config;
            Box::new(DelayedQcAggregator::new(
                Duration::from_millis(max_delay_after_round_start_ms),
                aggregated_voting_power_pct_to_wait,
                pct_delay_after_qc_aggregated,
                time_service,
                delayed_qc_tx,
            ))
        },
    }
}

impl QcAggregator for NoDelayQcAggregator {
    fn handle_aggregated_qc(
        &mut self,
        validator_verifier: &ValidatorVerifier,
        aggregated_voting_power: u128,
        vote: &Vote,
        li_with_sig: &LedgerInfoWithPartialSignatures,
    ) -> VoteReceptionResult {
        assert!(
            aggregated_voting_power >= validator_verifier.quorum_voting_power(),
            "QC aggregation should not be triggered if we don't have enough votes to form a QC"
        );
        PendingVotes::aggregate_qc_now(validator_verifier, li_with_sig, vote.vote_data())
    }
}

struct DelayedQcAggregator {
    round_start_time: Duration,
    max_delay_after_round_start: Duration,
    aggregated_voting_power_pct_to_wait: usize,
    pct_delay_after_qc_aggregated: usize,
    time_service: Arc<dyn TimeService>,
    // True, if we already have enough vote to aggregate a QC, but we have trigged a delayed QC
    // aggregation event to collect as many votes as possible.
    qc_aggregation_delayed: bool,
    // To send delayed QC aggregation events to the round manager.
    delayed_qc_tx: UnboundedSender<DelayedQcMsg>,
}

impl DelayedQcAggregator {
    pub fn new(
        max_delay_after_round_start: Duration,
        aggregated_voting_power_pct_to_wait: usize,
        pct_delay_after_qc_aggregated: usize,
        time_service: Arc<dyn TimeService>,
        delayed_qc_tx: UnboundedSender<DelayedQcMsg>,
    ) -> Self {
        let round_start_time = time_service.get_current_timestamp();
        Self {
            round_start_time,
            max_delay_after_round_start,
            aggregated_voting_power_pct_to_wait,
            pct_delay_after_qc_aggregated,
            time_service,
            qc_aggregation_delayed: false,
            delayed_qc_tx,
        }
    }
}

impl QcAggregator for DelayedQcAggregator {
    fn handle_aggregated_qc(
        &mut self,
        validator_verifier: &ValidatorVerifier,
        aggregated_voting_power: u128,
        vote: &Vote,
        li_with_sig: &LedgerInfoWithPartialSignatures,
    ) -> VoteReceptionResult {
        assert!(
            aggregated_voting_power >= validator_verifier.quorum_voting_power(),
            "QC aggregation should not be triggered if we don't have enough votes to form a QC"
        );
        let current_time = self.time_service.get_current_timestamp();

        // If we have reached the aggregated voting power threshold, we should aggregate the QC now.
        if aggregated_voting_power
            >= self.aggregated_voting_power_pct_to_wait as u128
                * validator_verifier.total_voting_power()
                / 100
        {
            // Voting power is u128 so there is no overflow here.
            info!(
                "QC aggregation triggered by aggregated voting power: {}",
                aggregated_voting_power
            );
            return PendingVotes::aggregate_qc_now(
                validator_verifier,
                li_with_sig,
                vote.vote_data(),
            );
        }

        // If we have not reached the aggregated voting power threshold and have
        // already triggered a delayed QC aggregation event, we should not trigger another
        // one.
        if self.qc_aggregation_delayed {
            return VoteReceptionResult::VoteAddedQCDelayed(aggregated_voting_power);
        }

        let time_since_round_start = current_time - self.round_start_time;
        if time_since_round_start >= self.max_delay_after_round_start {
            info!(
                "QC aggregation triggered by time: {} ms",
                time_since_round_start.as_millis()
            );
            return PendingVotes::aggregate_qc_now(
                validator_verifier,
                li_with_sig,
                vote.vote_data(),
            );
        }

        let wait_time = (self.max_delay_after_round_start - time_since_round_start)
            .min(time_since_round_start * self.pct_delay_after_qc_aggregated as u32 / 100);

        let delayed_qc_event = DelayedQcMsg::new(vote.clone());
        self.qc_aggregation_delayed = true;

        let mut delayed_qc_sender = self.delayed_qc_tx.clone();

        info!(
            "QC aggregation delayed by {} ms, wait time: {} ms",
            time_since_round_start.as_millis(),
            wait_time.as_millis()
        );

        tokio::spawn(async move {
            sleep(wait_time).await;
            if let Err(e) = delayed_qc_sender.send(delayed_qc_event).await {
                error!("Failed to send event to round manager {:?}", e);
            }
        });

        VoteReceptionResult::VoteAddedQCDelayed(aggregated_voting_power)
    }
}
