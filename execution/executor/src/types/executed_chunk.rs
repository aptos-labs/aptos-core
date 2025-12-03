#![forbid(unsafe_code)] // Copyright (c) Aptos Foundation
                        // Copyright (c) Aptos Foundation
                        // SPDX-License-Identifier: Innovation-Enabling Source Code License

// SPDX-License-Identifier: Innovation-Enabling Source Code License

use crate::types::partial_state_compute_result::PartialStateComputeResult;
use aptos_types::ledger_info::LedgerInfoWithSignatures;

#[derive(Debug)]
pub struct ExecutedChunk {
    pub output: PartialStateComputeResult,
    pub ledger_info_opt: Option<LedgerInfoWithSignatures>,
}
