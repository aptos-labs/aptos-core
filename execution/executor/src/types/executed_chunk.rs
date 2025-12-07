// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![forbid(unsafe_code)]

use crate::types::partial_state_compute_result::PartialStateComputeResult;
use aptos_types::ledger_info::LedgerInfoWithSignatures;

#[derive(Debug)]
pub struct ExecutedChunk {
    pub output: PartialStateComputeResult,
    pub ledger_info_opt: Option<LedgerInfoWithSignatures>,
}
