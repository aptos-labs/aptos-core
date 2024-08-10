// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::common::{Payload, PayloadFilter};
use anyhow::Result;
use futures::channel::oneshot;
use std::{fmt, fmt::Formatter, time::Duration};

pub enum GetPayloadCommand {
    /// Request to pull block to submit to consensus.
    GetPayloadRequest(
        // max number of transactions in the block
        u64,
        // max number of transactions after filtering in the block
        u64,
        // soft max number of transactions after filtering in the block (i.e. include one that crosses it)
        u64,
        // max byte size
        u64,
        // max number of inline transactions (transactions without a proof of store)
        u64,
        // max byte size of inline transactions (transactions without a proof of store)
        u64,
        // return non full
        bool,
        // block payloads to exclude from the requested block
        PayloadFilter,
        // callback to respond to
        oneshot::Sender<Result<GetPayloadResponse>>,
        // block timestamp
        Duration,
    ),
}

impl fmt::Display for GetPayloadCommand {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            GetPayloadCommand::GetPayloadRequest(
                max_txns,
                max_txns_after_filtering,
                soft_max_txns_after_filtering,
                max_bytes,
                max_inline_txns,
                max_inline_bytes,
                return_non_full,
                excluded,
                _,
                block_timestamp,
            ) => {
                write!(
                    f,
                    "GetPayloadRequest [max_txns: {}, max_txns_after_filtering: {} (soft: {}), max_bytes: {}, max_inline_txns: {}, max_inline_bytes:{}, return_non_full: {},  excluded: {}, block_timestamp: {:?}]",
                    max_txns, max_txns_after_filtering, soft_max_txns_after_filtering, max_bytes, max_inline_txns, max_inline_bytes, return_non_full, excluded, block_timestamp
                )
            },
        }
    }
}

#[derive(Debug)]
pub enum GetPayloadResponse {
    GetPayloadResponse(Payload),
}
