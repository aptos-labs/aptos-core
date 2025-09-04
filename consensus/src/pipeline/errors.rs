// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use velor_types::block_info::BlockInfo;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, Error, PartialEq, Eq, Serialize)]
/// Different reasons of errors in commit phase
pub enum Error {
    #[error("The block in the message, {0}, does not match expected block, {1}")]
    InconsistentBlockInfo(BlockInfo, BlockInfo),
    #[error("Verification Error")]
    VerificationError,
    #[error("Reset host dropped")]
    ResetDropped,
    #[error("Rand Reset host dropped")]
    RandResetDropped,
}
