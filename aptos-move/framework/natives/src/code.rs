// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    publish_verification::publish::validate_publish_request,
    transaction_context::NativeTransactionContext, unzip_metadata_str,
};
use anyhow::bail;
use aptos_gas_schedule::{gas_feature_versions, gas_params::natives::aptos_framework::*};
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use aptos_types::{
    chain_id::ChainId,
    move_any::Any,
    on_chain_config::{FeatureFlag, OnChainConfig, TimedFeatureFlag},
    state_store::state_key::StateKey,
    transaction::ModuleBundle,
    vm_status::StatusCode,
};
use better_any::{Tid, TidAble};
use bytes::Bytes;
use move_binary_format::{
    access::ModuleAccess,
    check_complexity::check_module_complexity,
    compatibility::Compatibility,
    errors::{PartialVMError, PartialVMResult},
    CompiledModule,
};
use move_core_types::{
    account_address::AccountAddress,
    gas_algebra::NumBytes,
    ident_str,
    identifier::IdentStr,
    language_storage::ModuleId,
    move_resource::{MoveResource, MoveStructType},
};
use move_vm_runtime::{
    native_extensions::{NativeRuntimeRefCheckModelsCompleted, SessionListener},
    native_functions::NativeFunction,
    ModuleStorage, StagingModuleStorage, WithRuntimeEnvironment,
};
use move_vm_types::{
    gas::DependencyKind,
    loaded_data::runtime_types::Type,
    values::{Struct, Value},
};
use serde::{Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};
use std::{
    collections::{btree_map::Entry, BTreeMap, BTreeSet, VecDeque},
    fmt,
    str::FromStr,
};

/// The package registry at the given address.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct PackageRegistry {
    /// Packages installed at this address.
    pub packages: Vec<PackageMetadata>,
}

impl OnChainConfig for PackageRegistry {
    const MODULE_IDENTIFIER: &'static str = "code";
    const TYPE_IDENTIFIER: &'static str = "PackageRegistry";
}

impl MoveStructType for PackageRegistry {
    const MODULE_NAME: &'static IdentStr = ident_str!("code");
    const STRUCT_NAME: &'static IdentStr = ident_str!("PackageRegistry");
}

impl MoveResource for PackageRegistry {}

/// The PackageMetadata type. This must be kept in sync with `code.move`. Documentation is
/// also found there.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct PackageMetadata {
    pub name: String,
    pub upgrade_policy: UpgradePolicy,
    pub upgrade_number: u64,
    pub source_digest: String,
    #[serde(with = "serde_bytes")]
    pub manifest: Vec<u8>,
    pub modules: Vec<ModuleMetadata>,
    pub deps: Vec<PackageDep>,
    pub extension: Option<Any>,
}

impl fmt::Display for PackageMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Package name:{}", self.name)?;
        writeln!(f, "Upgrade policy:{}", self.upgrade_policy)?;
        writeln!(f, "Upgrade number:{}", self.upgrade_number)?;
        writeln!(f, "Source digest:{}", self.source_digest)?;
        let manifest_str = unzip_metadata_str(&self.manifest).unwrap();
        writeln!(f, "Manifest:")?;
        writeln!(f, "{}", manifest_str)?;
        writeln!(f, "Package Dependency:")?;
        for dep in &self.deps {
            writeln!(f, "{:?}", dep)?;
        }
        writeln!(f, "extension:{:?}", self.extension)?;
        writeln!(f, "Modules:")?;
        for module in &self.modules {
            writeln!(f, "{}", module)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd)]
pub struct PackageDep {
    pub account: AccountAddress,
    pub package_name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModuleMetadata {
    pub name: String,
    #[serde(with = "serde_bytes")]
    pub source: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub source_map: Vec<u8>,
    pub extension: Option<Any>,
}

impl fmt::Display for ModuleMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Module name:{}", self.name)?;
        if !self.source.is_empty() {
            writeln!(f, "Source code:")?;
            let source = unzip_metadata_str(&self.source).unwrap();
            writeln!(f, "{}", source)?;
        }
        if !self.source_map.is_empty() {
            writeln!(f, "Source map:")?;
            let source_map = unzip_metadata_str(&self.source_map).unwrap();
            writeln!(f, "{}", source_map)?;
        }
        writeln!(f, "Module extension:{:?}", self.extension)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpgradePolicy {
    pub policy: u8,
}

impl UpgradePolicy {
    pub fn arbitrary() -> Self {
        UpgradePolicy { policy: 0 }
    }

    pub fn compat() -> Self {
        UpgradePolicy { policy: 1 }
    }

    pub fn immutable() -> Self {
        UpgradePolicy { policy: 2 }
    }
}

impl FromStr for UpgradePolicy {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "arbitrary" => Ok(UpgradePolicy::arbitrary()),
            "compatible" => Ok(UpgradePolicy::compat()),
            "immutable" => Ok(UpgradePolicy::immutable()),
            _ => bail!("unknown policy"),
        }
    }
}

impl fmt::Display for UpgradePolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.policy {
            0 => "arbitrary",
            1 => "compatible",
            _ => "immutable",
        })
    }
}

// ========================================================================================
// Code Publishing Logic

/// Abort code when code publishing is requested twice (0x03 == INVALID_STATE)
const EALREADY_REQUESTED: u64 = 0x03_0000;

const ARBITRARY_POLICY: u8 = 0;

/// The native code context.
#[derive(Tid)]
pub struct NativeCodeContext {
    /// If false, publish requests are ignored and any attempts to publish code result in runtime
    /// errors.
    enabled: bool,
    /// Possibly stores (if not [None]) the request to publish a module bundle. The request is made
    /// using the native code defined in this context. It is later extracted by the VM for further
    /// checks and processing the actual publish.
    requested_module_bundle: Option<PublishRequest>,
}

impl SessionListener for NativeCodeContext {
    fn start(&mut self, _session_hash: &[u8; 32], _script_hash: &[u8], _session_counter: u8) {
        // TODO(sessions): consider not enabling context for prologue.
        self.enabled = true;
        self.requested_module_bundle = None;
    }

    fn finish(&mut self) {
        // No state changes to save.
    }

    fn abort(&mut self) {
        // No state changes to abort. Context will be reset on new session's start.
    }
}

impl NativeRuntimeRefCheckModelsCompleted for NativeCodeContext {
    // No native functions in this context return references, so no models to add.
}

impl NativeCodeContext {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            enabled: true,
            requested_module_bundle: None,
        }
    }

    pub fn extract_publish_request(&mut self) -> Option<PublishRequest> {
        if !self.enabled {
            return None;
        }

        self.enabled = false;
        self.requested_module_bundle.take()
    }
}

/// Represents a request for code publishing made from a native call and to be processed
/// by the Aptos VM.
pub struct PublishRequest {
    pub destination: AccountAddress,
    pub bundle: ModuleBundle,
    pub expected_modules: BTreeSet<String>,
    /// Allowed module dependencies. Empty for no restrictions. An empty string in the set
    /// allows all modules from that address.
    pub allowed_deps: Option<BTreeMap<AccountAddress, BTreeSet<String>>>,
    pub check_compat: bool,
}

/// Gets the string value embedded in a Move `string::String` struct.
fn get_move_string(v: Value) -> PartialVMResult<String> {
    let bytes = v
        .value_as::<Struct>()?
        .unpack()?
        .next()
        .ok_or_else(|| PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR))?
        .value_as::<Vec<u8>>()?;
    String::from_utf8(bytes).map_err(|_| PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR))
}

/// Gets the fields of the `code::AllowedDep` helper structure.
fn unpack_allowed_dep(v: Value) -> PartialVMResult<(AccountAddress, String)> {
    let mut fields = v.value_as::<Struct>()?.unpack()?.collect::<Vec<_>>();
    if fields.len() != 2 {
        return Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR));
    }
    let module_name = get_move_string(fields.pop().unwrap())?;
    let account = fields.pop().unwrap().value_as::<AccountAddress>()?;
    Ok((account, module_name))
}

/***************************************************************************************************
 * native fun request_publish(
 *     destination: address,
 *     expected_modules: vector<String>,
 *     code: vector<vector<u8>>,
 *     policy: u8
 * )
 *
 * _and_
 *
 *  native fun request_publish_with_allowed_deps(
 *      owner: address,
 *      expected_modules: vector<String>,
 *      allowed_deps: vector<AllowedDep>,
 *      bundle: vector<vector<u8>>,
 *      policy: u8
 *  );
 *   gas cost: base_cost + unit_cost * bytes_len
 *
 **************************************************************************************************/
fn native_request_publish(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(matches!(args.len(), 4 | 5));
    let with_allowed_deps = args.len() == 5;

    context.charge(CODE_REQUEST_PUBLISH_BASE)?;

    let policy = safely_pop_arg!(args, u8);
    let mut code = vec![];
    for module in safely_pop_arg!(args, Vec<Value>) {
        let module_code = module.value_as::<Vec<u8>>()?;

        context.charge(CODE_REQUEST_PUBLISH_PER_BYTE * NumBytes::new(module_code.len() as u64))?;
        code.push(module_code);
    }

    let allowed_deps = if with_allowed_deps {
        let mut allowed_deps: BTreeMap<AccountAddress, BTreeSet<String>> = BTreeMap::new();

        for dep in safely_pop_arg!(args, Vec<Value>) {
            let (account, module_name) = unpack_allowed_dep(dep)?;

            let entry = allowed_deps.entry(account);

            if let Entry::Vacant(_) = &entry {
                // TODO: Is the 32 here supposed to indicate the length of an account address in bytes?
                context.charge(CODE_REQUEST_PUBLISH_PER_BYTE * NumBytes::new(32))?;
            }

            context
                .charge(CODE_REQUEST_PUBLISH_PER_BYTE * NumBytes::new(module_name.len() as u64))?;
            entry.or_default().insert(module_name);
        }

        Some(allowed_deps)
    } else {
        None
    };

    let mut expected_modules = BTreeSet::new();
    for name in safely_pop_arg!(args, Vec<Value>) {
        let str = get_move_string(name)?;

        // TODO(Gas): fine tune the gas formula
        context.charge(CODE_REQUEST_PUBLISH_PER_BYTE * NumBytes::new(str.len() as u64))?;
        expected_modules.insert(str);
    }

    let destination = safely_pop_arg!(args, AccountAddress);

    // Add own modules to allowed deps
    let allowed_deps = allowed_deps.map(|mut allowed| {
        allowed
            .entry(destination)
            .or_default()
            .extend(expected_modules.clone());
        allowed
    });

    let code_context = context.extensions_mut().get_mut::<NativeCodeContext>();
    if code_context.requested_module_bundle.is_some() || !code_context.enabled {
        // Can't request second time or if publish requests are not allowed.
        return Err(SafeNativeError::abort(EALREADY_REQUESTED));
    }
    code_context.requested_module_bundle = Some(PublishRequest {
        destination,
        bundle: ModuleBundle::new(code),
        expected_modules,
        allowed_deps,
        check_compat: policy != ARBITRARY_POLICY,
    });

    Ok(smallvec![])
}

/***************************************************************************************************
 * native fun verify_package(
 *     owner: address,
 *     expected_modules: vector<String>,
 *     allowed_deps: vector<AllowedDep>,
 *     bundle: vector<vector<u8>>,
 *     policy: u8
 * )
 *
 * Runs the same verification and gas charging the AptosVM performs for a published bundle
 * (compatibility, bytecode verification, linking, metadata / resource-group / event checks),
 * but from within the publishing transaction. Aborts on any failure. Does not run
 * `init_module` and does not materialize module writes: the verified bytecode is written to a
 * queue resource by the caller and materialized at the block epilogue.
 *
 **************************************************************************************************/
fn native_verify_package(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 5);

    context.charge(CODE_REQUEST_PUBLISH_BASE)?;

    // The policy is only used by the Move caller to update the package registry; it is
    // deliberately ignored here (arbitrary upgrade policy is rejected before this native is
    // called, so compatibility is always checked).
    let _policy = safely_pop_arg!(args, u8);

    let mut code = vec![];
    for module in safely_pop_arg!(args, Vec<Value>) {
        let module_code = module.value_as::<Vec<u8>>()?;
        context.charge(CODE_REQUEST_PUBLISH_PER_BYTE * NumBytes::new(module_code.len() as u64))?;
        code.push(module_code);
    }

    let mut allowed_deps: BTreeMap<AccountAddress, BTreeSet<String>> = BTreeMap::new();
    for dep in safely_pop_arg!(args, Vec<Value>) {
        let (account, module_name) = unpack_allowed_dep(dep)?;
        let entry = allowed_deps.entry(account);
        if let Entry::Vacant(_) = &entry {
            context.charge(CODE_REQUEST_PUBLISH_PER_BYTE * NumBytes::new(32))?;
        }
        context.charge(CODE_REQUEST_PUBLISH_PER_BYTE * NumBytes::new(module_name.len() as u64))?;
        entry.or_default().insert(module_name);
    }

    let mut expected_modules = BTreeSet::new();
    for name in safely_pop_arg!(args, Vec<Value>) {
        let str = get_move_string(name)?;
        context.charge(CODE_REQUEST_PUBLISH_PER_BYTE * NumBytes::new(str.len() as u64))?;
        expected_modules.insert(str);
    }

    let destination = safely_pop_arg!(args, AccountAddress);

    // Gather everything derived from the context before taking the mutable metering borrow.
    let features = context.get_feature_flags().clone();

    // A package may always depend on its own modules.
    allowed_deps
        .entry(destination)
        .or_default()
        .extend(expected_modules.clone());
    // The allowed-dependency check is only enforced when CODE_DEPENDENCY_CHECK is enabled, matching
    // the legacy publish path (request_publish vs request_publish_with_allowed_deps).
    let allowed_deps = features
        .is_enabled(FeatureFlag::CODE_DEPENDENCY_CHECK)
        .then_some(allowed_deps);

    let gas_feature_version = context.gas_feature_version();
    let treat_entry_as_public = context.timed_feature_enabled(TimedFeatureFlag::EntryCompatibility);
    let is_mainnet = {
        let txn_context = context.extensions().get::<NativeTransactionContext>();
        ChainId::new(txn_context.chain_id()).is_mainnet()
    };

    let compatibility = Compatibility::new(
        true,
        !features.is_enabled(FeatureFlag::TREAT_FRIEND_AS_PRIVATE),
        treat_entry_as_public,
        gas_feature_version < gas_feature_versions::RELEASE_V1_34,
        features.is_enabled(FeatureFlag::ALLOW_FRIEND_ENTRY_VISIBILITY_DOWNGRADE),
    );

    let mut modules = Vec::with_capacity(code.len());
    {
        let (module_storage, traversal_context, gas_meter) =
            context.module_storage_wrapper_with_metering();

        // Deserialize the bundle for metadata / resource-group / event validation.
        {
            let deserializer_config = &module_storage
                .runtime_environment()
                .vm_config()
                .deserializer_config;
            for blob in &code {
                let module = CompiledModule::deserialize_with_config(blob, deserializer_config)
                    .map_err(|_| {
                        SafeNativeError::InvariantViolation(PartialVMError::new(
                            StatusCode::CODE_DESERIALIZATION_ERROR,
                        ))
                    })?;
                modules.push(module);
            }
        }

        // Charge dependency gas for the old (upgraded) and new versions of each module, matching
        // the legacy publish path. `charge_dependency` is a no-op for special addresses and older
        // gas feature versions.
        for (module, blob) in modules.iter().zip(code.iter()) {
            let addr = module.self_addr();
            let name = module.self_name();
            let old_size = module_storage
                .unmetered_get_module_size(addr, name)
                .map_err(|err| SafeNativeError::InvariantViolation(err.to_partial()))?;
            if let Some(old_size) = old_size {
                gas_meter
                    .charge_dependency(
                        DependencyKind::Existing,
                        addr,
                        name,
                        NumBytes::new(old_size as u64),
                    )
                    .map_err(SafeNativeError::InvariantViolation)?;
            }
            gas_meter
                .charge_dependency(
                    DependencyKind::New,
                    addr,
                    name,
                    NumBytes::new(blob.len() as u64),
                )
                .map_err(SafeNativeError::InvariantViolation)?;

            // Mark the modules in the bundle as visited so that validation (resource groups, events)
            // and later checks do not treat them as unmetered accesses.
            traversal_context.visit_if_not_special_module_id(&module.self_id());
        }

        // Deferred publishing is only reachable under lazy loading (see `code.move`), which links
        // and charges immediate dependencies rather than the whole transitive closure. This mirrors
        // the lazy branch of the legacy `charge_package_dependencies`.
        let module_ids_in_bundle = modules
            .iter()
            .map(|module| (module.self_addr(), module.self_name()))
            .collect::<BTreeSet<_>>();

        // Charge gas for immediate dependencies that are not part of the bundle.
        for (dep_addr, dep_name) in modules
            .iter()
            .flat_map(|module| module.immediate_dependencies_iter())
            .filter(|addr_and_name| !module_ids_in_bundle.contains(addr_and_name))
        {
            let module_id = ModuleId::new(*dep_addr, dep_name.to_owned());
            if traversal_context.visit_if_not_special_module_id(&module_id) {
                let size = module_storage
                    .unmetered_get_existing_module_size(dep_addr, dep_name)
                    .map_err(|err| SafeNativeError::InvariantViolation(err.to_partial()))?;
                gas_meter
                    .charge_dependency(
                        DependencyKind::Existing,
                        dep_addr,
                        dep_name,
                        NumBytes::new(size as u64),
                    )
                    .map_err(SafeNativeError::InvariantViolation)?;
            }
        }

        // A friend of a published module must belong to the same bundle.
        for (friend_addr, friend_name) in modules
            .iter()
            .flat_map(|module| module.immediate_friends_iter())
        {
            if !module_ids_in_bundle.contains(&(friend_addr, friend_name)) {
                let msg = format!(
                    "Module {}::{} is declared as a friend and should be part of the module \
                     bundle, but it is not",
                    friend_addr, friend_name
                );
                return Err(SafeNativeError::InvariantViolation(
                    PartialVMError::new(StatusCode::FRIEND_NOT_FOUND_IN_MODULE_BUNDLE)
                        .with_message(msg),
                ));
            }
        }

        // Module complexity check (matches the legacy publish path).
        for (module, blob) in modules.iter().zip(code.iter()) {
            // TODO(Gas): Make budget configurable.
            let budget = 2048 + blob.len() as u64 * 20;
            check_module_complexity(module, budget).map_err(SafeNativeError::InvariantViolation)?;
        }

        // Aptos-specific validation: unstable bytecode, native functions, expected modules, allowed
        // dependencies, module metadata, resource groups, and events.
        validate_publish_request(
            &features,
            is_mainnet,
            &module_storage,
            traversal_context,
            gas_meter,
            &modules,
            expected_modules,
            allowed_deps,
        )
        .map_err(|err| SafeNativeError::InvariantViolation(err.to_partial()))?;

        // Bytecode verification, address check, compatibility, and linking against dependencies.
        StagingModuleStorage::create_with_compat_config(
            &destination,
            compatibility,
            &module_storage,
            code.into_iter().map(Bytes::from).collect(),
        )
        .map_err(|err| SafeNativeError::InvariantViolation(err.to_partial()))?;
    }

    // Pre-charge I/O for the module writes that are materialized at the block epilogue. Only the
    // state slot and key are charged here; the value bytes are covered by the queue resource write.
    for module in &modules {
        let state_key = StateKey::module_id(&module.self_id());
        context.charge_io_gas_for_write(NumBytes::new(state_key.size() as u64))?;
    }

    Ok(smallvec![])
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [
        ("request_publish", native_request_publish as RawSafeNative),
        ("request_publish_with_allowed_deps", native_request_publish),
        ("verify_package", native_verify_package),
    ];

    builder.make_named_natives(natives)
}
