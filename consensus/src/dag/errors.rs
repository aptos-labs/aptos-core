// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_bitvec::BitVec;
use aptos_consensus_types::common::Round;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use thiserror::Error as ThisError;

#[derive(Clone, ThisError, Debug, Serialize, Deserialize)]
pub enum DagFetchError {
    #[error("fetch failed")]
    Failed,
    #[error("already exists")]
    AlreadyExists,
}

#[derive(Clone, ThisError, Debug, Serialize, Deserialize)]
pub enum NodeBroadcastHandleError {
    #[error("invalid parent in node")]
    InvalidParent,
    #[error("missing parents")]
    MissingParents,
    #[error("stale round number")]
    StaleRound(Round),
    #[error("refused to vote")]
    VoteRefused,
}

#[derive(Clone, Debug, ThisError, Serialize, Deserialize)]
pub enum DagDriverError {
    #[error("missing parents")]
    MissingParents,
    #[error("payload not found")]
    PayloadNotFound,
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
    dag_id: u8,
}

impl DAGRpcError {
    pub fn new(dag_id: u8, epoch: u64, error: DAGError) -> Self {
        Self {
            dag_id,
            epoch,
            error,
        }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn dag_id(&self) -> u8 {
        self.dag_id
    }
}

impl Deref for DAGRpcError {
    type Target = DAGError;

    fn deref(&self) -> &Self::Target {
        &self.error
    }
}
