// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_consensus_types::common::{Payload, PayloadFilter};
use futures::channel::oneshot;
use std::{fmt, fmt::Formatter};

pub enum GetPayloadCommand {
    /// Request to pull block to submit to consensus.
    GetPayloadRequest(
        // max block size
        u64,
        // max byte size
        u64,
        // return non full
        bool,
        // block payloads to exclude from the requested block
        PayloadFilter,
        // TODO: possibly add block reader here
        // Arc<dyn BlockReader + Send + Sync>,
        // callback to respond to
        oneshot::Sender<Result<GetPayloadResponse>>,
    ),
}

impl fmt::Display for GetPayloadCommand {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            GetPayloadCommand::GetPayloadRequest(
                max_txns,
                max_bytes,
                return_non_full,
                _excluded,
                _,
            ) => {
                write!(
                    f,
                    "GetPayloadRequest [max_txns: {}, max_bytes: {}, return_non_full: {}]",
                    max_txns, max_bytes, return_non_full,
                )
            },
        }
    }
}

#[derive(Debug)]
pub enum GetPayloadResponse {
    GetPayloadResponse(Payload),
}
