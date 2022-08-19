// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::bail;
use aptos_types::transaction::ModuleBundle;
use aptos_types::vm_status::StatusCode;
use better_any::{Tid, TidAble};
use move_deps::move_binary_format::errors::PartialVMError;
use move_deps::move_core_types::gas_algebra::{InternalGas, InternalGasPerByte, NumBytes};
use move_deps::move_vm_types::pop_arg;
use move_deps::move_vm_types::values::Struct;
use move_deps::{
    move_binary_format::errors::PartialVMResult,
    move_core_types::account_address::AccountAddress,
    move_vm_runtime::native_functions::{NativeContext, NativeFunction},
    move_vm_types::{
        loaded_data::runtime_types::Type, natives::function::NativeResult, values::Value,
    },
};
use serde::{Deserialize, Deserializer, Serialize};
use smallvec::smallvec;
use std::collections::{BTreeSet, VecDeque};
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

/// The package registry at the given address.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct PackageRegistry {
    /// Packages installed at this address.
    pub packages: Vec<PackageMetadata>,
}

/// The PackageMetadata type. All blobs are encoded as base64-gzipped.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct PackageMetadata {
    /// Name of this package.
    pub name: String,
    /// The upgrade policy of this package.
    pub upgrade_policy: UpgradePolicy,
    /// The numbers of times this module has been upgraded. Also serves as the on-chain version.
    /// This field will be automatically assigned on successful upgrade.
    pub upgrade_number: u64,
    /// Build info, in BuildInfo.yaml format
    pub build_info: String,
    /// The package manifest, in the Move.toml format.
    pub manifest: String,
    /// The list of modules installed by this package.
    pub modules: Vec<ModuleMetadata>,
    /// ABIs, in compressed BCS
    pub abis: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModuleMetadata {
    /// Name of the module.
    pub name: String,
    /// Source text if available, in compressed form.
    pub source: String,
    /// Source map, in BCS encoding, in compressed form.
    pub source_map: String,
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
// Duplication for JSON

// For JSON we need attributes on fields which aren't compatible with BCS, therefore we
// need to duplicate the definitions...

fn deserialize_from_string<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Display,
{
    use serde::de::Error;

    let s = <String>::deserialize(deserializer)?;
    s.parse::<T>().map_err(D::Error::custom)
}

/// The package registry at the given address.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct PackageRegistryJson {
    pub packages: Vec<PackageMetadataJson>,
}

/// The PackageMetadata type, with an annotation on `upgrade_number`.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct PackageMetadataJson {
    pub name: String,
    pub upgrade_policy: UpgradePolicy,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub upgrade_number: u64,
    pub build_info: String,
    pub manifest: String,
    pub modules: Vec<ModuleMetadata>,
    pub abis: Vec<String>,
}

// ========================================================================================
// Code Publishing Logic

/// Abort code when code publishing is requested twice (0x03 == INVALID_STATE)
const EALREADY_REQUESTED: u64 = 0x03_0000;

const CHECK_COMPAT_POLICY: u8 = 1;

/// The native code context.
#[derive(Tid, Default)]
pub struct NativeCodeContext {
    /// Remembers whether the publishing of a module bundle was requested during transaction
    /// execution.
    pub requested_module_bundle: Option<PublishRequest>,
}

/// Represents a request for code publishing made from a native call and to be processed
/// by the Aptos VM.
pub struct PublishRequest {
    pub destination: AccountAddress,
    pub bundle: ModuleBundle,
    pub expected_modules: BTreeSet<String>,
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

/***************************************************************************************************
 * native fun request_publish(
 *     destination: address,
 *     expected_modules: vector<String>,
 *     code: vector<vector<u8>>,
 *     policy: u8
 * )
 *
 *   gas cost: base_cost + unit_cost * bytes_len
 *
 **************************************************************************************************/
#[derive(Clone, Debug)]
pub struct RequestPublishGasParameters {
    pub base: InternalGas,
    pub per_byte: InternalGasPerByte,
}

fn native_request_publish(
    gas_params: &RequestPublishGasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert_eq!(args.len(), 4);

    let policy = pop_arg!(args, u8);
    let mut code = vec![];
    for module in pop_arg!(args, Vec<Value>) {
        code.push(module.value_as::<Vec<u8>>()?);
    }

    let mut expected_modules = BTreeSet::new();
    for name in pop_arg!(args, Vec<Value>) {
        expected_modules.insert(get_move_string(name)?);
    }

    // TODO(Gas): fine tune the gas formula
    let cost = gas_params.base
        + gas_params.per_byte
            * code.iter().fold(NumBytes::new(0), |acc, module_code| {
                acc + NumBytes::new(module_code.len() as u64)
            })
        + gas_params.per_byte
            * expected_modules.iter().fold(NumBytes::new(0), |acc, name| {
                acc + NumBytes::new(name.len() as u64)
            });

    let destination = pop_arg!(args, AccountAddress);
    let code_context = context.extensions_mut().get_mut::<NativeCodeContext>();
    if code_context.requested_module_bundle.is_some() {
        // Can't request second time.
        return Ok(NativeResult::err(cost, EALREADY_REQUESTED));
    }
    code_context.requested_module_bundle = Some(PublishRequest {
        destination,
        bundle: ModuleBundle::new(code),
        expected_modules,
        check_compat: policy == CHECK_COMPAT_POLICY,
    });
    // TODO(Gas): charge gas for requesting code load (charge for actual code loading done elsewhere)
    Ok(NativeResult::ok(cost, smallvec![]))
}

pub fn make_native_request_publish(gas_params: RequestPublishGasParameters) -> NativeFunction {
    Arc::new(move |context, ty_args, args| {
        native_request_publish(&gas_params, context, ty_args, args)
    })
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct GasParameters {
    pub request_publish: RequestPublishGasParameters,
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [(
        "request_publish",
        make_native_request_publish(gas_params.request_publish),
    )];

    crate::natives::helpers::make_module_natives(natives)
}
