// Copyright © Aptos Foundation

use aptos_bitvec::BitVec;
use aptos_consensus_types::common::Round;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use thiserror::Error as ThisError;

#[derive(Clone, ThisError, Debug, Serialize, Deserialize)]
pub enum NodeBroadcastHandleError {
    #[error("invalid parent in node")]
    InvalidParent,
    #[error("missing parents")]
    MissingParents,
    #[error("stale round number")]
    StaleRound(Round),
}

#[derive(Clone, Debug, ThisError, Serialize, Deserialize)]
pub enum DagDriverError {
    #[error("missing parents")]
    MissingParents,
}

#[derive(Clone, Debug, ThisError, Serialize, Deserialize)]
pub enum FetchRequestHandleError {
    #[error("target nodes are missing, missing {}", .0.count_ones())]
    TargetsMissing(BitVec),
    #[error("garbage collected, request round {0}, lowest round {1}")]
    GarbageCollected(Round, Round),
}

#[derive(Clone, Debug, ThisError, Serialize, Deserialize)]
pub enum DAGError {
    #[error(transparent)]
    NodeBroadcastHandleError(NodeBroadcastHandleError),
    #[error(transparent)]
    DagDriverError(DagDriverError),
    #[error(transparent)]
    FetchRequestHandleError(FetchRequestHandleError),
    #[error("unable to verify message")]
    MessageVerificationError,
    #[error("unknown error")]
    Unknown,
}

#[derive(Clone, Debug, ThisError, Serialize, Deserialize)]
#[error("{error}")]
pub struct DAGRpcError {
    error: DAGError,
    epoch: u64,
}

impl DAGRpcError {
    pub fn new(epoch: u64, error: DAGError) -> Self {
        Self { epoch, error }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }
}

impl Deref for DAGRpcError {
    type Target = DAGError;

    fn deref(&self) -> &Self::Target {
        &self.error
    }
}
