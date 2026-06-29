// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![forbid(unsafe_code)]

use crate::types::partial_state_compute_result::PartialStateComputeResult;
use anyhow::Result;
use aptos_executor_types::execution_output::ExecutionOutput;
use aptos_storage_interface::{
    state_store::state_summary::{ProvablePositionStateSummary, ProvableStateSummary},
    DbReader, LedgerSummary,
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
        let position_persisted = execution_output
            .compute_trading_native_state_roots
            .then(|| ProvablePositionStateSummary::new_persisted(reader))
            .transpose()?;
        let state_checkpoint_output = DoStateCheckpoint::run()
            .execution_output(&execution_output)
            .parent_state_summary(&base_view.state_summary)
            .persisted_state_summary(&ProvableStateSummary::new_persisted(reader)?)
            .maybe_parent_position_state_summary(base_view.position_state_summary.as_ref())
            .maybe_persisted_position_state_summary(position_persisted.as_ref())
            .build()?;
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
