// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    framework::{
        module_network::{ModuleEventTrait, ModuleId},
        NodeId,
    },
    metrics,
    raptr::types::*,
};
use aptos_bitvec::BitVec;
use aptos_consensus_types::common::Author;
use std::{any::Any, collections::HashSet, fmt::Debug, future::Future, time::SystemTime};
use tokio::time::Instant;

#[cfg(all(feature = "sim-types", not(feature = "force-aptos-types")))]
pub mod native;

#[cfg(all(feature = "sim-types", not(feature = "force-aptos-types")))]
pub mod penalty_tracker;

/// Event sent by the consensus module to the dissemination layer to notify of a new block.
#[derive(Debug)]
pub struct ProposalReceived {
    pub leader: NodeId,
    pub round: Round,
    pub payload: Payload,
}

impl ModuleEventTrait for ProposalReceived {
    fn as_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

#[derive(Debug)]
pub struct NewQCWithPayload {
    pub payload: Payload,
    pub qc: QC,
}

impl ModuleEventTrait for NewQCWithPayload {
    fn as_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

/// Event sent by the consensus module to the dissemination layer to notify that it should stop.
#[derive(Debug)]
pub struct Kill();

impl ModuleEventTrait for Kill {
    fn as_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

/// Event sent by the dissemination layer to the consensus module in response to `ProposalReceived`
/// to notify that all data from the proposed block is available.
#[derive(Debug)]
pub struct FullBlockAvailable {
    pub round: Round,
}

impl ModuleEventTrait for FullBlockAvailable {
    fn as_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

pub trait DisseminationLayer: Send + Sync + 'static {
    fn module_id(&self) -> ModuleId;

    fn prepare_block(
        &self,
        round: Round,
        exclude: HashSet<BatchInfo>,
        exclude_authors: Option<BitVec>,
    ) -> impl Future<Output = Payload> + Send;

    fn available_prefix(
        &self,
        payload: &Payload,
        cached_value: Prefix,
    ) -> impl Future<Output = (Prefix, BitVec)> + Send;

    fn notify_commit(
        &self,
        payloads: Vec<Payload>,
        block_timestamp: u64,
        voters: Option<BitVec>,
    ) -> impl Future<Output = ()> + Send;

    fn check_payload(&self, payload: &Payload) -> Result<(), BitVec> {
        Ok(())
    }

    fn set_first_committed_block_timestamp(
        &self,
        timestamp: SystemTime,
    ) -> impl Future<Output = ()> + Send;
}

pub struct Metrics {
    pub batch_commit_time: Option<metrics::UnorderedSender<(Instant, f64)>>,
    pub batch_execute_time: Option<metrics::UnorderedSender<(Instant, f64)>>,
    pub queueing_time: Option<metrics::UnorderedSender<(Instant, f64)>>,
    pub penalty_wait_time: Option<metrics::UnorderedSender<(Instant, f64)>>,
    pub fetch_wait_time_after_commit: Option<metrics::UnorderedSender<(Instant, f64)>>,
    // pub average_penalty: Option<metrics::UnorderedSender<(Instant, f64)>>,
    // pub total_committed_batches: Option<metrics::UnorderedSender<(Instant, usize)>>,
    // pub two_chain_commit_batches: Option<metrics::UnorderedSender<(Instant, usize)>>,
    // pub order_vote_committed_batches: Option<metrics::UnorderedSender<(Instant, usize)>>,
    // pub committed_acs: Option<metrics::UnorderedSender<(Instant, usize)>>,
    // pub optimistically_committed_batches: Option<metrics::UnorderedSender<(Instant, usize)>>,
}
