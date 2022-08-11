// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::{Payload, PayloadFilter, Round};
use anyhow::Result;
use futures::channel::oneshot;
use std::{fmt, fmt::Formatter};

/// Message sent from Consensus to QuorumStore.
pub enum ConsensusRequest {
    /// Request to pull block to submit to consensus.
    GetBlockRequest(
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
    CleanRequest(
        // epoch
        u64,
        // round
        Round,
        // callback to respond to
        oneshot::Sender<Result<ConsensusResponse>>,
    ),
}

impl fmt::Display for ConsensusRequest {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ConsensusRequest::GetBlockRequest(max_txns, max_bytes, excluded, _) => {
                write!(
                    f,
                    "GetBlockRequest [max_txns: {}, max_bytes: {} excluded: {}]",
                    max_txns, max_bytes, excluded
                )
            }
            ConsensusRequest::CleanRequest(epoch, round, _) => {
                write!(f, "CleanRequest [epoch: {}, round: {}]", epoch, round)
            }
        }
    }
}

pub enum ConsensusResponse {
    GetBlockResponse(Payload),
    CleanResponse(),
}
