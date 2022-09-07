// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::natives::any::Any;
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
use serde::{Deserialize, Serialize};
use smallvec::smallvec;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

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
    debug_assert!(matches!(args.len(), 4 | 5));
    let with_allowed_deps = args.len() == 5;

    let policy = pop_arg!(args, u8);
    let mut code = vec![];
    for module in pop_arg!(args, Vec<Value>) {
        code.push(module.value_as::<Vec<u8>>()?);
    }

    let allowed_deps = if with_allowed_deps {
        let mut allowed_deps: BTreeMap<AccountAddress, BTreeSet<String>> = BTreeMap::new();
        for dep in pop_arg!(args, Vec<Value>) {
            let (account, module_name) = unpack_allowed_dep(dep)?;
            allowed_deps.entry(account).or_default().insert(module_name);
        }
        Some(allowed_deps)
    } else {
        None
    };

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
            })
        + gas_params.per_byte
            * allowed_deps.clone().unwrap_or_default().iter().fold(
                NumBytes::new(0),
                |acc, (_, deps)| {
                    acc + NumBytes::new(32)
                        + deps.iter().fold(NumBytes::zero(), |inner_acc, name| {
                            inner_acc + NumBytes::new(name.len() as u64)
                        })
                },
            );

    let destination = pop_arg!(args, AccountAddress);

    // Add own modules to allowed deps
    let allowed_deps = allowed_deps.map(|mut allowed| {
        allowed
            .entry(destination)
            .or_default()
            .extend(expected_modules.clone().into_iter());
        allowed
    });

    let code_context = context.extensions_mut().get_mut::<NativeCodeContext>();
    if code_context.requested_module_bundle.is_some() {
        // Can't request second time.
        return Ok(NativeResult::err(cost, EALREADY_REQUESTED));
    }
    code_context.requested_module_bundle = Some(PublishRequest {
        destination,
        bundle: ModuleBundle::new(code),
        expected_modules,
        allowed_deps,
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
    let natives = [
        (
            "request_publish",
            make_native_request_publish(gas_params.request_publish.clone()),
        ),
        (
            "request_publish_with_allowed_deps",
            make_native_request_publish(gas_params.request_publish),
        ),
    ];

    crate::natives::helpers::make_module_natives(natives)
}
