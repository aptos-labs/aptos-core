// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{Payload, PayloadFilter},
    payload_pull_params::OptQSPayloadPullParams,
    utils::PayloadTxnsSize,
};
use anyhow::Result;
use futures::channel::oneshot;
use std::{fmt, fmt::Formatter, time::Duration};

pub struct GetPayloadRequest {
    // max number of transactions in the block
    pub max_txns: PayloadTxnsSize,
    // max number of transactions after filtering in the block
    pub max_txns_after_filtering: u64,
    // soft max number of transactions after filtering in the block (i.e. include one that crosses it)
    pub soft_max_txns_after_filtering: u64,
    // opt payload pull params
    pub maybe_optqs_payload_pull_params: Option<OptQSPayloadPullParams>,
    // max number of inline transactions (transactions without a proof of store)
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
                    "GetPayloadRequest [max_txns: {}, max_txns_after_filtering: {}, soft_max_txns_after_filtering: {}, max_inline_txns: {}, return_non_full: {}, block_timestamp: {:?}]",
                    request.max_txns, request.max_txns_after_filtering, request.soft_max_txns_after_filtering, request.max_inline_txns, request.return_non_full, request.block_timestamp
                )
            },
        }
    }
}

#[derive(Debug)]
pub enum GetPayloadResponse {
    GetPayloadResponse(Payload),
}
