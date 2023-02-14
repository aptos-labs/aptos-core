// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::in_memory_state_calculator::NEW_EPOCH_EVENT_KEY;
use aptos_types::{
    contract_event::ContractEvent,
    transaction::{TransactionOutput, TransactionStatus},
    write_set::WriteSet,
};
use std::ops::Deref;

pub struct ParsedTransactionOutput {
    output: TransactionOutput,
    reconfig_events: Vec<ContractEvent>,
}

impl ParsedTransactionOutput {
    pub fn parse_reconfig_events(events: &[ContractEvent]) -> impl Iterator<Item = &ContractEvent> {
        events.iter().filter(|e| *e.key() == *NEW_EPOCH_EVENT_KEY)
    }
}

impl From<TransactionOutput> for ParsedTransactionOutput {
    fn from(output: TransactionOutput) -> Self {
        let reconfig_events = Self::parse_reconfig_events(output.events())
            .cloned()
            .collect();
        Self {
            output,
            reconfig_events,
        }
    }
}

impl Deref for ParsedTransactionOutput {
    type Target = TransactionOutput;

    fn deref(&self) -> &Self::Target {
        &self.output
    }
}

impl ParsedTransactionOutput {
    pub fn is_reconfig(&self) -> bool {
        !self.reconfig_events.is_empty()
    }

    pub fn unpack(
        self,
    ) -> (
        WriteSet,
        Vec<ContractEvent>,
        Vec<ContractEvent>,
        u64,
        TransactionStatus,
    ) {
        let Self {
            output,
            reconfig_events,
        } = self;
        let (write_set, events, gas_used, status) = output.unpack();

        (write_set, events, reconfig_events, gas_used, status)
    }
}
