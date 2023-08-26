// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::LedgerUpdateOutput;
use aptos_storage_interface::state_delta::StateDelta;
use once_cell::sync::OnceCell;

pub struct ExecutionOutput {
    state: StateDelta,
    ledger_update_output: OnceCell<LedgerUpdateOutput>,
}

impl ExecutionOutput {
    pub fn new(state: StateDelta) -> Self {
        Self {
            state,
            ledger_update_output: OnceCell::new(),
        }
    }

    pub fn new_with_ledger_update(
        state: StateDelta,
        ledger_update_output: LedgerUpdateOutput,
    ) -> Self {
        let ledger_update = OnceCell::new();
        ledger_update.set(ledger_update_output).unwrap();
        Self {
            state,
            ledger_update_output: ledger_update,
        }
    }

    pub fn has_ledger_update(&self) -> bool {
        self.ledger_update_output.get().is_some()
    }

    pub fn get_ledger_update(&self) -> &LedgerUpdateOutput {
        self.ledger_update_output.get().unwrap()
    }

    pub fn set_ledger_update(&self, ledger_update_output: LedgerUpdateOutput) {
        self.ledger_update_output
            .set(ledger_update_output)
            .expect("LedgerUpdateOutput already set");
    }

    pub fn next_version(&self) -> u64 {
        self.state().current_version.unwrap()
    }

    pub fn is_same_state(&self, rhs: &Self) -> bool {
        self.state().has_same_current_state(rhs.state())
    }

    pub fn state(&self) -> &StateDelta {
        &self.state
    }
}
