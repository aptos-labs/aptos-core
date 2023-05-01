// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_gas::{Fee, GasScalingFactor};
use aptos_types::state_store::state_key::StateKey;
use move_binary_format::{file_format::CodeOffset, file_format_common::Opcodes};
use move_core_types::{
    account_address::AccountAddress,
    gas_algebra::InternalGas,
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
};

/// An event occurred during the execution of a function, along with the
/// gas cost associated with it, if any.
#[derive(Debug)]
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
}

/// An enum representing the name of a call frame.
/// Could be either a script or a function.
#[derive(Debug)]
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
#[derive(Debug)]
pub struct CallFrame {
    pub name: FrameName,
    pub events: Vec<ExecutionGasEvent>,
}

/// The type of an operation performed on a storage item.
///
/// Possible values: Creation, Modification & Deletion.
#[derive(Debug)]
pub enum WriteOpType {
    Creation,
    Modification,
    Deletion,
}

/// Struct representing the transient (IO) cost of a write operation.
#[derive(Debug)]
pub struct WriteTransient {
    pub key: StateKey,
    pub op_type: WriteOpType,
    pub cost: InternalGas,
}

/// Struct representing the storage cost of a write operation.
#[derive(Debug)]
pub struct WriteStorage {
    pub key: StateKey,
    pub op_type: WriteOpType,
    pub cost: Fee,
}

#[derive(Debug)]
/// Struct representing the storage cost of an event.
pub struct EventStorage {
    pub ty: TypeTag,
    pub cost: Fee,
}

#[derive(Debug)]
// Struct containing all types of storage fees.
pub struct StorageFees {
    pub write_set_storage: Vec<WriteStorage>,
    pub events: Vec<EventStorage>,
    pub event_discount: Fee,
    pub txn_storage: Fee,
}

/// A complete log that contains all gas-related information about a transaction, including
/// the intrinsic cost, a detailed execution log and the write set costs.
#[derive(Debug)]
pub struct TransactionGasLog {
    // TODO(Gas): move the execution-related entries into a separate struct.
    pub gas_scaling_factor: GasScalingFactor,
    pub intrinsic_cost: InternalGas,
    pub call_graph: CallFrame,
    pub write_set_transient: Vec<WriteTransient>,

    pub storage: StorageFees,
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
        }
    }

    pub fn new_script() -> Self {
        Self {
            name: FrameName::Script,
            events: vec![],
        }
    }
}

impl TransactionGasLog {
    pub fn entry_point(&self) -> &FrameName {
        &self.call_graph.name
    }
}
