// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::unzip_metadata_str;
use anyhow::bail;
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use aptos_types::{
    move_any::Any, on_chain_config::OnChainConfig, transaction::ModuleBundle, vm_status::StatusCode,
};
use better_any::{Tid, TidAble};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{account_address::AccountAddress, gas_algebra::NumBytes};
use move_vm_runtime::{
    native_extensions::VersionControlledNativeExtension, native_functions::NativeFunction,
};
use move_vm_types::{
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

/// A wrapper around the representation of a Move Option, which is a vector with 0 or 1 element.
/// TODO: move this elsewhere for reuse?
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct MoveOption<T> {
    pub value: Vec<T>,
}

impl<T> Default for MoveOption<T> {
    fn default() -> Self {
        MoveOption::none()
    }
}

impl<T> MoveOption<T> {
    pub fn none() -> Self {
        Self { value: vec![] }
    }

    pub fn some(x: T) -> Self {
        Self { value: vec![x] }
    }

    pub fn is_none(&self) -> bool {
        self.value.is_empty()
    }

    pub fn is_some(&self) -> bool {
        !self.value.is_empty()
    }
}

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
    pub extension: MoveOption<Any>,
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
    pub extension: MoveOption<Any>,
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

impl VersionControlledNativeExtension for NativeCodeContext {
    fn undo(&mut self) {
        // No-op: nothing to undo.
    }

    fn save(&mut self) {
        // No-op: nothing to save.
    }

    fn update(&mut self, _txn_hash: &[u8; 32], _script_hash: &[u8]) {
        // TODO: double check if we should allow this in user session only.
    }
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
    _ty_args: Vec<Type>,
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
        return Err(SafeNativeError::Abort {
            abort_code: EALREADY_REQUESTED,
        });
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
 * module
 *
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [
        ("request_publish", native_request_publish as RawSafeNative),
        ("request_publish_with_allowed_deps", native_request_publish),
    ];

    builder.make_named_natives(natives)
}
