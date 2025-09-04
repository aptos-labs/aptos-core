// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::types::partial_state_compute_result::PartialStateComputeResult;
use velor_types::ledger_info::LedgerInfoWithSignatures;

#[derive(Debug)]
pub struct ExecutedChunk {
    pub output: PartialStateComputeResult,
    pub ledger_info_opt: Option<LedgerInfoWithSignatures>,
}
