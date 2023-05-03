// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::common::{Payload, PayloadFilter};
use anyhow::Result;
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
                excluded,
                _,
            ) => {
                write!(
                    f,
                    "GetPayloadRequest [max_txns: {}, max_bytes: {}, return_non_full: {},  excluded: {}]",
                    max_txns, max_bytes, return_non_full, excluded
                )
            },
        }
    }
}

#[derive(Debug)]
pub enum GetPayloadResponse {
    GetPayloadResponse(Payload),
}
