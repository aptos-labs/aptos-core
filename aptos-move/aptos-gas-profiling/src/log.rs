// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::render::Render;
use aptos_gas_algebra::{AbstractValueSize, Fee, GasScalingFactor, InternalGas, NumBytes};
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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum FrameName {
    Script,
    Function {
        module_id: ModuleId,
        name: Identifier,
        ty_args: Vec<TypeTag>,
    },
    TransactionBatch,
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
    /// Execution gas - corresponds to FeeStatement::execution_gas_units.
    pub execution_gas: InternalGas,
    /// IO gas - corresponds to FeeStatement::io_gas_units.
    pub io_gas: InternalGas,

    pub intrinsic_cost: InternalGas,
    pub keyless_cost: InternalGas,
    pub slh_dsa_sha2_128s_cost: InternalGas,
    pub encrypted_txn_decryption_cost: InternalGas,
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
    pub num_txns: usize,
    pub peak_memory_usage: AbstractValueSize,
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

    pub fn new_transaction_batch() -> Self {
        Self {
            name: FrameName::TransactionBatch,
            events: vec![],
            native_gas: 0.into(),
        }
    }
}

/// Describes a mismatch between the gas meter's totals and the sum of the
/// itemized costs the profiler recorded.
///
/// Reaching this state always indicates a bug in the gas profiler itself --
/// the underlying transaction execution and gas accounting are unaffected.
#[derive(Debug, Clone)]
pub struct ConsistencyError {
    /// Which subtotal failed to add up.
    pub kind: ConsistencyErrorKind,
    /// The total reported by the gas meter (source of truth).
    pub from_gas_meter: u128,
    /// The total the profiler computed by summing the recorded items.
    pub calculated: u128,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsistencyErrorKind {
    ExecutionAndIo,
    StorageFee,
    StorageRefund,
}

impl ConsistencyErrorKind {
    fn label(self) -> &'static str {
        match self {
            Self::ExecutionAndIo => "Execution & IO costs",
            Self::StorageFee => "Storage fees",
            Self::StorageRefund => "Storage refunds",
        }
    }
}

impl std::fmt::Display for ConsistencyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Gas profiler consistency check failed: {} do not add up. \
             This is an unexpected condition that indicates a bug in the gas profiler \
             (not in the transaction being profiled or in the gas meter). \
             The generated gas report is likely incomplete or inaccurate. \
             Please report this at https://github.com/aptos-labs/aptos-core/issues, \
             including the transaction ID or payload along with the details below. \
             From gas meter: {}. Calculated: {}.",
            self.kind.label(),
            self.from_gas_meter,
            self.calculated,
        )
    }
}

impl std::error::Error for ConsistencyError {}

impl StorageFees {
    /// Returns `Ok(())` if the recorded storage fees and refunds sum to the
    /// totals reported by the gas meter, otherwise returns the first detected
    /// inconsistency.
    pub fn check_consistency(&self) -> Result<(), ConsistencyError> {
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
            return Err(ConsistencyError {
                kind: ConsistencyErrorKind::StorageFee,
                from_gas_meter: u64::from(self.total).into(),
                calculated: u64::from(total).into(),
            });
        }
        if total_refund != self.total_refund {
            return Err(ConsistencyError {
                kind: ConsistencyErrorKind::StorageRefund,
                from_gas_meter: u64::from(self.total_refund).into(),
                calculated: u64::from(total_refund).into(),
            });
        }
        Ok(())
    }

    /// Panics if [`Self::check_consistency`] fails. Used by callers that
    /// expect strict behavior (tests, internal tools).
    pub(crate) fn assert_consistency(&self) {
        if let Err(err) = self.check_consistency() {
            panic!("{}", err);
        }
    }
}

impl ExecutionAndIOCosts {
    /// Returns the total execution + IO gas.
    pub fn total(&self) -> InternalGas {
        self.execution_gas + self.io_gas
    }

    #[allow(clippy::needless_lifetimes)]
    pub fn gas_events<'a>(&'a self) -> GasEventIter<'a> {
        GasEventIter {
            stack: smallvec![(&self.call_graph, 0)],
        }
    }

    /// Returns `Ok(())` if the itemized execution and IO costs recorded by the
    /// profiler sum to the totals reported by the gas meter, otherwise returns
    /// a structured error describing the discrepancy.
    pub fn check_consistency(&self) -> Result<(), ConsistencyError> {
        use ExecutionGasEvent::{Bytecode, Call, CallNative, CreateTy, LoadResource, Loc};

        let mut total = InternalGas::zero();

        total += self.intrinsic_cost;
        total += self.keyless_cost;
        total += self.slh_dsa_sha2_128s_cost;
        total += self.encrypted_txn_decryption_cost;

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

        if total != self.total() {
            return Err(ConsistencyError {
                kind: ConsistencyErrorKind::ExecutionAndIo,
                from_gas_meter: u64::from(self.total()).into(),
                calculated: u64::from(total).into(),
            });
        }
        Ok(())
    }

    /// Panics if [`Self::check_consistency`] fails. Used by callers that
    /// expect strict behavior (tests, internal tools).
    pub(crate) fn assert_consistency(&self) {
        if let Err(err) = self.check_consistency() {
            panic!("{}", err);
        }
    }
}

impl TransactionGasLog {
    pub fn entry_point(&self) -> &FrameName {
        &self.exec_io.call_graph.name
    }
}
