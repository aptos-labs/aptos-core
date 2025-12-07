// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use serde::{Deserialize, Serialize};
use std::fmt;

/// Request to get a EpochChangeProof from current_epoch to target_epoch
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EpochRetrievalRequest {
    pub start_epoch: u64,
    pub end_epoch: u64,
}

impl fmt::Display for EpochRetrievalRequest {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "EpochRetrievalRequest: start_epoch {}, end_epoch {}",
            self.start_epoch, self.end_epoch
        )
    }
}
