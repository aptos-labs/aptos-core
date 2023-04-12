// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::change_set::{DeltaChangeSet, WriteChangeSet};
use aptos_types::{contract_event::ContractEvent, transaction::TransactionStatus};

#[derive(Clone, Debug)]
pub struct TransactionOutput {
    writes: WriteChangeSet,
    deltas: DeltaChangeSet,
    events: Vec<ContractEvent>,
    gas_used: u64,
    status: TransactionStatus,
}

impl TransactionOutput {
    pub fn new(
        writes: WriteChangeSet,
        deltas: DeltaChangeSet,
        events: Vec<ContractEvent>,
        gas_used: u64,
        status: TransactionStatus,
    ) -> Self {
        Self {
            writes,
            deltas,
            events,
            gas_used,
            status,
        }
    }

    pub fn empty_with_status(status: TransactionStatus) -> Self {
        Self {
            writes: WriteChangeSet::empty(),
            deltas: DeltaChangeSet::empty(),
            events: vec![],
            gas_used: 0,
            status,
        }
    }

    pub fn into(self) -> (WriteChangeSet, DeltaChangeSet, Vec<ContractEvent>) {
        (self.writes, self.deltas, self.events)
    }

    pub fn unpack(
        self,
    ) -> (
        WriteChangeSet,
        DeltaChangeSet,
        Vec<ContractEvent>,
        u64,
        TransactionStatus,
    ) {
        (
            self.writes,
            self.deltas,
            self.events,
            self.gas_used,
            self.status,
        )
    }

    pub fn writes(&self) -> &WriteChangeSet {
        &self.writes
    }

    pub fn deltas(&self) -> &DeltaChangeSet {
        &self.deltas
    }

    pub fn events(&self) -> &[ContractEvent] {
        &self.events
    }

    pub fn gas_used(&self) -> u64 {
        self.gas_used
    }

    pub fn status(&self) -> &TransactionStatus {
        &self.status
    }
}
