// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Opt-in function-usage collection for off-chain transaction replay.

use aptos_crypto::HashValue;
use aptos_types::transaction::TransactionStatus;
use move_core_types::{account_address::AccountAddress, language_storage::ModuleId};
use move_vm_runtime::{
    execution_tracing::{FunctionCallKind, Trace, TraceRecorder},
    LoadedFunction,
};
use move_vm_types::instr::Instruction;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

/// Stable function identity used by framework-usage reports.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct FunctionId {
    pub module_id: Option<ModuleId>,
    pub function_name: String,
}

impl FunctionId {
    fn from_loaded_function(function: &LoadedFunction) -> Self {
        Self {
            module_id: function.module_id().cloned(),
            function_name: function.name().to_owned(),
        }
    }
}

/// Serializable equivalent of the generic Move VM call kind.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageCallKind {
    Entrypoint,
    Call,
    CallGeneric,
    CallClosure,
    NativeDynamicDispatch,
}

impl From<FunctionCallKind> for UsageCallKind {
    fn from(kind: FunctionCallKind) -> Self {
        match kind {
            FunctionCallKind::Entrypoint => Self::Entrypoint,
            FunctionCallKind::Call => Self::Call,
            FunctionCallKind::CallGeneric => Self::CallGeneric,
            FunctionCallKind::CallClosure => Self::CallClosure,
            FunctionCallKind::NativeDynamicDispatch => Self::NativeDynamicDispatch,
        }
    }
}

/// One invocation of a target framework function.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FunctionCall {
    pub caller: Option<FunctionId>,
    pub callee: FunctionId,
    pub kind: UsageCallKind,
}

/// Function usage collected while executing one user transaction.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TransactionFunctionUsage {
    pub transaction_hash: HashValue,
    pub sender: AccountAddress,
    pub multisig_address: Option<AccountAddress>,
    pub root_function: Option<FunctionId>,
    pub status: TransactionStatus,
    pub calls: Vec<FunctionCall>,
}

/// Consumer installed by an off-chain analysis command.
///
/// Implementations must be thread-safe because archive replay can execute independent chunks on
/// multiple threads. The Move VM calls `is_target_module` before allocating a call record.
pub trait FunctionUsageSink: Send + Sync {
    fn is_target_module(&self, module_id: &ModuleId) -> bool;
    fn record_transaction(&self, usage: TransactionFunctionUsage);
}

static FUNCTION_USAGE_SINK: OnceLock<Mutex<Option<Arc<dyn FunctionUsageSink>>>> = OnceLock::new();

fn sink_slot() -> &'static Mutex<Option<Arc<dyn FunctionUsageSink>>> {
    FUNCTION_USAGE_SINK.get_or_init(|| Mutex::new(None))
}

fn lock_sink_slot() -> MutexGuard<'static, Option<Arc<dyn FunctionUsageSink>>> {
    sink_slot()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Installs the process-wide sink used by the dedicated off-chain replay command.
///
/// Only one analysis may be active in a process. Install the sink before constructing the VMs
/// that execute the analysis. Dropping the returned guard uninstalls the sink for subsequently
/// constructed VMs; existing VMs retain their captured sink.
pub fn install_function_usage_sink(
    sink: Arc<dyn FunctionUsageSink>,
) -> anyhow::Result<FunctionUsageSinkGuard> {
    let mut slot = lock_sink_slot();
    anyhow::ensure!(slot.is_none(), "a function usage sink is already installed");
    *slot = Some(sink);
    Ok(FunctionUsageSinkGuard { _private: () })
}

/// Returns the active off-chain usage sink for a VM being constructed.
///
/// VMs retain this `Option` for their lifetime, avoiding global synchronization on the hot
/// transaction-execution path.
pub(crate) fn get_function_usage_sink() -> Option<Arc<dyn FunctionUsageSink>> {
    lock_sink_slot().clone()
}

pub struct FunctionUsageSinkGuard {
    _private: (),
}

impl Drop for FunctionUsageSinkGuard {
    fn drop(&mut self) {
        *lock_sink_slot() = None;
    }
}

#[derive(Default)]
pub(crate) struct TransactionCalls {
    root_function: Option<FunctionId>,
    calls: Vec<FunctionCall>,
}

pub(crate) type TransactionCallsHandle = Arc<Mutex<TransactionCalls>>;

pub(crate) fn new_transaction_calls_handle() -> TransactionCallsHandle {
    Arc::new(Mutex::new(TransactionCalls::default()))
}

pub(crate) fn finish_transaction_usage(
    handle: TransactionCallsHandle,
    transaction_hash: HashValue,
    sender: AccountAddress,
    multisig_address: Option<AccountAddress>,
    status: TransactionStatus,
) -> TransactionFunctionUsage {
    let mut calls = handle
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    TransactionFunctionUsage {
        transaction_hash,
        sender,
        multisig_address,
        root_function: calls.root_function.take(),
        status,
        calls: std::mem::take(&mut calls.calls),
    }
}

/// Trace-recorder adapter which delegates trace replay data to `inner` and independently records
/// target function calls. `is_enabled` deliberately delegates to `inner`, so usage collection does
/// not change runtime-check behavior.
pub(crate) struct FunctionUsageTraceRecorder<R> {
    inner: R,
    sink: Arc<dyn FunctionUsageSink>,
    calls: TransactionCallsHandle,
}

impl<R> FunctionUsageTraceRecorder<R> {
    pub(crate) fn new(
        inner: R,
        sink: Arc<dyn FunctionUsageSink>,
        calls: TransactionCallsHandle,
    ) -> Self {
        Self { inner, sink, calls }
    }
}

impl<R: TraceRecorder> TraceRecorder for FunctionUsageTraceRecorder<R> {
    fn is_enabled(&self) -> bool {
        self.inner.is_enabled()
    }

    fn finish(self) -> Trace {
        self.inner.finish()
    }

    fn record_successful_instruction(&mut self, instr: &Instruction) {
        self.inner.record_successful_instruction(instr)
    }

    fn record_branch_outcome(&mut self, taken: bool) {
        self.inner.record_branch_outcome(taken)
    }

    fn record_entrypoint(&mut self, function: &LoadedFunction) {
        self.inner.record_entrypoint(function)
    }

    fn record_call_closure(
        &mut self,
        function: &LoadedFunction,
        mask: move_core_types::function::ClosureMask,
    ) {
        self.inner.record_call_closure(function, mask)
    }

    fn record_function_call(
        &mut self,
        caller: Option<&LoadedFunction>,
        callee: &LoadedFunction,
        kind: FunctionCallKind,
    ) {
        self.inner.record_function_call(caller, callee, kind);

        if kind == FunctionCallKind::Entrypoint {
            let mut calls = self
                .calls
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if calls.root_function.is_none() {
                calls.root_function = Some(FunctionId::from_loaded_function(callee));
            }
        }

        let Some(module_id) = callee.module_id() else {
            return;
        };
        if !self.sink.is_target_module(module_id) {
            return;
        }

        let call = FunctionCall {
            caller: caller.map(FunctionId::from_loaded_function),
            callee: FunctionId::from_loaded_function(callee),
            kind: kind.into(),
        };
        self.calls
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .calls
            .push(call);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use move_vm_runtime::execution_tracing::NoOpTraceRecorder;

    #[test]
    fn usage_adapter_does_not_enable_runtime_tracing() {
        struct NoTargets;

        impl FunctionUsageSink for NoTargets {
            fn is_target_module(&self, _module_id: &ModuleId) -> bool {
                false
            }

            fn record_transaction(&self, _usage: TransactionFunctionUsage) {}
        }

        let recorder = FunctionUsageTraceRecorder::new(
            NoOpTraceRecorder,
            Arc::new(NoTargets),
            new_transaction_calls_handle(),
        );
        assert!(!recorder.is_enabled());
    }
}
