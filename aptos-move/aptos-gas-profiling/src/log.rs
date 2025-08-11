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
#[derive(Debug, Clone)]
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
}

impl TransactionGasLog {
    pub fn entry_point(&self) -> &FrameName {
        &self.exec_io.call_graph.name
    }
}
