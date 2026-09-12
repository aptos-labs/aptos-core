// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Module storage, publishing checks, and function execution for one
//! transactional test.

use crate::{
    engine::build_natives,
    extensions::seed_extensions,
    module_provider::InMemoryModuleProvider,
    resource_provider::{StorageResourceProvider, MATERIALIZATION_HEAP_SIZE},
};
use bytes::Bytes;
use mono_move_core::{
    intern_type_tag,
    interner::{InternedIdentifier, InternedModuleId},
    nominal_tag,
    storage::resource_provider::InMemoryStorageKey,
    type_tag_of,
    types::{is_signer_or_signer_immut_ref, InternedType, InternedTypeList},
    BytecodeOffset, Function, FunctionDefinitionIndex, GasMeter, Interner, VMInternalError,
};
use mono_move_global_context::{view_type_list, ExecutionGuard, GlobalContext};
use mono_move_loader::{Loader, LoaderError, LoadingPolicy, LoweringPolicy, ModuleReadSet};
use mono_move_runtime::{
    serialize, InterpreterContext, RuntimeError, RuntimeStatus, SessionEffects, WriteClass,
};
use move_binary_format::{compatibility::Compatibility, errors::VMError};
use move_core_types::{
    account_address::AccountAddress,
    effects::{ChangeSet, Op},
    identifier::IdentStr,
    language_storage::{ModuleId, StructTag, TypeTag},
    value::{MoveStruct, MoveStructLayout, MoveValue, MASTER_SIGNER_VARIANT},
    vm_status::AbortLocation,
};
use move_transactional_test_runner::vm_test_harness::create_runtime_environment;
use move_vm_runtime::{config::VMConfig, AsUnsyncModuleStorage, StagingModuleStorage};
use move_vm_test_utils::InMemoryStorage;
use std::ptr::NonNull;
use thiserror::Error;

/// Published modules and global state shared by V1's publishing checks and
/// MonoVM's execution.
///
/// V1 stages, links, and checks compatibility against [`InMemoryStorage`],
/// which also holds the committed resources as canonical BCS. MonoVM's
/// [`InMemoryModuleProvider`] supplies the same module bytes to its loader,
/// and each run reads resources from the storage through a
/// [`StorageResourceProvider`]. [`commit`](Self::commit) updates both module
/// stores together; a successful [`run`](Self::run) writes its effects back
/// into the storage.
///
/// Each operation uses one execution guard, then releases it and resets the
/// [`GlobalContext`]'s caches and arenas before returning, so nothing bound to
/// an arena (interned types, materialized values) outlives its operation and
/// subsequent operations read republished modules from storage.
pub struct TransactionalSession {
    ctx: GlobalContext,
    storage: InMemoryStorage,
    module_provider: InMemoryModuleProvider,
}

/// Failure in V1 publishing checks or MonoVM loading.
#[derive(Debug, Error)]
pub enum PublishError {
    /// V1's publishing checks rejected the bundle.
    #[error(transparent)]
    Staging(VMError),
    /// V1 accepted the bundle, but MonoVM failed to load a module.
    /// Loading includes deserialization, verification, and translation, but no lowering.
    #[error("Unable to load module '{module}' into MonoVM. Got error: {error}")]
    MonoLoad {
        module: ModuleId,
        error: VMInternalError,
    },
}

/// How a run ended. Only a successful run commits its effects.
#[derive(Debug)]
pub enum RunOutcome {
    /// The function returned; its effects are committed. Each return value is
    /// BCS with its type tag.
    Success {
        return_values: Vec<(TypeTag, Vec<u8>)>,
    },
    /// The function aborted; nothing is committed. `offset` names the
    /// aborting instruction unless a native raised the abort.
    Aborted {
        code: u64,
        message: Option<String>,
        location: AbortLocation,
        offset: Option<(FunctionDefinitionIndex, BytecodeOffset)>,
    },
}

/// Why a run produced no [`RunOutcome`].
#[derive(Debug, Error)]
pub enum RunError {
    /// The task's signers and arguments do not fit the function's parameters.
    #[error(transparent)]
    Arguments(ArgumentError),
    /// MonoVM failed to load or run the function in a way V1 could report.
    #[error(transparent)]
    Vm(VMInternalError),
    /// MonoVM does not implement a feature the run needs: a construct its
    /// lowering skips, a missing native, or a runtime operation it rejects.
    #[error("unsupported by MonoVM: {0}")]
    VmUnsupported(String),
    /// The session lacks something the run needs: entry type-argument
    /// handling, multiple return values, return values of natives or of types
    /// without a type tag, writes outside plain resources.
    #[error("the MonoVM transactional session cannot {0}")]
    Unsupported(String),
    /// V1's storage rejected the effects.
    #[error("the effects could not be committed to storage: {0}")]
    Commit(String),
}

/// An argument shape or decoding failure, as V1 classifies it.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ArgumentError {
    /// Counting signers as arguments, as V1 does.
    #[error("argument length mismatch: expected {expected} got {actual}")]
    CountMismatch { expected: usize, actual: usize },
    #[error("an argument does not decode as its parameter type")]
    Undecodable,
}

impl From<VMInternalError> for RunError {
    /// Separates MonoVM's own feature gaps from failures V1 could also report,
    /// so a baseline records them as gaps rather than as VM statuses.
    // TODO(completeness): lowering failures on unsupported constructs (function
    // value equality) surface as `LoweringError` invariant violations, not as
    // skips, so they still render as VM statuses here.
    fn from(err: VMInternalError) -> Self {
        if let Some(RuntimeError::Unsupported(what)) = err.downcast_ref::<RuntimeError>() {
            return RunError::VmUnsupported(what.to_string());
        }
        match err.downcast_ref::<LoaderError>() {
            Some(LoaderError::LoweringSkipped { reason }) => {
                RunError::VmUnsupported(reason.to_string())
            },
            Some(native @ LoaderError::NativeFunctionNotLoadable { .. }) => {
                RunError::VmUnsupported(native.to_string())
            },
            Some(
                LoaderError::ModuleNotFound { .. }
                | LoaderError::FunctionNotFound { .. }
                | LoaderError::ScriptDeserializationFailed { .. }
                | LoaderError::ScriptVerificationFailed { .. }
                | LoaderError::GlobalContext(_)
                | LoaderError::InvariantViolation(_),
            )
            | None => RunError::Vm(err),
        }
    }
}

impl TransactionalSession {
    pub fn new(vm_config: &VMConfig) -> Self {
        Self {
            ctx: GlobalContext::with_num_execution_workers(1),
            storage: InMemoryStorage::new_with_runtime_environment(create_runtime_environment(
                vm_config.clone(),
            )),
            module_provider: InMemoryModuleProvider::new(),
        }
    }

    /// V1's view of the published code and the committed state.
    pub fn storage(&self) -> &InMemoryStorage {
        &self.storage
    }

    /// Publishes `bundle` from `sender` after V1's checks under `compat` and
    /// MonoVM's loading checks. Modules are translated without lowering.
    /// Both stores remain unchanged if either check fails.
    pub fn publish(
        &mut self,
        sender: &AccountAddress,
        compat: Compatibility,
        bundle: Vec<Bytes>,
    ) -> Result<(), PublishError> {
        let verified = StagingModuleStorage::create_with_compat_config(
            sender,
            compat,
            &self.storage.as_unsync_module_storage(),
            bundle,
        )
        .map_err(PublishError::Staging)?
        .release_verified_module_bundle()
        .into_iter()
        .collect::<Vec<_>>();
        self.load_into_mono(&verified)?;
        self.commit(verified);
        Ok(())
    }

    /// Runs `module::function<ty_args>` on MonoVM, unmetered, with `signers`
    /// and BCS `args` placed positionally as V1 places them (signers first).
    /// A successful run commits its resource writes to the storage; an abort
    /// or error commits nothing.
    pub fn run(
        &mut self,
        module: &ModuleId,
        function: &IdentStr,
        ty_args: &[TypeTag],
        signers: &[AccountAddress],
        args: &[Vec<u8>],
    ) -> Result<RunOutcome, RunError> {
        let (outcome, changes) = with_guard(&mut self.ctx, |guard| -> Result<_, RunError> {
            let natives = build_natives();
            let resources =
                StorageResourceProvider::new(guard, &self.storage, MATERIALIZATION_HEAP_SIZE);
            let loader = Loader::new_with_policy(
                guard,
                &self.module_provider,
                LoadingPolicy::Lazy(LoweringPolicy::Lazy),
                natives,
            );
            let mut interp =
                InterpreterContext::new(loader, GasMeter::with_max_budget(), &resources, natives)
                    .with_extensions(seed_extensions(false));
            let outcome = execute(guard, &mut interp, module, function, ty_args, signers, args)?;
            let changes = match &outcome {
                RunOutcome::Success { .. } => resource_changes(&interp.finish())?,
                RunOutcome::Aborted { .. } => ChangeSet::new(),
            };
            Ok((outcome, changes))
        })?;
        self.storage
            .apply(changes)
            .map_err(|err| RunError::Commit(err.to_string()))?;
        Ok(outcome)
    }

    /// Updates both stores with modules that have passed V1's publishing checks.
    pub(crate) fn commit(&mut self, modules: impl IntoIterator<Item = (ModuleId, Bytes)>) {
        for (id, bytes) in modules {
            self.storage
                .add_module_bytes(id.address(), id.name(), bytes.clone());
            self.module_provider
                .add_module_bytes(*id.address(), id.name().to_owned(), bytes);
        }
    }

    /// Deserializes, verifies, and translates each module without lowering.
    /// A temporary provider includes the candidate modules, so failed loads
    /// leave the stored modules unchanged.
    fn load_into_mono(&mut self, modules: &[(ModuleId, Bytes)]) -> Result<(), PublishError> {
        let mut provider = self.module_provider.clone();
        for (id, bytes) in modules {
            provider.add_module_bytes(*id.address(), id.name().to_owned(), bytes.clone());
        }
        with_guard(&mut self.ctx, |guard| {
            let loader = Loader::new_with_policy(
                guard,
                &provider,
                LoadingPolicy::Lazy(LoweringPolicy::Lazy),
                build_natives(),
            );
            modules.iter().try_for_each(|(id, _)| {
                let mut read_set = ModuleReadSet::new();
                let mut gas_meter = GasMeter::with_max_budget();
                loader
                    .load_module(
                        &mut read_set,
                        &mut gas_meter,
                        guard.intern_address_name(id.address(), id.name()),
                    )
                    .map(|_| ())
                    .map_err(|error| PublishError::MonoLoad {
                        module: id.clone(),
                        error,
                    })
            })
        })
    }
}

/// Runs `operation` under a fresh execution guard, then resets the context's
/// caches and arenas so nothing arena-bound outlives the operation.
fn with_guard<T>(ctx: &mut GlobalContext, operation: impl FnOnce(&ExecutionGuard<'_>) -> T) -> T {
    let result = {
        let guard = ctx
            .try_execution_context(0)
            .expect("the session releases its guard after every operation");
        operation(&guard)
    };
    ctx.maintenance_context().reset_arena_pool();
    result
}

/// Loads and calls one function on `interp`, then serializes its return value.
fn execute(
    guard: &ExecutionGuard<'_>,
    interp: &mut InterpreterContext<'_>,
    module: &ModuleId,
    function: &IdentStr,
    ty_args: &[TypeTag],
    signers: &[AccountAddress],
    args: &[Vec<u8>],
) -> Result<RunOutcome, RunError> {
    let ty_args = ty_args
        .iter()
        .map(|tag| intern_type_tag(tag, guard))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| RunError::Unsupported(format!("intern the type arguments: {err:#}")))?;
    let ty_args = guard.type_list_of(&ty_args);
    let module_id = guard.module_id_of(module.address(), module.name());
    let function_id = guard.identifier_of(function);

    // TODO(correctness): `load_function` performs no entry validation of
    // `ty_args` (arity, ability constraints, struct constraints), so MonoVM
    // runs instantiations V1 rejects. The check belongs in the loader; once it
    // exists, `describe` needs the V1 statuses for it.
    let func = interp.load_function(module_id, function_id, ty_args)?;
    match call(interp, func, signers, args)? {
        RuntimeStatus::Aborted {
            code,
            message,
            location,
            offset,
        } => Ok(RunOutcome::Aborted {
            code,
            message,
            location,
            offset,
        }),
        RuntimeStatus::Success => Ok(RunOutcome::Success {
            return_values: return_values(guard, interp, module_id, function_id, ty_args)?,
        }),
    }
}

/// One parameter's value: a signer address, or BCS bytes.
enum Placement<'a> {
    Signer(AccountAddress),
    Bcs(&'a [u8]),
}

/// Calls the function with `signers` followed by `args`, decoded by parameter
/// type. Signers use V1's single-variant enum encoding containing an address,
/// preserving V1's decoding behavior for misplaced arguments.
fn call(
    interp: &mut InterpreterContext<'_>,
    func: &Function,
    signers: &[AccountAddress],
    args: &[Vec<u8>],
) -> Result<RuntimeStatus, RunError> {
    let expected = func.param_tys.len();
    let actual = signers.len() + args.len();
    if expected != actual {
        return Err(RunError::Arguments(ArgumentError::CountMismatch {
            expected,
            actual,
        }));
    }
    let encoded_signers = signers
        .iter()
        .map(|signer| {
            MoveValue::Signer(*signer)
                .simple_serialize()
                .expect("a signer serializes")
        })
        .collect::<Vec<_>>();
    let blobs = encoded_signers.iter().chain(args).map(Vec::as_slice);
    let placements = func
        .param_tys
        .iter()
        .zip(blobs)
        .map(|(&ty, blob)| {
            if is_signer_or_signer_immut_ref(ty) {
                decode_signer(blob)
                    .map(Placement::Signer)
                    .ok_or(RunError::Arguments(ArgumentError::Undecodable))
            } else {
                Ok(Placement::Bcs(blob))
            }
        })
        .collect::<Result<Vec<_>, RunError>>()?;

    let mut call = interp.build_call(func)?;
    for placement in &placements {
        match placement {
            Placement::Signer(address) => call.signer(address)?,
            Placement::Bcs(bytes) => call.arg_bcs(bytes).map_err(|err| {
                if err
                    .downcast_ref::<RuntimeError>()
                    .is_some_and(RuntimeError::is_bcs_decode_error)
                {
                    RunError::Arguments(ArgumentError::Undecodable)
                } else {
                    RunError::from(err)
                }
            })?,
        }
    }
    call.run().map_err(RunError::from)
}

/// The address of a signer in V1's wire encoding, or `None` for any other
/// bytes.
fn decode_signer(blob: &[u8]) -> Option<AccountAddress> {
    let value =
        MoveStruct::simple_deserialize(blob, &MoveStructLayout::signer_serialization_layout())
            .ok()?;
    match value {
        MoveStruct::RuntimeVariant(MASTER_SIGNER_VARIANT, fields) => match fields.as_slice() {
            [MoveValue::Address(address)] => Some(*address),
            _ => None,
        },
        MoveStruct::RuntimeVariant(..)
        | MoveStruct::Runtime(_)
        | MoveStruct::WithFields(_)
        | MoveStruct::WithTypes { .. }
        | MoveStruct::WithVariantFields(..) => None,
    }
}

/// Serializes the value a completed call returned, with its type tag. The
/// runtime exposes the root frame's single return slot; more return values
/// are not representable yet.
fn return_values(
    guard: &ExecutionGuard<'_>,
    interp: &InterpreterContext<'_>,
    module_id: InternedModuleId,
    function: InternedIdentifier,
    ty_args: InternedTypeList,
) -> Result<Vec<(TypeTag, Vec<u8>)>, RunError> {
    let loaded = interp
        .read_set()
        .get_loaded(guard.arena_ref_for_module_id(module_id))?;
    let returns = match loaded.function_return_types(guard, function, ty_args) {
        Some(Ok(returns)) => view_type_list(returns),
        Some(Err(err)) => {
            return Err(RunError::Unsupported(format!(
                "instantiate the return types: {err}"
            )))
        },
        None => {
            return Err(RunError::Unsupported(
                "return the values of a native function".to_string(),
            ))
        },
    };
    match returns {
        [] => Ok(vec![]),
        [ty] => {
            let tag = type_tag_of(*ty).ok_or_else(|| {
                RunError::Unsupported("return a value whose type has no type tag".to_string())
            })?;
            // TODO(correctness): a returned `signer` serializes as a bare address,
            // while V1 renders its one-variant enum encoding; only an empty
            // `vector<signer>` is returned anywhere in the corpus today.
            let bytes = interp.serialize_root_result(*ty)?;
            Ok(vec![(tag, bytes)])
        },
        returns => Err(RunError::Unsupported(format!(
            "return {} values from one call",
            returns.len()
        ))),
    }
}

/// Converts a successful run's resource writes to a V1 change set.
/// Resource group and table writes are unsupported. Among conversion errors,
/// returns the one whose message sorts first, independent of write order.
fn resource_changes(effects: &SessionEffects<'_>) -> Result<ChangeSet, RunError> {
    let layouts = effects.layout_provider();
    let write_op = |key: &InMemoryStorageKey,
                    class: WriteClass,
                    group: Option<InternedType>|
     -> Result<(AccountAddress, StructTag, Op<Bytes>), RunError> {
        if group.is_some() {
            return Err(RunError::Unsupported(
                "commit resource group writes".to_string(),
            ));
        }
        let InMemoryStorageKey::Resource { address, ty } = key else {
            return Err(RunError::Unsupported("commit table writes".to_string()));
        };
        let tag = nominal_tag(*ty).map_err(|err| RunError::Commit(format!("{err:#}")))?;
        // SAFETY: written pointers refer to live values in the effects' frozen
        // heap, and `layouts` is the guard that described them.
        let written = |ptr: NonNull<u8>| {
            unsafe { serialize(layouts, ptr.as_ptr(), *ty) }
                .map(Bytes::from)
                .map_err(RunError::from)
        };
        let op = match class {
            WriteClass::Creation(ptr) => Op::New(written(ptr)?),
            WriteClass::Modification(ptr) => Op::Modify(written(ptr)?),
            WriteClass::Deletion => Op::Delete,
        };
        Ok((*address, tag, op))
    };

    let mut changes = ChangeSet::new();
    let mut failures = Vec::new();
    for (key, class, group) in effects.read_write_set().writes_unordered() {
        match write_op(key, class, group) {
            Ok((address, tag, op)) => changes
                .add_resource_op(address, tag, op)
                .map_err(|err| RunError::Commit(format!("{err:#}")))?,
            Err(err) => failures.push(err),
        }
    }
    match failures.into_iter().min_by_key(|err| err.to_string()) {
        Some(failure) => Err(failure),
        None => Ok(changes),
    }
}
