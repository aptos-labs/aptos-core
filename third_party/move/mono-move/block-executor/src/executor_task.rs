// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The mono VM as a Block-STM transaction executor. Mirrors the
//! replay-benchmark harness (`replay-benchmark/src/v2.rs`), with reads served
//! from the multi-version map instead of a captured read-set.
//!
//! P0 scope: entry-function user transactions only (no prologue/epilogue,
//! fees or gas metering); every other transaction produces an empty success
//! record. Parallel execution only.

use crate::{
    entry_call::{classify_entry_params, intern_type_tag, place_args},
    events::finalize_events,
    natives::build_production_natives,
    record::MonoRecord,
    resource_provider::{BlockStmResourceProvider, ReadMode},
    value::MonoValue,
    StateViewModuleProvider,
};
use aptos_block_executor::{task::TransactionExecutor, view::ViewArgs};
use aptos_mvhashmap::types::{Incarnation, TxnIndex};
use aptos_types::{
    contract_event::ContractEvent,
    error::{code_invariant_error, PanicError},
    state_store::{state_key::StateKey, TStateView},
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, AuxiliaryInfo, EntryFunction,
        SignedTransaction, Transaction, TransactionExecutable, TransactionPayload,
        TransactionPayloadInner,
    },
    write_set::WriteOpKind,
};
use aptos_vm_environment::environment::AptosEnvironment;
use mono_move_core::{
    native::NativeExtensions, storage::resource_provider::InMemoryStorageKey, GasMeter, Interner,
};
use mono_move_global_context::GlobalContext;
use mono_move_loader::{Loader, LoadingPolicy, LoweringPolicy};
use mono_move_natives::{
    EventStore, ObjectContextExtension, StorageUsageAtEpochBoundary, TransactionContextExtension,
};
use mono_move_runtime::{
    ExecutionContext, Heap, InterpreterContext, ProductionNativeRegistry, ResourceReadWriteSet,
    RuntimeStatus, StorageRead, StorageWrite, TransactionContext,
};
use move_core_types::{ident_str, vm_status::VMStatus};
use std::{collections::HashMap, sync::Arc};

/// Effectively unbounded gas budget (no gas metering on the mono path).
const GAS_BUDGET: u64 = u64::MAX;

/// One executor per Block-STM worker, running transactions on the mono VM.
pub struct MonoExecutorTask {
    worker_id: usize,
    global_context: Arc<GlobalContext>,
    natives: ProductionNativeRegistry,
}

impl TransactionExecutor for MonoExecutorTask {
    type AuxiliaryInfo = AuxiliaryInfo;
    type Error = VMStatus;
    type Record = MonoRecord;
    type Txn = SignatureVerifiedTransaction;

    fn init(
        _environment: &AptosEnvironment,
        _state_view: &impl TStateView<Key = StateKey>,
        _async_runtime_checks_enabled: bool,
    ) -> Self {
        unimplemented!("the mono executor is initialized via init_v2")
    }

    fn init_v2(worker_id: usize, global_context: Arc<GlobalContext>) -> Self {
        // Interning the native names needs a guard; the names outlive it (the
        // arena persists until maintenance, which is out of scope).
        let natives = {
            let guard = global_context
                .try_execution_context(worker_id)
                .expect("the worker's execution slot is private to its thread");
            build_production_natives(&guard)
        };
        Self {
            worker_id,
            global_context,
            natives,
        }
    }

    fn execute_transaction_v2<S: TStateView<Key = StateKey> + Sync>(
        &self,
        args: &ViewArgs<'_, Self::Record, S>,
        txn: &Self::Txn,
        _auxiliary_info: &Self::AuxiliaryInfo,
        txn_idx: TxnIndex,
    ) -> Result<Self::Record, PanicError> {
        let (Some(versioned_cache), Some(incarnation), Some(is_v2)) = (
            args.versioned_cache(),
            args.incarnation(),
            args.is_blockstm_v2(),
        ) else {
            // Sequential mode fails loudly: P0 supports parallel execution
            // only, and a fallback would re-fail anyway.
            return Err(code_invariant_error(
                "the mono path does not support sequential execution",
            ));
        };
        let record_incarnation = is_v2.then_some(incarnation);

        // Only entry-function user transactions execute on the mono path;
        // everything else commits an empty success record.
        let Some((signed_txn, entry)) = entry_function(txn) else {
            return Ok(MonoRecord::empty_success(record_incarnation));
        };

        self.execute_entry_function(
            args,
            versioned_cache.data(),
            signed_txn,
            entry,
            txn.hash().to_vec(),
            txn_idx,
            incarnation,
            is_v2,
        )
    }
}

impl MonoExecutorTask {
    /// Runs one entry function on the interpreter. Transaction-level failures
    /// (unloadable module, bad arguments, aborts, runtime errors) produce a
    /// success record with empty writes; only infrastructure invariants
    /// return an error.
    #[allow(clippy::too_many_arguments)]
    fn execute_entry_function<S: TStateView<Key = StateKey> + Sync>(
        &self,
        args: &ViewArgs<'_, MonoRecord, S>,
        versioned_data: &aptos_mvhashmap::versioned_data::VersionedData<
            InMemoryStorageKey,
            MonoValue,
        >,
        signed_txn: &SignedTransaction,
        entry: &EntryFunction,
        txn_hash: Vec<u8>,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
        is_v2: bool,
    ) -> Result<MonoRecord, PanicError> {
        let record_incarnation = is_v2.then_some(incarnation);

        // Per-call, worker-private slot: never contended within a worker.
        let guard = self
            .global_context
            .try_execution_context(self.worker_id)
            .ok_or_else(|| code_invariant_error("worker execution slot must be free"))?;

        let module_provider = StateViewModuleProvider::new(args.base_view());
        let loader = Loader::new_with_policy(
            &guard,
            &module_provider,
            LoadingPolicy::Lazy(LoweringPolicy::Lazy),
            &self.natives,
        );

        let read_mode = if is_v2 {
            ReadMode::BlockStmV2 { incarnation }
        } else {
            ReadMode::BlockStmV1
        };
        let resource_provider = BlockStmResourceProvider::new(
            &guard,
            versioned_data,
            args.base_view(),
            txn_idx,
            read_mode,
        );

        let mut extensions = NativeExtensions::new();
        extensions.add(TransactionContextExtension::new(txn_hash, 0, txn_idx, 0));
        extensions.add(ObjectContextExtension::new());
        let usage = args
            .base_view()
            .get_usage()
            .map_err(|e| code_invariant_error(format!("state usage unavailable: {}", e)))?;
        extensions.add(StorageUsageAtEpochBoundary::new(
            usage.items() as u64,
            usage.bytes() as u64,
        ));
        extensions.add(EventStore::new());

        let mut txn_ctx = TransactionContext::new(
            loader,
            GasMeter::new(GAS_BUDGET),
            &resource_provider,
            &self.natives,
        )
        .with_extensions(extensions);

        // Intern the transaction's type arguments.
        let Ok(interned_ty_args) = entry
            .ty_args()
            .iter()
            .map(|tag| intern_type_tag(&guard, tag))
            .collect::<anyhow::Result<Vec<_>>>()
        else {
            return Ok(MonoRecord::empty_success(record_incarnation));
        };
        let ty_arg_list = guard.type_list_of(&interned_ty_args);

        // Load the entry function; this publishes the layouts of the types it
        // touches.
        let module_id = guard
            .intern_address_name(entry.module().address(), entry.module().name())
            .into_global_arena_ptr();
        let function = guard
            .intern_identifier(entry.function())
            .into_global_arena_ptr();
        let func = match txn_ctx.load_function(module_id, function, ty_arg_list) {
            // SAFETY: the pointer lives in a LoadedModule arena kept alive by
            // `guard`.
            Ok(ptr) => unsafe { ptr.as_ref_unchecked() },
            Err(_) => return Ok(MonoRecord::empty_success(record_incarnation)),
        };

        // Classify each parameter as a signer or a value.
        let Ok(params) = classify_entry_params(&module_provider, entry, &guard, ty_arg_list) else {
            return Ok(MonoRecord::empty_success(record_incarnation));
        };

        // Sender bytes backing any `&signer` parameter; must outlive the run.
        let signer_bytes = signed_txn.sender().into_bytes();

        let mut interp = InterpreterContext::new(&mut txn_ctx, func);
        interp.set_rng_seed(txn_idx as u64);

        // TODO(completeness): distinguish aborts from runtime errors once
        // outputs matter; both currently commit with empty writes.
        let mut succeeded = place_args(&mut interp, func, &params, &signer_bytes, entry.args())
            .is_ok()
            && matches!(interp.run(), Ok(RuntimeStatus::Success));

        // Serialize the emitted events while the heap is still live (their
        // interior pointers reach into it). A finalization failure demotes
        // the run to failed, so the divergence is visible in comparison
        // rather than silently dropping events.
        let mut events = vec![];
        if succeeded {
            // SAFETY: `interp` (and so its heap) and `guard` are live.
            match unsafe { finalize_events(interp.extensions(), &guard) } {
                Ok(finalized) => events = finalized,
                Err(_) => succeeded = false,
            }
        }

        let (heap, rws) = interp.into_storage_effects();
        if resource_provider.speculative_failure() {
            return Ok(MonoRecord::speculative_failure(
                "read observed a speculative dependency".to_string(),
                record_incarnation,
            ));
        }
        Ok(build_record(heap, rws, succeeded, events, record_incarnation))
    }
}

/// The entry function of a user transaction, unless the mono path must skip
/// it (non-entry payloads, module publishing).
fn entry_function(
    txn: &SignatureVerifiedTransaction,
) -> Option<(&SignedTransaction, &EntryFunction)> {
    let signed_txn = match txn {
        SignatureVerifiedTransaction::Valid(Transaction::UserTransaction(signed_txn)) => signed_txn,
        SignatureVerifiedTransaction::Valid(
            Transaction::GenesisTransaction(_)
            | Transaction::BlockMetadata(_)
            | Transaction::StateCheckpoint(_)
            | Transaction::ValidatorTransaction(_)
            | Transaction::BlockMetadataExt(_)
            | Transaction::BlockEpilogue(_),
        )
        | SignatureVerifiedTransaction::Invalid(_) => return None,
    };
    let entry = match signed_txn.payload() {
        TransactionPayload::EntryFunction(entry) => entry,
        TransactionPayload::Payload(TransactionPayloadInner::V1 {
            executable: TransactionExecutable::EntryFunction(entry),
            extra_config: _,
        }) => entry,
        TransactionPayload::Payload(TransactionPayloadInner::V1 {
            executable:
                TransactionExecutable::Script(_)
                | TransactionExecutable::Empty
                | TransactionExecutable::Encrypted,
            extra_config: _,
        })
        | TransactionPayload::Script(_)
        | TransactionPayload::ModuleBundle(_)
        | TransactionPayload::Multisig(_)
        | TransactionPayload::EncryptedPayload(_) => return None,
    };
    // Module publishing is unsupported: executing the wrapper entry function
    // without the publish effect would silently diverge.
    if entry.module().name() == ident_str!("code")
        && entry.function() == ident_str!("publish_package_txn")
    {
        return None;
    }
    Some((signed_txn, entry))
}

/// Builds the record from the interpreter's read-write set: reads (with the
/// versions the provider stamped) always; writes and events only for
/// successful runs (`events` is already empty otherwise).
fn build_record(
    heap: Heap,
    rws: ResourceReadWriteSet,
    succeeded: bool,
    events: Vec<ContractEvent>,
    incarnation: Option<Incarnation>,
) -> MonoRecord {
    let heap = Arc::new(heap);
    let mut reads = HashMap::new();
    let mut writes = HashMap::new();
    for (key, entry) in rws.entries() {
        reads.insert(key.clone(), entry.read.version());
        if !succeeded {
            continue;
        }
        match entry.write {
            StorageWrite::NotModified => {},
            StorageWrite::Deleted { .. } => {
                writes.insert(key.clone(), MonoValue::Deletion);
            },
            StorageWrite::LocalHeap { ptr, .. } => {
                let kind = match &entry.read {
                    StorageRead::DoesNotExist { .. } => WriteOpKind::Creation,
                    StorageRead::ExternalHeap { .. } => WriteOpKind::Modification,
                };
                writes.insert(key.clone(), MonoValue::Write {
                    ptr,
                    kind,
                    heap: heap.clone(),
                });
            },
        }
    }
    MonoRecord::from_execution(reads, writes, events, incarnation)
}
