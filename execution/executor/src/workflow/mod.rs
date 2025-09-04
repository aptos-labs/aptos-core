// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::types::partial_state_compute_result::PartialStateComputeResult;
use anyhow::Result;
use velor_executor_types::execution_output::ExecutionOutput;
use velor_storage_interface::{
    state_store::state_summary::ProvableStateSummary, DbReader, LedgerSummary,
};
use do_ledger_update::DoLedgerUpdate;
use do_state_checkpoint::DoStateCheckpoint;

pub mod do_get_execution_output;
pub mod do_ledger_update;
pub mod do_state_checkpoint;

pub struct ApplyExecutionOutput;

impl ApplyExecutionOutput {
    pub fn run(
        execution_output: ExecutionOutput,
        base_view: LedgerSummary,
        reader: &(dyn DbReader + Sync),
    ) -> Result<PartialStateComputeResult> {
        let state_checkpoint_output = DoStateCheckpoint::run(
            &execution_output,
            &base_view.state_summary,
            &ProvableStateSummary::new_persisted(reader)?,
            None,
        )?;
        let ledger_update_output = DoLedgerUpdate::run(
            &execution_output,
            &state_checkpoint_output,
            base_view.transaction_accumulator,
        )?;
        let output = PartialStateComputeResult::new(execution_output);
        output.set_state_checkpoint_output(state_checkpoint_output);
        output.set_ledger_update_output(ledger_update_output);

        Ok(output)
    }
}
