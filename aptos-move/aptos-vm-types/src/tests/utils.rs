// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    change_set::VMChangeSet,
    check_change_set::CheckChangeSet,
    output::VMOutput, abstract_write_op::{AbstractResourceWriteOp, WriteWithDelayedFieldsOp},
};
use aptos_aggregator::{
    delayed_change::DelayedChange,
    delta_change_set::{delta_add, serialize, DeltaOp},
    types::DelayedFieldID,
};
use aptos_types::{
    account_address::AccountAddress,
    fee_statement::FeeStatement,
    on_chain_config::CurrentTimeMicroseconds,
    state_store::{state_key::StateKey, state_value::StateValueMetadata},
    transaction::{ExecutionStatus, TransactionStatus},
    write_set::{WriteOp, WriteOpSize},
};
use move_core_types::{
    identifier::Identifier,
    language_storage::{StructTag, TypeTag},
    value::MoveTypeLayout,
    vm_status::VMStatus,
};
use std::{collections::BTreeMap, sync::Arc};

pub(crate) struct MockChangeSetChecker;

impl CheckChangeSet for MockChangeSetChecker {
    fn check_change_set(&self, _change_set: &VMChangeSet) -> anyhow::Result<(), VMStatus> {
        Ok(())
    }
}

macro_rules! as_state_key {
    ($k:ident) => {
        StateKey::raw($k.to_string().into_bytes())
    };
}
pub(crate) use as_state_key;

macro_rules! as_bytes {
    ($v:ident) => {
        serialize(&$v)
    };
}

pub(crate) fn raw_metadata(v: u64) -> StateValueMetadata {
    StateValueMetadata::new(v, &CurrentTimeMicroseconds { microseconds: v })
}

pub(crate) fn mock_create(k: impl ToString, v: u128) -> (StateKey, WriteOp) {
    (as_state_key!(k), WriteOp::Creation(as_bytes!(v).into()))
}

pub(crate) fn mock_modify(k: impl ToString, v: u128) -> (StateKey, WriteOp) {
    (as_state_key!(k), WriteOp::Modification(as_bytes!(v).into()))
}

pub(crate) fn mock_delete(k: impl ToString) -> (StateKey, WriteOp) {
    (as_state_key!(k), WriteOp::Deletion)
}

fn with_layout_to_abstract(
    write_op: WriteOp,
    layout: Option<Arc<MoveTypeLayout>>,
) -> AbstractResourceWriteOp {
    if let Some(layout) = layout {
        let materialized_size = WriteOpSize::from(&write_op).write_len();
        AbstractResourceWriteOp::WriteWithDelayedFields(WriteWithDelayedFieldsOp {
            write_op,
            layout,
            materialized_size,
        })
    } else {
        AbstractResourceWriteOp::Write(write_op)
    }
}

pub(crate) fn mock_create_with_layout(
    k: impl ToString,
    v: u128,
    layout: Option<Arc<MoveTypeLayout>>,
) -> (StateKey, AbstractResourceWriteOp) {
    (
        as_state_key!(k),
        with_layout_to_abstract(WriteOp::Creation(as_bytes!(v).into()), layout),
    )
}

pub(crate) fn mock_modify_with_layout(
    k: impl ToString,
    v: u128,
    layout: Option<Arc<MoveTypeLayout>>,
) -> (StateKey, AbstractResourceWriteOp) {
    (
        as_state_key!(k),
        with_layout_to_abstract(WriteOp::Modification(as_bytes!(v).into()), layout),
    )
}

pub(crate) fn mock_delete_with_layout(k: impl ToString) -> (StateKey, AbstractResourceWriteOp) {
    (
        as_state_key!(k),
        with_layout_to_abstract(WriteOp::Deletion, None),
    )
}

pub(crate) fn mock_add(k: impl ToString, v: u128) -> (StateKey, DeltaOp) {
    const DUMMY_LIMIT: u128 = 1000;
    (as_state_key!(k), delta_add(v, DUMMY_LIMIT))
}

pub(crate) fn mock_tag_0() -> StructTag {
    StructTag {
        address: AccountAddress::ONE,
        module: Identifier::new("a").unwrap(),
        name: Identifier::new("a").unwrap(),
        type_params: vec![TypeTag::U8],
    }
}

pub(crate) fn mock_tag_1() -> StructTag {
    StructTag {
        address: AccountAddress::ONE,
        module: Identifier::new("abcde").unwrap(),
        name: Identifier::new("fgh").unwrap(),
        type_params: vec![TypeTag::U64],
    }
}

pub(crate) fn mock_tag_2() -> StructTag {
    StructTag {
        address: AccountAddress::ONE,
        module: Identifier::new("abcdex").unwrap(),
        name: Identifier::new("fghx").unwrap(),
        type_params: vec![TypeTag::U128],
    }
}

pub(crate) struct VMChangeSetBuilder {
    resource_write_set: BTreeMap<StateKey, AbstractResourceWriteOp>,
    module_write_set: BTreeMap<StateKey, WriteOp>,
    events: Vec<(ContractEvent, Option<MoveTypeLayout>)>,
    delayed_field_change_set: BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>>,
    aggregator_v1_write_set: BTreeMap<StateKey, WriteOp>,
    aggregator_v1_delta_set: BTreeMap<StateKey, DeltaOp>,
}

impl VMChangeSetBuilder {
    pub(crate) fn new() -> Self {
        Self {
            resource_write_set: BTreeMap::new(),
            module_write_set: BTreeMap::new(),
            events: vec![],
            delayed_field_change_set: BTreeMap::new(),
            aggregator_v1_write_set: BTreeMap::new(),
            aggregator_v1_delta_set: BTreeMap::new(),
        }
    }

    pub(crate) fn with_resource_write_set(
        mut self,
        resource_write_set: impl IntoIterator<Item = (StateKey, AbstractResourceWriteOp)>,
    ) -> Self {
        assert!(self.resource_write_set.is_empty());
        self.resource_write_set.extend(resource_write_set);
        self
    }

    pub(crate) fn with_module_write_set(
        mut self,
        module_write_set: impl IntoIterator<Item = (StateKey, WriteOp)>,
    ) -> Self {
        assert!(self.module_write_set.is_empty());
        self.module_write_set.extend(module_write_set);
        self
    }

    pub(crate) fn with_events(
        mut self,
        events: impl IntoIterator<Item = (ContractEvent, Option<MoveTypeLayout>)>,
    ) -> Self {
        assert!(self.events.is_empty());
        self.events.extend(events);
        self
    }

    pub(crate) fn with_delayed_field_change_set(
        mut self,
        delayed_field_change_set: impl IntoIterator<Item = (DelayedFieldID, DelayedChange<DelayedFieldID>)>,
    ) -> Self {
        assert!(self.delayed_field_change_set.is_empty());
        self.delayed_field_change_set.extend(delayed_field_change_set);
        self
    }

    pub(crate) fn with_aggregator_v1_write_set(
        mut self,
        aggregator_v1_write_set: impl IntoIterator<Item = (StateKey, WriteOp)>,
    ) -> Self {
        assert!(self.aggregator_v1_write_set.is_empty());
        self.aggregator_v1_write_set.extend(aggregator_v1_write_set);
        self
    }

    pub(crate) fn with_aggregator_v1_delta_set(
        mut self,
        aggregator_v1_delta_set: impl IntoIterator<Item = (StateKey, DeltaOp)>,
    ) -> Self {
        assert!(self.aggregator_v1_delta_set.is_empty());
        self.aggregator_v1_delta_set.extend(aggregator_v1_delta_set);
        self
    }

    pub(crate) fn build(self) -> VMChangeSet {
        VMChangeSet::new(
            self.resource_write_set,
            self.module_write_set,
            self.events,
            self.delayed_field_change_set,
            self.aggregator_v1_write_set,
            self.aggregator_v1_delta_set,
            &MockChangeSetChecker,
        )
        .unwrap()
    }
}

// For testing, output has always a success execution status and uses 100 gas units.
pub(crate) fn build_vm_output(
    resource_write_set: impl IntoIterator<Item = (StateKey, AbstractResourceWriteOp)>,
    module_write_set: impl IntoIterator<Item = (StateKey, WriteOp)>,
    delayed_field_change_set: impl IntoIterator<Item = (DelayedFieldID, DelayedChange<DelayedFieldID>)>,
    aggregator_v1_write_set: impl IntoIterator<Item = (StateKey, WriteOp)>,
    aggregator_v1_delta_set: impl IntoIterator<Item = (StateKey, DeltaOp)>,
) -> VMOutput {
    const GAS_USED: u64 = 100;
    const STATUS: TransactionStatus = TransactionStatus::Keep(ExecutionStatus::Success);
    VMOutput::new(
        VMChangeSetBuilder::new()
            .with_resource_write_set(resource_write_set)
            .with_module_write_set(module_write_set)
            .with_delayed_field_change_set(delayed_field_change_set)
            .with_aggregator_v1_write_set(aggregator_v1_write_set)
            .with_aggregator_v1_delta_set(aggregator_v1_delta_set)
            .build(),
        FeeStatement::new(GAS_USED, GAS_USED, 0, 0, 0),
        STATUS,
    )
}

pub(crate) struct ExpandedVMChangeSetBuilder {
    resource_write_set: BTreeMap<StateKey, (WriteOp, Option<Arc<MoveTypeLayout>>)>,
    resource_group_write_set: BTreeMap<StateKey, GroupWrite>,
    module_write_set: BTreeMap<StateKey, WriteOp>,
    aggregator_v1_write_set: BTreeMap<StateKey, WriteOp>,
    aggregator_v1_delta_set: BTreeMap<StateKey, DeltaOp>,
    delayed_field_change_set: BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>>,
    reads_needing_delayed_field_exchange: BTreeMap<StateKey, (WriteOp, Arc<MoveTypeLayout>)>,
    group_reads_needing_delayed_field_exchange: BTreeMap<StateKey, (WriteOp, u64)>,
    events: Vec<(ContractEvent, Option<MoveTypeLayout>)>,
}

impl ExpandedVMChangeSetBuilder {
    pub(crate) fn new() -> Self {
        Self {
            resource_write_set: BTreeMap::new(),
            resource_group_write_set: BTreeMap::new(),
            module_write_set: BTreeMap::new(),
            aggregator_v1_write_set: BTreeMap::new(),
            aggregator_v1_delta_set: BTreeMap::new(),
            delayed_field_change_set: BTreeMap::new(),
            reads_needing_delayed_field_exchange: BTreeMap::new(),
            group_reads_needing_delayed_field_exchange: BTreeMap::new(),
            events: vec![],
        }
    }

    pub(crate) fn with_resource_write_set(
        mut self,
        resource_write_set: impl IntoIterator<Item = (StateKey, (WriteOp, Option<Arc<MoveTypeLayout>>))>,
    ) -> Self {
        assert!(self.resource_write_set.is_empty());
        self.resource_write_set.extend(resource_write_set);
        self
    }

    pub(crate) fn with_resource_group_write_set(
        mut self,
        resource_group_write_set: impl IntoIterator<Item = (StateKey, GroupWrite)>,
    ) -> Self {
        assert!(self.resource_group_write_set.is_empty());
        self.resource_group_write_set.extend(resource_group_write_set);
        self
    }

    pub(crate) fn with_module_write_set(
        mut self,
        module_write_set: impl IntoIterator<Item = (StateKey, WriteOp)>,
    ) -> Self {
        assert!(self.module_write_set.is_empty());
        self.module_write_set.extend(module_write_set);
        self
    }

    pub(crate) fn with_aggregator_v1_write_set(
        mut self,
        aggregator_v1_write_set: impl IntoIterator<Item = (StateKey, WriteOp)>,
    ) -> Self {
        assert!(self.aggregator_v1_write_set.is_empty());
        self.aggregator_v1_write_set.extend(aggregator_v1_write_set);
        self
    }

    pub(crate) fn with_aggregator_v1_delta_set(
        mut self,
        aggregator_v1_delta_set: impl IntoIterator<Item = (StateKey, DeltaOp)>,
    ) -> Self {
        assert!(self.aggregator_v1_delta_set.is_empty());
        self.aggregator_v1_delta_set.extend(aggregator_v1_delta_set);
        self
    }

    pub(crate) fn with_delayed_field_change_set(
        mut self,
        delayed_field_change_set: impl IntoIterator<Item = (DelayedFieldID, DelayedChange<DelayedFieldID>)>,
    ) -> Self {
        assert!(self.delayed_field_change_set.is_empty());
        self.delayed_field_change_set.extend(delayed_field_change_set);
        self
    }

    pub(crate) fn with_reads_needing_delayed_field_exchange(
        mut self,
        reads_needing_delayed_field_exchange: impl IntoIterator<Item = (StateKey, (WriteOp, Arc<MoveTypeLayout>))>,
    ) -> Self {
        assert!(self.reads_needing_delayed_field_exchange.is_empty());
        self.reads_needing_delayed_field_exchange
            .extend(reads_needing_delayed_field_exchange);
        self
    }

    pub(crate) fn with_group_reads_needing_delayed_field_exchange(
        mut self,
        group_reads_needing_delayed_field_exchange: impl IntoIterator<Item = (StateKey, (WriteOp, u64))>,
    ) -> Self {
        assert!(self.group_reads_needing_delayed_field_exchange.is_empty());
        self.group_reads_needing_delayed_field_exchange
            .extend(group_reads_needing_delayed_field_exchange);
        self
    }

    pub(crate) fn with_events(
        mut self,
        events: impl IntoIterator<Item = (ContractEvent, Option<MoveTypeLayout>)>,
    ) -> Self {
        assert!(self.events.is_empty());
        self.events.extend(events);
        self
    }

    pub(crate) fn build(self) -> ExpandedVMChangeSet {
        ExpandedVMChangeSet::new_expanded(
            self.resource_write_set,
            self.resource_group_write_set,
            self.module_write_set,
            self.aggregator_v1_write_set,
            self.aggregator_v1_delta_set,
            self.delayed_field_change_set,
            self.reads_needing_delayed_field_exchange,
            self.group_reads_needing_delayed_field_exchange,
            self.events,
            &MockChangeSetChecker,
        )
        .unwrap()
    }
}
