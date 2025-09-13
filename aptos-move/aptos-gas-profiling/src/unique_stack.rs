// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    log::{CallFrame, Dependency, ExecutionAndIOCosts, ExecutionGasEvent, StorageFees},
    FrameName, TransactionGasLog,
};
use aptos_gas_algebra::InternalGas;
use move_binary_format::file_format_common::Opcodes;
use move_core_types::{
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
};
use std::collections::{btree_map::Entry, BTreeMap};

impl CallFrame {
    fn into_unique_stack_folded_call_frame(self) -> (FrameName, UniqueStackFoldedCallFrame) {
        let mut result = UniqueStackFoldedCallFrame::empty();
        let name = self.name.clone();
        result.combine(self);
        (name, result)
    }

    pub fn fold_unique_stack(self) -> CallFrame {
        let (name, folded) = self.into_unique_stack_folded_call_frame();
        folded.into_call_frame(name)
    }
}

impl StorageFees {
    fn combine(&self, other: &StorageFees) -> StorageFees {
        StorageFees {
            total: self.total + other.total,
            total_refund: self.total_refund + other.total_refund,
            write_set_storage: vec_concat(
                self.write_set_storage.clone(),
                other.write_set_storage.clone(),
            ),
            events: vec_concat(self.events.clone(), other.events.clone()),
            event_discount: self.event_discount + other.event_discount,
            txn_storage: self.txn_storage + other.txn_storage,
        }
    }
}

impl TransactionGasLog {
    pub fn combine(&self, other: &TransactionGasLog) -> TransactionGasLog {
        TransactionGasLog {
            exec_io: self.exec_io.combine(&other.exec_io),
            storage: self.storage.combine(&other.storage),
            multi_txn: true,
        }
    }
}

impl ExecutionAndIOCosts {
    fn combine(&self, other: &ExecutionAndIOCosts) -> ExecutionAndIOCosts {
        assert_eq!(self.gas_scaling_factor, other.gas_scaling_factor);

        // nest all entry functions under script:
        let mut call_graph = CallFrame::new_transaction_batch();
        for cur in [&self.call_graph, &other.call_graph] {
            if let FrameName::TransactionBatch = self.call_graph.name {
                call_graph.native_gas += cur.native_gas;
                call_graph.events.append(&mut cur.events.clone());
            } else {
                call_graph.events.push(ExecutionGasEvent::Call(cur.clone()));
            }
        }
        let mut dependencies = BTreeMap::new();
        for cur in self.dependencies.iter().chain(other.dependencies.iter()) {
            dependencies
                .entry((cur.kind, cur.id.clone(), cur.size))
                .and_modify(|v| *v += cur.cost)
                .or_insert(cur.cost);
        }

        ExecutionAndIOCosts {
            gas_scaling_factor: self.gas_scaling_factor,
            total: self.total + other.total,
            intrinsic_cost: self.intrinsic_cost + other.intrinsic_cost,
            keyless_cost: self.keyless_cost + other.keyless_cost,
            dependencies: dependencies
                .into_iter()
                .map(|((kind, id, size), cost)| Dependency {
                    kind,
                    id,
                    size,
                    cost,
                })
                .collect(),
            call_graph,
            transaction_transient: self
                .transaction_transient
                .map(|val_a| {
                    other
                        .transaction_transient
                        .map_or(val_a, |val_b| val_a + val_b)
                })
                .or(other.transaction_transient),
            events_transient: vec_concat(
                self.events_transient.clone(),
                other.events_transient.clone(),
            ),
            write_set_transient: vec_concat(
                self.write_set_transient.clone(),
                other.write_set_transient.clone(),
            ),
        }
    }
}

fn vec_concat<T>(mut left: Vec<T>, mut right: Vec<T>) -> Vec<T> {
    left.append(&mut right);
    left
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
enum InstructionKey {
    Bytecode {
        op: Opcodes,
    },
    CallNative {
        module_id: ModuleId,
        fn_name: Identifier,
        ty_args: Vec<TypeTag>,
    },
    LoadResource {
        addr: AccountAddress,
        ty: TypeTag,
    },
    CreateTy,
}

struct UniqueStackFoldedCallFrame {
    self_gas: InternalGas,
    instructions: BTreeMap<InstructionKey, InternalGas>,
    children: BTreeMap<FrameName, UniqueStackFoldedCallFrame>,
}

impl UniqueStackFoldedCallFrame {
    fn empty() -> Self {
        Self {
            self_gas: 0.into(),
            instructions: BTreeMap::new(),
            children: BTreeMap::new(),
        }
    }

    fn combine(&mut self, other: CallFrame) {
        self.self_gas += other.native_gas;

        for child in other.events {
            let (instruction_key, cost) = match child {
                ExecutionGasEvent::Loc(_) => continue,
                ExecutionGasEvent::Bytecode { op, cost } => (InstructionKey::Bytecode { op }, cost),
                ExecutionGasEvent::LoadResource { addr, ty, cost } => {
                    (InstructionKey::LoadResource { addr, ty }, cost)
                },
                ExecutionGasEvent::CallNative {
                    module_id,
                    fn_name,
                    ty_args,
                    cost,
                } => (
                    InstructionKey::CallNative {
                        module_id,
                        fn_name,
                        ty_args,
                    },
                    cost,
                ),
                ExecutionGasEvent::CreateTy { cost } => (InstructionKey::CreateTy, cost),
                ExecutionGasEvent::Call(frame) => {
                    match self.children.entry(frame.name.clone()) {
                        Entry::Vacant(vacant) => {
                            vacant.insert(frame.into_unique_stack_folded_call_frame().1);
                        },
                        Entry::Occupied(mut occupied) => {
                            occupied.get_mut().combine(frame);
                        },
                    };
                    continue;
                },
            };
            self.instructions
                .entry(instruction_key)
                .and_modify(|val| *val += cost)
                .or_insert(cost);
        }
    }

    fn into_call_frame(self, name: FrameName) -> CallFrame {
        let mut events = vec![];
        for (instruction_key, cost) in self.instructions {
            events.push(match instruction_key {
                InstructionKey::Bytecode { op } => ExecutionGasEvent::Bytecode { op, cost },
                InstructionKey::CallNative {
                    module_id,
                    fn_name,
                    ty_args,
                } => ExecutionGasEvent::CallNative {
                    module_id,
                    fn_name,
                    ty_args,
                    cost,
                },
                InstructionKey::LoadResource { addr, ty } => {
                    ExecutionGasEvent::LoadResource { addr, ty, cost }
                },
                InstructionKey::CreateTy => ExecutionGasEvent::CreateTy { cost },
            });
        }
        for (frame_name, frame) in self.children {
            events.push(ExecutionGasEvent::Call(frame.into_call_frame(frame_name)));
        }
        events.sort_by_key(|v| match v {
            ExecutionGasEvent::Loc(_) => 0.into(),
            ExecutionGasEvent::Bytecode { cost, .. } => *cost,
            ExecutionGasEvent::Call(call_frame) => call_frame.native_gas,
            ExecutionGasEvent::CallNative { cost, .. } => *cost,
            ExecutionGasEvent::LoadResource { cost, .. } => *cost,
            ExecutionGasEvent::CreateTy { cost } => *cost,
        });
        events.reverse();
        CallFrame {
            name,
            events,
            native_gas: self.self_gas,
        }
    }
}
