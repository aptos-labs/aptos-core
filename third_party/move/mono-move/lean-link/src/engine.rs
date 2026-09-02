// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Compile, load, and run a request's calls against MonoMove.
//!
//! One request compiles its source bundle once and serves the modules from
//! an in-memory provider. Every call then runs in its own interpreter
//! context — the same isolation shape as one transaction per session — with
//! the request's gas budget re-armed per call, so outcomes depend only on
//! the request. Calls are placed and run through the runtime's `CallBuilder`;
//! this module contributes the compile step and the orchestration only.

use crate::{
    marshal::{encode_bcs, parse_address, read_root_results},
    payload::{
        Call, CompileSpec, ExhaustedResource, Identity, Limits, Outcome, Request, Response, Stage,
        PAYLOAD_VERSION,
    },
};
use bytes::Bytes;
use codespan_reporting::term::termcolor::Buffer;
use legacy_move_compiler::{compiled_unit::CompiledUnit, shared::known_attributes::KnownAttribute};
use mono_move_aptos_transaction_executor::production_natives;
use mono_move_core::{
    interner::InternedIdentifier,
    types::{view_type_list, InternedType, InternedTypeList, EMPTY_TYPE_LIST},
    ExecutionErrorKind, GasMeter, Interner, IntoExecutionError, NoResourceProvider,
    VMInternalError, VMResult,
};
use mono_move_global_context::{ExecutionGuard, FunctionIrLookup, GlobalContext, LoadedModule};
use mono_move_loader::{Loader, LoadingPolicy, LoweringPolicy, ModuleProvider};
use mono_move_runtime::{
    error::RuntimeError, InterpreterContext, InterpreterOptions, ProductionNativeRegistry,
    RuntimeStatus,
};
use move_binary_format::CompiledModule;
use move_compiler_v2::Options;
use move_core_types::{account_address::AccountAddress, identifier::Identifier};
use move_model::metadata::LanguageVersion;
use std::{collections::HashMap, path::Path};
use thiserror::Error;

/// The adapter's build identity, stamped by `build.rs`.
pub fn identity(abi: u32) -> Identity {
    Identity {
        abi,
        rustc: env!("LEAN_LINK_RUSTC").to_string(),
        profile: env!("LEAN_LINK_PROFILE").to_string(),
    }
}

/// In-memory module storage for one request's freshly compiled package.
///
/// Every module is its own package: a request compiles one bundle whose
/// members are published together, and nothing outside the bundle is
/// addressable.
pub struct InMemoryModuleProvider {
    module_bytes: HashMap<(AccountAddress, Identifier), Bytes>,
}

#[derive(Debug, Error)]
#[error("deserialization failed: {0}")]
struct DeserializationError(move_binary_format::errors::PartialVMError);

impl IntoExecutionError for DeserializationError {
    fn kind(&self) -> ExecutionErrorKind {
        ExecutionErrorKind::Placeholder
    }
}

impl InMemoryModuleProvider {
    fn new() -> Self {
        Self {
            module_bytes: HashMap::new(),
        }
    }

    fn add_modules(&mut self, modules: &[CompiledModule]) {
        for module in modules {
            let id = module.self_id();
            let mut bytes = Vec::new();
            module
                .serialize(&mut bytes)
                .expect("module serialization should succeed");
            self.module_bytes
                .insert((id.address, id.name), Bytes::from(bytes));
        }
    }
}

impl ModuleProvider for InMemoryModuleProvider {
    fn get_module_bytes(&self, address: &AccountAddress, name: &str) -> VMResult<Option<Bytes>> {
        let Ok(identifier) = Identifier::new(name) else {
            return Ok(None);
        };
        Ok(self.module_bytes.get(&(*address, identifier)).cloned())
    }

    fn deserialize_module(&self, bytes: &[u8]) -> VMResult<CompiledModule> {
        CompiledModule::deserialize(bytes)
            .map_err(|e| VMInternalError::new(DeserializationError(e)))
    }

    fn verify_module(&self, _module: &CompiledModule) -> VMResult<()> {
        // Modules come from the in-process compiler, which produces valid
        // bytecode; the loader still runs its own lowering checks.
        Ok(())
    }

    fn get_same_package_modules(
        &self,
        address: &AccountAddress,
        module_name: &str,
    ) -> VMResult<Vec<Identifier>> {
        // Each compiled module is its own package, satisfying the trait's
        // requirement that the module itself is part of the returned list.
        match Identifier::new(module_name) {
            Ok(identifier)
                if self
                    .module_bytes
                    .contains_key(&(*address, identifier.clone())) =>
            {
                Ok(vec![identifier])
            },
            _ => Ok(vec![]),
        }
    }
}

/// Runs one request to its response.
pub fn run_request(request: &Request, abi: u32) -> Response {
    let identity = identity(abi);
    let modules = match compile_modules(&request.compile) {
        Ok(modules) => modules,
        Err(message) => {
            return Response::uniform_error(request, &identity, Stage::Compile, message)
        },
    };
    let mut provider = InMemoryModuleProvider::new();
    provider.add_modules(&modules);

    let global_ctx = GlobalContext::with_num_execution_workers(1);
    let Some(guard) = global_ctx.try_execution_context(0) else {
        return Response::uniform_error(
            request,
            &identity,
            Stage::Internal,
            "failed to acquire execution guard 0".to_string(),
        );
    };
    let natives = production_natives();

    let outcomes = request
        .calls
        .iter()
        .map(|call| run_call(&guard, natives, &provider, &request.limits, call))
        .collect();
    Response {
        version: PAYLOAD_VERSION,
        identity,
        outcomes,
    }
}

/// Runs one call in a fresh interpreter context.
fn run_call<'guard>(
    guard: &'guard ExecutionGuard<'_>,
    natives: &'guard ProductionNativeRegistry,
    provider: &'guard InMemoryModuleProvider,
    limits: &Limits,
    call: &Call,
) -> Outcome {
    let Some((address, module_name, function_name)) = parse_function(&call.function) else {
        return Outcome::Error {
            stage: Stage::Abi,
            message: format!(
                "function reference {:?} is not `0x…::module::function`",
                call.function
            ),
        };
    };
    let args = match call
        .args
        .iter()
        .map(encode_bcs)
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(args) => args,
        Err(message) => {
            return Outcome::Error {
                stage: Stage::Abi,
                message,
            }
        },
    };
    let signers = match call
        .signers
        .iter()
        .map(|signer| parse_address(signer))
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(signers) => signers,
        Err(message) => {
            return Outcome::Error {
                stage: Stage::Abi,
                message,
            }
        },
    };

    let loader = Loader::new_with_policy(
        guard,
        provider,
        LoadingPolicy::Lazy(LoweringPolicy::Lazy),
        natives,
    );
    let gas_meter = GasMeter::new(limits.gas);
    let mut options = InterpreterOptions::default();
    if let Some(heap_size) = limits.heap {
        options.heap_size = heap_size;
    }
    let mut interp = InterpreterContext::new_with_options(
        loader,
        gas_meter,
        &NoResourceProvider,
        natives,
        options,
    );

    let module_id = guard.module_id_of(&address, module_name.as_ident_str());
    let function_id = guard.identifier_of(function_name.as_ident_str());
    let result = (|| {
        let func = interp.load_function(module_id, function_id, EMPTY_TYPE_LIST)?;
        let mut call = interp.build_call(func)?;
        for signer in &signers {
            call.signer(signer)?;
        }
        for arg in &args {
            call.arg_bcs(arg)?;
        }
        call.run()
    })();
    let gas_used = limits.gas.saturating_sub(interp.gas_balance());
    let gc_count = interp.gc_count();

    match result {
        Ok(RuntimeStatus::Success) => {
            // Resolve the callee's return types through the read set the
            // call left behind.
            let returns = match interp
                .read_set()
                .get_loaded(guard.arena_ref_for_module_id(module_id))
            {
                Ok(loaded) => match return_types(guard, loaded, function_id, EMPTY_TYPE_LIST) {
                    Ok(returns) => returns,
                    Err(error) => {
                        return Outcome::Error {
                            stage: Stage::Run,
                            message: format!("failed to derive return types: {error:#}"),
                        }
                    },
                },
                Err(error) => {
                    return Outcome::Error {
                        stage: Stage::Run,
                        message: format!("failed to look up the loaded module: {error}"),
                    }
                },
            };
            match read_root_results(&interp, &returns) {
                Ok(values) => Outcome::Returned {
                    values,
                    gas_used,
                    gc_count,
                },
                Err(message) => Outcome::Error {
                    stage: Stage::Run,
                    message,
                },
            }
        },
        Ok(RuntimeStatus::Aborted {
            code,
            message,
            location,
        }) => Outcome::Aborted {
            code,
            location: Some(location.to_string()),
            message,
        },
        Err(error) => {
            if let Some(RuntimeError::OutOfHeapMemory { .. }) = error.downcast_ref::<RuntimeError>()
            {
                return Outcome::Exhausted {
                    resource: ExhaustedResource::Heap,
                };
            }
            // Split MonoVM's execution errors by what they say about the
            // program, following the documented meaning of each kind. The
            // harness compares program outcomes, so collapsing all of them
            // into an adapter error would claim no outcome was obtained for
            // failures the program genuinely produced.
            match error.kind() {
                ExecutionErrorKind::OutOfGas => Outcome::Exhausted {
                    resource: ExhaustedResource::Gas,
                },
                // Outcomes of the program. MonoVM reports arithmetic
                // overflow, an out-of-bounds index, a missing resource, or a
                // structural limit this way rather than as an abort, so they
                // are propagated with the kind a caller branches on.
                kind @ (ExecutionErrorKind::InvalidOperation
                | ExecutionErrorKind::RuntimeLimitExceeded) => Outcome::Failed {
                    failure: kind.to_string(),
                    message: format!("{error}"),
                },
                // Not outcomes of the program. An unresolvable callee means
                // the request itself is wrong, and an invariant violation is
                // a VM bug that must stay loud instead of becoming something
                // to compare; an untyped placeholder is classified with them
                // because it carries no claim either way.
                ExecutionErrorKind::LinkingError
                | ExecutionErrorKind::InvariantViolation
                | ExecutionErrorKind::Placeholder => Outcome::Error {
                    stage: Stage::Run,
                    message: format!("{error}"),
                },
            }
        },
    }
}

/// The return types of a loaded module's function, instantiated with
/// `ty_args`.
fn return_types(
    guard: &ExecutionGuard<'_>,
    loaded: &LoadedModule,
    function: InternedIdentifier,
    ty_args: InternedTypeList,
) -> Result<Vec<InternedType>, String> {
    let FunctionIrLookup::Ir(ir) = loaded.get_function_ir(function) else {
        return Err("function has no IR in its loaded module".to_string());
    };
    let returns = loaded
        .ir()
        .module
        .function_signature_at(ir.handle_idx)
        .returns;
    let returns = guard
        .subst_type_list(returns, ty_args)
        .map_err(|error| error.to_string())?;
    Ok(view_type_list(returns).to_vec())
}

/// Parses `0x…::module::function`.
fn parse_function(text: &str) -> Option<(AccountAddress, Identifier, Identifier)> {
    let mut parts = text.split("::");
    let address = parts.next()?;
    let module = parts.next()?;
    let function = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    let address = AccountAddress::from_hex_literal(address).ok()?;
    let module = Identifier::new(module).ok()?;
    let function = Identifier::new(function).ok()?;
    Some((address, module, function))
}

/// Compiles the request's source bundle into its modules, injecting the
/// Move stdlib as dependencies, following the mono-move testsuite's compile
/// path.
fn compile_modules(spec: &CompileSpec) -> Result<Vec<CompiledModule>, String> {
    let language_version = match spec.language {
        2 => LanguageVersion::latest_stable(),
        other => return Err(format!("unsupported language version {other}; expected 2")),
    };
    let tmp_dir = tempfile::tempdir().map_err(|e| format!("failed to create temp dir: {e}"))?;
    let sources = spec
        .sources
        .iter()
        .map(|source| stage_source(&tmp_dir, source))
        .collect::<Result<Vec<_>, _>>()?;
    let dependencies = aptos_move_stdlib::move_stdlib_files();
    let mut named_address_mapping = aptos_move_stdlib::move_stdlib_named_addresses_strings();
    named_address_mapping.extend(
        spec.addresses
            .iter()
            .map(|(name, address)| format!("{name}={address}")),
    );
    run_compiler(Options {
        sources,
        dependencies,
        named_address_mapping,
        known_attributes: KnownAttribute::get_all_attribute_names().clone(),
        language_version: Some(language_version),
        ..Options::default()
    })
}

/// Writes one source file into the staging directory.
fn stage_source(
    tmp_dir: &tempfile::TempDir,
    source: &crate::payload::SourceFile,
) -> Result<String, String> {
    let path = Path::new(tmp_dir.path()).join(&source.name);
    std::fs::write(&path, &source.text)
        .map_err(|e| format!("failed to stage source {name}: {e}", name = source.name))?;
    Ok(path.to_string_lossy().into_owned())
}

/// Runs the v2 compiler and collects the produced modules, scripts dropped.
fn run_compiler(options: Options) -> Result<Vec<CompiledModule>, String> {
    let mut errors = Buffer::no_color();
    let result = {
        let mut emitter = options.error_emitter(&mut errors);
        move_compiler_v2::run_move_compiler(emitter.as_mut(), options)
    };
    let (_env, units) = result.map_err(|e| {
        format!(
            "Move compilation failed:\n{e:#}\n{}",
            String::from_utf8_lossy(&errors.into_inner())
        )
    })?;
    Ok(units
        .into_iter()
        .filter_map(|unit| match unit.into_compiled_unit() {
            CompiledUnit::Module(m) => Some(m.module),
            CompiledUnit::Script(_) => None,
        })
        .collect())
}
