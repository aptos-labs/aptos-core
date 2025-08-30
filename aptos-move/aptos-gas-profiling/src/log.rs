// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::render::Render;
use aptos_gas_algebra::{Fee, GasScalingFactor, InternalGas, NumBytes};
use aptos_types::state_store::state_key::StateKey;
use move_binary_format::{file_format::CodeOffset, file_format_common::Opcodes};
use move_core_types::{
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
};
use move_vm_types::gas::DependencyKind;
use smallvec::{smallvec, SmallVec};
use std::collections::{btree_map::Entry, BTreeMap};

/// An event occurred during the execution of a function, along with the
/// gas cost associated with it, if any.
#[derive(Debug, Clone)]
pub enum ExecutionGasEvent {
    /// A special event indicating that the program counter has moved to
    /// a specific offset. This is emitted by the branch instructions
    /// and is crucial for reconstructing the control flow.
    Loc(CodeOffset),
    Bytecode {
        op: Opcodes,
        cost: InternalGas,
    },
    Call(CallFrame),
    CallNative {
        module_id: ModuleId,
        fn_name: Identifier,
        ty_args: Vec<TypeTag>,
        cost: InternalGas,
    },
    LoadResource {
        addr: AccountAddress,
        ty: TypeTag,
        cost: InternalGas,
    },
    CreateTy {
        cost: InternalGas,
    },
}

/// An enum representing the name of a call frame.
/// Could be either a script or a function.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum FrameName {
    Script,
    Function {
        module_id: ModuleId,
        name: Identifier,
        ty_args: Vec<TypeTag>,
    },
}

/// A struct containing information about a function call, including the name of the
/// function and all gas events that happened during the call.
#[derive(Debug, Clone)]
pub struct CallFrame {
    pub name: FrameName,
    pub events: Vec<ExecutionGasEvent>,
    /// Accumulates gas charged by native functions. For frames of non-native functions, kept as 0.
    pub native_gas: InternalGas,
}

/// The type of an operation performed on a storage item.
///
/// Possible values: Creation, Modification & Deletion.
#[derive(Debug, Clone)]
pub enum WriteOpType {
    Creation,
    Modification,
    Deletion,
}

/// Struct representing the transient (IO) cost of a write operation.
#[derive(Debug, Clone)]
pub struct WriteTransient {
    pub key: StateKey,
    pub op_type: WriteOpType,
    pub cost: InternalGas,
}

/// Struct representing the storage cost of a write operation.
#[derive(Debug, Clone)]
pub struct WriteStorage {
    pub key: StateKey,
    pub op_type: WriteOpType,
    pub cost: Fee,
    pub refund: Fee,
}

/// Struct representing the transient (IO) cost of an event.
#[derive(Debug, Clone)]
pub struct EventTransient {
    pub ty: TypeTag,
    pub cost: InternalGas,
}

#[derive(Debug, Clone)]
/// Struct representing the storage cost of an event.
pub struct EventStorage {
    pub ty: TypeTag,
    pub cost: Fee,
}

#[derive(Debug, Clone)]
/// Struct representing the cost of a dependency.
pub struct Dependency {
    pub kind: DependencyKind,
    pub id: ModuleId,
    pub size: NumBytes,
    pub cost: InternalGas,
}

impl Dependency {
    pub(crate) fn render(&self) -> String {
        let kind = match self.kind {
            DependencyKind::New => " (new)",
            DependencyKind::Existing => "",
        };
        format!("{}{}", Render(&self.id), kind,)
    }
}

/// Struct containing all execution and io costs.
#[derive(Debug, Clone)]
pub struct ExecutionAndIOCosts {
    pub gas_scaling_factor: GasScalingFactor,
    pub total: InternalGas,

    pub intrinsic_cost: InternalGas,
    pub keyless_cost: InternalGas,
    pub dependencies: Vec<Dependency>,
    pub call_graph: CallFrame,
    pub transaction_transient: Option<InternalGas>,
    pub events_transient: Vec<EventTransient>,
    pub write_set_transient: Vec<WriteTransient>,
}

#[derive(Debug, Clone)]
/// Struct containing all types of storage fees.
pub struct StorageFees {
    pub total: Fee,
    pub total_refund: Fee,

    pub write_set_storage: Vec<WriteStorage>,
    pub events: Vec<EventStorage>,
    pub event_discount: Fee,
    pub txn_storage: Fee,
}

/// A complete log that contains all gas-related information about a transaction, including
/// the intrinsic cost, a detailed execution log and the write set costs.
#[derive(Debug, Clone)]
pub struct TransactionGasLog {
    pub exec_io: ExecutionAndIOCosts,
    pub storage: StorageFees,
}

pub struct GasEventIter<'a> {
    stack: SmallVec<[(&'a CallFrame, usize); 16]>,
}

impl<'a> Iterator for GasEventIter<'a> {
    type Item = &'a ExecutionGasEvent;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.stack.last_mut() {
                None => return None,
                Some((frame, pc)) => {
                    if *pc >= frame.events.len() {
                        self.stack.pop();
                        continue;
                    }

                    let event = &frame.events[*pc];
                    *pc += 1;
                    if let ExecutionGasEvent::Call(child_frame) = event {
                        self.stack.push((child_frame, 0))
                    }
                    return Some(event);
                },
            }
        }
    }
}

impl CallFrame {
    pub fn new_function(module_id: ModuleId, name: Identifier, ty_args: Vec<TypeTag>) -> Self {
        Self {
            name: FrameName::Function {
                module_id,
                name,
                ty_args,
            },
            events: vec![],
            native_gas: 0.into(),
        }
    }

    pub fn new_script() -> Self {
        Self {
            name: FrameName::Script,
            events: vec![],
            native_gas: 0.into(),
        }
    }
}

impl StorageFees {
    pub(crate) fn assert_consistency(&self) {
        let mut total = Fee::zero();
        let mut total_refund = Fee::zero();

        for write in &self.write_set_storage {
            total += write.cost;
            total_refund += write.refund;
        }

        for event in &self.events {
            total += event.cost;
        }

        total += self.txn_storage;

        if total != self.total {
            panic!(
                "Storage fees do not add up. Check if the gas meter & the gas profiler have been implemented correctly. From gas meter: {}. Calculated: {}.",
                self.total, total
            );
        }
        if total_refund != self.total_refund {
            panic!(
                "Storage refunds do not add up. Check if the gas meter & the gas profiler have been implemented correctly. From gas meter: {}. Calculated: {}.",
                self.total_refund, total
            );
        }
    }

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

impl ExecutionAndIOCosts {
    #[allow(clippy::needless_lifetimes)]
    pub fn gas_events<'a>(&'a self) -> GasEventIter<'a> {
        GasEventIter {
            stack: smallvec![(&self.call_graph, 0)],
        }
    }

    pub(crate) fn assert_consistency(&self) {
        use ExecutionGasEvent::{Bytecode, Call, CallNative, CreateTy, LoadResource, Loc};

        let mut total = InternalGas::zero();

        total += self.intrinsic_cost;
        total += self.keyless_cost;

        for dep in &self.dependencies {
            total += dep.cost;
        }

        for op in self.gas_events() {
            match op {
                Loc(..) | Call(..) => (),
                Bytecode { cost, .. }
                | CallNative { cost, .. }
                | LoadResource { cost, .. }
                | CreateTy { cost, .. } => total += *cost,
            }
        }

        if let Some(cost) = self.transaction_transient {
            total += cost;
        }

        for event in &self.events_transient {
            total += event.cost;
        }

        for write in &self.write_set_transient {
            total += write.cost;
        }

        if total != self.total {
            panic!(
                "Execution & IO costs do not add up. Check if the gas meter & the gas profiler have been implemented correctly. From gas meter: {}. Calculated: {}.",
                self.total, total
            )
        }
    }

    fn combine(&self, other: &ExecutionAndIOCosts) -> ExecutionAndIOCosts {
        assert_eq!(self.gas_scaling_factor, other.gas_scaling_factor);

        // nest all entry functions under script:
        let mut call_graph = CallFrame::new_script();
        for cur in [&self.call_graph, &other.call_graph] {
            if let FrameName::Script = self.call_graph.name {
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
            events_transient: [&self.events_transient[..], &other.events_transient[..]].concat(),
            write_set_transient: [
                &self.write_set_transient[..],
                &other.write_set_transient[..],
            ]
            .concat(),
        }
    }
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

struct FoldedCallFrame {
    self_gas: InternalGas,
    instructions: BTreeMap<InstructionKey, InternalGas>,
    children: BTreeMap<FrameName, FoldedCallFrame>,
}

impl FoldedCallFrame {
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
                            vacant.insert(frame.into_folded_call_frame().1);
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

    fn unfold(self, name: FrameName) -> CallFrame {
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
            events.push(ExecutionGasEvent::Call(frame.unfold(frame_name)));
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

impl CallFrame {
    fn into_folded_call_frame(self) -> (FrameName, FoldedCallFrame) {
        let mut result = FoldedCallFrame::empty();
        let name = self.name.clone();
        result.combine(self);
        (name, result)
    }

    pub fn fold(self) -> CallFrame {
        let (name, folded) = self.into_folded_call_frame();
        folded.unfold(name)
    }
}

impl TransactionGasLog {
    pub fn entry_point(&self) -> &FrameName {
        &self.exec_io.call_graph.name
    }

    pub fn combine(&self, other: &TransactionGasLog) -> TransactionGasLog {
        TransactionGasLog {
            exec_io: self.exec_io.combine(&other.exec_io),
            storage: self.storage.combine(&other.storage),
        }
    }
}

fn vec_concat<T>(mut left: Vec<T>, mut right: Vec<T>) -> Vec<T> {
    left.append(&mut right);
    left
}
