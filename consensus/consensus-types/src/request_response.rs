// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::{Payload, PayloadFilter};
use crate::proof_of_store::LogicalTime;
use anyhow::Result;
use aptos_crypto::HashValue;
use futures::channel::oneshot;
use std::{fmt, fmt::Formatter};

/// Message sent from Consensus to QuorumStore.
pub enum ConsensusRequest {
    /// Request to pull block to submit to consensus.
    GetBlockRequest(
        // max block size
        u64,
        // block payloads to exclude from the requested block
        PayloadFilter,
        // callback to respond to
        oneshot::Sender<Result<ConsensusResponse>>,
    ),
    CleanRequest(LogicalTime, Vec<HashValue>),
}

impl fmt::Display for ConsensusRequest {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ConsensusRequest::GetBlockRequest(block_size, excluded, _) => {
                write!(
                    f,
                    "GetBlockRequest [block_size: {}, excluded: {}]",
                    block_size, excluded
                )
            }
            ConsensusRequest::CleanRequest(logical_time, digests) => {
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

#[derive(Debug)]
pub enum ConsensusResponse {
    GetBlockResponse(Payload),
}
