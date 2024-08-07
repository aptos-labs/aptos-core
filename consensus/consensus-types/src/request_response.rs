// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{Payload, PayloadFilter},
    utils::PayloadTxnsSize,
};
use anyhow::Result;
use futures::channel::oneshot;
use std::{fmt, fmt::Formatter, time::Duration};

#[derive(Debug)]
pub struct GetPayloadRequest {
    // max txns
    pub max_txns: PayloadTxnsSize,
    pub max_unique_txns: u64,
    // target txns with opt batches in max_txns as pct
    pub opt_batch_txns_pct: u8,
    // max inline txns
    pub max_inline_txns: PayloadTxnsSize,
    // return non full
    pub return_non_full: bool,
    // block payloads to exclude from the requested block
    pub filter: PayloadFilter,
    // callback to respond to
    pub callback: oneshot::Sender<Result<GetPayloadResponse>>,
    // block timestamp
    pub block_timestamp: Duration,
}

pub enum GetPayloadCommand {
    /// Request to pull block to submit to consensus.
    GetPayloadRequest(GetPayloadRequest),
}

impl fmt::Display for GetPayloadCommand {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            GetPayloadCommand::GetPayloadRequest(request) => {
                write!(f,
                    "GetPayloadRequest [max_txns: {}, max_unique_txns: {}, max_inline_txns: {}, return_non_full: {}, block_timestamp: {:?}]",
                    request.max_txns, request.max_unique_txns, request.max_inline_txns, request.return_non_full, request.block_timestamp
                )
            },
        }
    }
}

#[derive(Debug)]
pub enum GetPayloadResponse {
    GetPayloadResponse(Payload),
}
