// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use aptos_executor_types::{
    execution_output::ExecutionOutput, state_checkpoint_output::StateCheckpointOutput,
    state_compute_result::StateComputeResult, LedgerUpdateOutput,
};
use aptos_storage_interface::{
    state_store::{state::State, state_summary::StateSummary},
    LedgerSummary,
};
use once_cell::sync::OnceCell;

#[derive(Clone, Debug)]
pub struct PartialStateComputeResult {
    pub execution_output: ExecutionOutput,
    pub state_checkpoint_output: OnceCell<StateCheckpointOutput>,
    pub ledger_update_output: OnceCell<LedgerUpdateOutput>,
}

impl PartialStateComputeResult {
    pub fn new(execution_output: ExecutionOutput) -> Self {
        Self {
            execution_output,
            state_checkpoint_output: OnceCell::new(),
            ledger_update_output: OnceCell::new(),
        }
    }

    pub fn new_empty(ledger_summary: LedgerSummary) -> Self {
        // Deliberately not reusing Self::new() here to make sure we don't leave
        // any OnceCell unset.
        let execution_output = ExecutionOutput::new_empty(ledger_summary.state);
        let ledger_update_output = OnceCell::new();
        ledger_update_output
            .set(LedgerUpdateOutput::new_empty(
                ledger_summary.transaction_accumulator,
            ))
            .expect("First set.");
        let state_checkpoint_output = OnceCell::new();
        state_checkpoint_output
            .set(StateCheckpointOutput::new_empty(
                ledger_summary.state_summary,
            ))
            .expect("First set.");

        Self {
            execution_output,
            state_checkpoint_output,
            ledger_update_output,
        }
    }

    pub fn has_reconfiguration(&self) -> bool {
        self.execution_output.next_epoch_state.is_some()
    }

    pub fn get_state_checkpoint_output(&self) -> Option<&StateCheckpointOutput> {
        self.state_checkpoint_output.get()
    }

    pub fn expect_state_checkpoint_output(&self) -> &StateCheckpointOutput {
        self.state_checkpoint_output
            .get()
            .expect("StateCheckpointOutput not set")
    }

    pub fn expect_result_state(&self) -> &State {
        &self.execution_output.result_state
    }

    pub fn expect_result_state_summary(&self) -> &StateSummary {
        // FIXME(aldenhu):
        // &self.expect_state_checkpoint_output().result_state_summary
        todo!()
    }

    pub fn set_state_checkpoint_output(&self, state_checkpoint_output: StateCheckpointOutput) {
        self.state_checkpoint_output
            .set(state_checkpoint_output)
            .expect("StateCheckpointOutput already set");
    }

    pub fn get_ledger_update_output(&self) -> Option<&LedgerUpdateOutput> {
        self.ledger_update_output.get()
    }

    pub fn expect_ledger_update_output(&self) -> &LedgerUpdateOutput {
        self.ledger_update_output
            .get()
            .expect("LedgerUpdateOutput not set")
    }

    pub fn set_ledger_update_output(&self, ledger_update_output: LedgerUpdateOutput) {
        self.ledger_update_output
            .set(ledger_update_output)
            .expect("LedgerUpdateOutput already set");
    }

    pub fn get_complete_result(&self) -> Option<StateComputeResult> {
        self.ledger_update_output.get().map(|ledger_update_output| {
            StateComputeResult::new(
                self.execution_output.clone(),
                // ledger_update_output is set in a later stage, so it's safe to `expect` here.
                self.expect_state_checkpoint_output().clone(),
                ledger_update_output.clone(),
            )
        })
    }

    pub fn expect_complete_result(&self) -> StateComputeResult {
        self.get_complete_result().expect("Result is not complete.")
    }
}
