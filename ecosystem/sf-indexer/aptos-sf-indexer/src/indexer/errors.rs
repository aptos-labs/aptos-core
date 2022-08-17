// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::Error;

/// Error, block_height, substream_module_name
type ErrorWithBlockAndName = (Error, u64, &'static str);

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum BlockProcessingError {
    /// Could not parse substream or blocks
    ParsingError(ErrorWithBlockAndName),
    /// Could not get a connection
    ConnectionPoolError(ErrorWithBlockAndName),
    /// Could not commit the block
    BlockCommitError(ErrorWithBlockAndName),
}

impl BlockProcessingError {
    pub fn inner(&self) -> &ErrorWithBlockAndName {
        match self {
            BlockProcessingError::ParsingError(ewb) => ewb,
            BlockProcessingError::ConnectionPoolError(ewb) => ewb,
            BlockProcessingError::BlockCommitError(ewb) => ewb,
        }
    }
}
