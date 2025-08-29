// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_config::{
        CoinStoreResource, ConcurrentSupplyResource, FungibleStoreResource, ObjectGroupResource,
    },
    contract_event::ContractEvent,
    fee_statement::FeeStatement,
    state_store::state_key::StateKey,
    transaction::{diff_filter::DiffFilter, ExecutionStatus, TransactionOutput},
    write_set::{TransactionWrite, WriteOp, WriteSet, TOTAL_SUPPLY_STATE_KEY},
    AptosCoinType,
};
use move_core_types::{
    account_address::AccountAddress,
    language_storage::{StructTag, TypeTag},
    move_resource::MoveStructType,
};
use once_cell::sync::Lazy;
use std::{
    collections::{BTreeMap, HashMap},
    str::FromStr,
};

static FA_SUPPLY_TAG: Lazy<StructTag> = Lazy::new(ConcurrentSupplyResource::struct_tag);
static FA_STORE_TAG: Lazy<StructTag> = Lazy::new(FungibleStoreResource::struct_tag);
static COIN_STORE_TAG: Lazy<StructTag> = Lazy::new(CoinStoreResource::<AptosCoinType>::struct_tag);
static FEE_STATEMENT_TAG: Lazy<TypeTag> =
    Lazy::new(|| TypeTag::Struct(Box::new(FeeStatement::struct_tag())));
static OBJECT_GROUP_TAG: Lazy<StructTag> = Lazy::new(ObjectGroupResource::struct_tag);

static FA_SUPPLY_GROUP_STATE_KEY: Lazy<StateKey> = Lazy::new(|| {
    let fa_addr = AccountAddress::from_str("0xa").unwrap();
    StateKey::resource_group(&fa_addr, &OBJECT_GROUP_TAG)
});

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct GasUsedDiff {
    pub before: u64,
    pub after: u64,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct ExecutionStatusDiff {
    pub before: ExecutionStatus,
    pub after: ExecutionStatus,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct EventDiff {
    pub before: Option<ContractEvent>,
    pub after: Option<ContractEvent>,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct WriteOpDiff {
    pub before: Option<WriteOp>,
    pub after: Option<WriteOp>,
}

/// Holds all differences for a pair of transaction outputs.
#[derive(Debug)]
pub struct TransactionDiff {
    gas_used_diff: Option<GasUsedDiff>,
    execution_status_diff: Option<ExecutionStatusDiff>,
    event_diffs: HashMap<TypeTag, EventDiff>,
    write_op_diffs: HashMap<StateKey, WriteOpDiff>,

    fee_payer: Option<AccountAddress>,
}

impl TransactionDiff {
    /// Given a pair of transaction outputs, computes its [TransactionDiff] that includes the gas
    /// used, execution status, events and write sets.
    pub fn build_from_outputs(
        before: TransactionOutput,
        after: TransactionOutput,
        fee_payer: Option<AccountAddress>,
    ) -> TransactionDiff {
        let (write_set_before, events_before, gas_used_before, transaction_status_before, _) =
            before.unpack();
        let (write_set_after, events_after, gas_used_after, transaction_status_after, _) =
            after.unpack();

        let gas_used_diff = (gas_used_before != gas_used_after).then_some(GasUsedDiff {
            before: gas_used_before,
            after: gas_used_after,
        });

        // All statuses must be kept, since we are replaying transactions.
        let execution_status_before = transaction_status_before.as_kept_status().unwrap();
        let execution_status_after = transaction_status_after.as_kept_status().unwrap();
        let execution_status_diff =
            (execution_status_before != execution_status_after).then_some(ExecutionStatusDiff {
                before: execution_status_before,
                after: execution_status_after,
            });

        let event_diffs = Self::diff_events(events_before, events_after);
        let write_op_diffs = Self::diff_write_sets(write_set_before, write_set_after);

        TransactionDiff {
            gas_used_diff,
            execution_status_diff,
            event_diffs,
            write_op_diffs,
            fee_payer,
        }
    }

    /// Returns true if the current diff is empty.
    pub fn is_empty(&self) -> bool {
        self.gas_used_diff.is_none()
            && self.execution_status_diff.is_none()
            && self.event_diffs.is_empty()
            && self.write_op_diffs.is_empty()
    }

    /// Returns the number of differences in this diff.
    pub fn num_differences(&self) -> usize {
        let gas_count = if self.gas_used_diff.is_some() { 1 } else { 0 };
        let status_count = if self.execution_status_diff.is_some() {
            1
        } else {
            0
        };
        gas_count + status_count + self.event_diffs.len() + self.write_op_diffs.len()
    }

    /// Returns difference in gas used, if any.
    pub fn gas_used_diff(&self) -> Option<GasUsedDiff> {
        self.gas_used_diff.clone()
    }

    /// Returns difference in execution statuses, if any.
    pub fn execution_status_diff(&self) -> Option<ExecutionStatusDiff> {
        self.execution_status_diff.clone()
    }

    /// Returns all events that are different.
    pub fn event_diffs(&self) -> impl Iterator<Item = (&TypeTag, &EventDiff)> {
        self.event_diffs.iter()
    }

    /// Returns all writes which are different.
    pub fn write_op_diffs(&self) -> impl Iterator<Item = (&StateKey, &WriteOpDiff)> {
        self.write_op_diffs.iter()
    }

    /// Apply filters to this diff, consuming it and returning a filtered minimized diff that
    /// excludes acceptable differences. The returned diff contains only the differences that were
    /// not filtered out.
    pub fn evaluate(mut self, filter: &DiffFilter) -> Self {
        match filter {
            DiffFilter::HardStatusChange { from, to } => {
                if let Some(ExecutionStatusDiff { before, after }) = self.execution_status_diff() {
                    if before.eq(from) && after.eq(to) {
                        self.execution_status_diff = None;
                        self.gas_used_diff = None;
                        self.event_diffs.clear();
                        self.write_op_diffs.clear();
                    }
                }
            },
            DiffFilter::SoftStatusChange { from, to } => {
                if let Some(ExecutionStatusDiff { before, after }) = self.execution_status_diff() {
                    if before.eq(from) && after.eq(to) {
                        self.execution_status_diff = None;
                    }
                }
            },
            DiffFilter::GasChange {
                min_delta,
                max_delta,
            } => {
                if let Some(GasUsedDiff { before, after }) = self.gas_used_diff() {
                    let gas_delta = after as i64 - before as i64;
                    let ignore_gas_used = match (min_delta, max_delta) {
                        (None, None) => true,
                        (None, Some(max)) => gas_delta <= *max,
                        (Some(min), None) => gas_delta >= *min,
                        (Some(min), Some(max)) => gas_delta >= *min && gas_delta <= *max,
                    };
                    if ignore_gas_used {
                        self.gas_used_diff = None;
                    }
                }

                // Remove fee statement.
                self.event_diffs.remove(&FEE_STATEMENT_TAG);

                // Legacy supply for coin standard.
                self.write_op_diffs.remove(&*TOTAL_SUPPLY_STATE_KEY);

                // FA-based supply.
                let fa_group_key = &*FA_SUPPLY_GROUP_STATE_KEY;
                let remove_group_diff =
                    if let Some(group_diff) = self.write_op_diffs.get_mut(fa_group_key) {
                        patch_object_group_write_op(&mut group_diff.before, |group| {
                            group.group.remove(&FA_SUPPLY_TAG);
                        });
                        patch_object_group_write_op(&mut group_diff.after, |group| {
                            group.group.remove(&FA_SUPPLY_TAG);
                        });
                        group_diff.before == group_diff.after
                    } else {
                        false
                    };
                if remove_group_diff {
                    self.write_op_diffs.remove(fa_group_key);
                }

                // Filter out fee payer's coin balance or FA balance.
                let fee_payer = self.fee_payer;
                if let Some(fee_payer) = fee_payer {
                    let coin_store_key = StateKey::resource(&fee_payer, &COIN_STORE_TAG)
                        .expect("Creating CoinStore key always succeeds");
                    self.write_op_diffs.remove(&coin_store_key);

                    let fee_payer_group_key =
                        StateKey::resource_group(&fee_payer, &OBJECT_GROUP_TAG);
                    let remove_group_diff = if let Some(group_diff) =
                        self.write_op_diffs.get_mut(&fee_payer_group_key)
                    {
                        patch_object_group_write_op(&mut group_diff.before, |group| {
                            group.group.remove(&FA_STORE_TAG);
                        });
                        patch_object_group_write_op(&mut group_diff.after, |group| {
                            group.group.remove(&FA_STORE_TAG);
                        });
                        group_diff.before == group_diff.after
                    } else {
                        false
                    };
                    if remove_group_diff {
                        self.write_op_diffs.remove(&fee_payer_group_key);
                    }
                }
            },
        }

        self
    }
}

// Private interfaces.
impl TransactionDiff {
    /// Computes the differences between a pair of event vectors.
    fn diff_events(
        before: Vec<ContractEvent>,
        after: Vec<ContractEvent>,
    ) -> HashMap<TypeTag, EventDiff> {
        let event_vec_to_map = |events: Vec<ContractEvent>| {
            events
                .into_iter()
                .map(|event| (event.type_tag().clone(), event))
                .collect::<BTreeMap<_, _>>()
        };

        let before = event_vec_to_map(before);
        let mut after = event_vec_to_map(after);

        let mut diffs = HashMap::new();
        for (ty_tag, left_event) in before {
            let maybe_event_after = after.remove(&ty_tag);
            if maybe_event_after
                .as_ref()
                .is_some_and(|right_event| left_event.event_data() == right_event.event_data())
            {
                continue;
            }

            diffs.insert(ty_tag, EventDiff {
                before: Some(left_event),
                after: maybe_event_after,
            });
        }

        for (ty_tag, right_event) in after.into_iter() {
            diffs.insert(ty_tag, EventDiff {
                before: None,
                after: Some(right_event),
            });
        }
        diffs
    }

    /// Computes the differences between a pair of write sets.
    fn diff_write_sets(before: WriteSet, after: WriteSet) -> HashMap<StateKey, WriteOpDiff> {
        let before = before.into_mut().into_inner();
        let mut after = after.into_mut().into_inner();

        let mut diffs = HashMap::new();
        for (state_key, write_op_before) in before {
            let maybe_right_write_op_after = after.remove(&state_key);
            if maybe_right_write_op_after
                .as_ref()
                .is_some_and(|right_write_op| right_write_op == &write_op_before)
            {
                // Both write ops exist and are the same.
                continue;
            }

            diffs.insert(state_key, WriteOpDiff {
                before: Some(write_op_before),
                after: maybe_right_write_op_after,
            });
        }

        for (state_key, write_op_after) in after {
            diffs.insert(state_key, WriteOpDiff {
                before: None,
                after: Some(write_op_after),
            });
        }
        diffs
    }
}

fn patch_object_group_write_op<F>(write_op: &mut Option<WriteOp>, action: F)
where
    F: FnOnce(&mut ObjectGroupResource),
{
    if let Some(write_op) = write_op {
        let patched_bytes = match write_op.bytes() {
            Some(bytes) => {
                let mut group = bcs::from_bytes::<ObjectGroupResource>(bytes).unwrap();
                action(&mut group);
                Some(bcs::to_bytes(&group).unwrap())
            },
            None => None,
        };
        if let Some(bytes) = patched_bytes {
            write_op.set_bytes(bytes.into());
        }
    }
}
