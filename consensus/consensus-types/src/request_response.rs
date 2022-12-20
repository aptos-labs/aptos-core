// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::{Payload, PayloadFilter, Round};
use crate::proof_of_store::LogicalTime;
use anyhow::Result;
use aptos_crypto::HashValue;
use futures::channel::oneshot;
use std::{fmt, fmt::Formatter};

/// Message sent from Consensus to QuorumStore.
pub enum PayloadRequest {
    /// Request to pull block to submit to consensus.
    GetBlockRequest(
        Round,
        // max block size
        u64,
        // max byte size
        u64,
        // block payloads to exclude from the requested block
        PayloadFilter,
        // callback to respond to
        oneshot::Sender<Result<ConsensusResponse>>,
    ),
    /// Request to clean quorum store at commit logical time
    CleanRequest(LogicalTime, Vec<HashValue>),
}

impl fmt::Display for PayloadRequest {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            PayloadRequest::GetBlockRequest(round, max_txns, max_bytes, excluded, _) => {
                write!(
                    f,
                    "GetBlockRequest [round: {}, max_txns: {}, max_bytes: {} excluded: {}]",
                    round, max_txns, max_bytes, excluded
                )
            }
            PayloadRequest::CleanRequest(logical_time, digests) => {
                write!(
                    f,
                    "CleanRequest [epoch: {}, round: {}, digests: {:?}]",
                    logical_time.epoch(),
                    logical_time.round(),
                    digests
                )
            }
        }
    }
}

pub enum ConsensusResponse {
    GetBlockResponse(Payload),
}
