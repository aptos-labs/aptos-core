// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Records how the Move VM and MonoVM run a set of calls, for the Lean
//! specification's differential tests (plan §5.2, "Validation").
//!
//! ```text
//! move-bytecode-lean-vm-oracle SPEC.json [--output FILE]
//! ```
//!
//! `SPEC.json` names `.mv`/`.masm` inputs, relative to the spec file, and the
//! calls to make:
//!
//! ```text
//! {"inputs": ["a.masm", ...],
//!  "calls": [{"function": "0x66::m::f", "args": [<value>, ...]}, ...]}
//! ```
//!
//! A spec may also list constants, each a type and its serialized bytes:
//!
//! ```text
//! {"inputs": [], "calls": [], "constants": [{"type": "vector<u8>", "data": [1, 7]}, ...]}
//! ```
//!
//! Each becomes a module `0xcc::constant_<i>` whose constant pool holds just
//! that constant, so that the production constant decoder's decision on it is
//! recorded (deliverable A4).
//!
//! A spec may also name release bundles (`.mrb`), relative to the spec file,
//! whose modules come first, in bundle order (`scripts/check-framework.sh`):
//!
//! ```text
//! {"bundles": ["head.mrb"], "inputs": [], "calls": []}
//! ```
//!
//! Values use the schema of `mono-move-lean-link` (`payload::Value`). The
//! output is the exporter's document plus, for each module, the decision of
//! the Rust bytecode verifier under the recorded production configuration,
//! and, for each call, the normalized outcome on each VM:
//!
//! ```text
//! {"format": "move-bytecode-lean-diff/1", "modules": [...],
//!  "rust_verifier": [{"module": "0x66::m", "local": "ok", "linking": "ok"}, ...],
//!  "calls": [{"function": ..., "args": [...], "legacy": <outcome>, "monovm": <outcome>}]}
//! ```
//!
//! Outcomes keep what is comparable and drop what is not (gas, messages,
//! locations): `returned` with values, `aborted` with the code, `failed` with
//! the status (the legacy VM's `StatusCode`, MonoVM's error kind),
//! `exhausted`, or `error` for anything that is not an outcome of the program.

use anyhow::{bail, Context, Result};
use aptos_release_bundle::ReleaseBundle;
use mono_move_lean_link::{
    engine::run_modules,
    marshal::encode_bcs,
    payload::{Call, Limits, Outcome as MonoOutcome, Value},
};
use move_binary_format::{
    errors::{VMError, VMResult},
    file_format::{Constant, SignatureToken},
    CompiledModule,
};
use move_bytecode_verifier::{dependencies, verify_module_with_config, VerifierConfig};
use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    language_storage::ModuleId,
    value::MoveValue,
    vm_status::{StatusCode, StatusType},
};
use move_vm_runtime::{
    data_cache::{MoveVmDataCacheAdapter, TransactionDataCache},
    dispatch_loader,
    module_traversal::{TraversalContext, TraversalStorage},
    move_vm::{MoveVM, SerializedReturnValues},
    native_extensions::NativeContextExtensions,
    AsUnsyncModuleStorage, InstantiatedFunctionLoader, LegacyLoaderConfig, ModuleStorage,
};
use move_vm_test_utils::InMemoryStorage;
use move_vm_types::{gas::UnmeteredGasMeter, resolver::ResourceResolver};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{fs, path::Path};

const FORMAT: &str = "move-bytecode-lean-diff/1";

/// MonoVM gas per call: a termination bound, never a compared quantity.
const MONOVM_GAS: u64 = 100_000_000;

#[derive(Deserialize)]
struct Spec {
    #[serde(default)]
    bundles: Vec<String>,
    inputs: Vec<String>,
    calls: Vec<Call>,
    #[serde(default)]
    constants: Vec<ConstantCase>,
}

/// A constant to decode: its type, as in `vector<u8>`, and its bytes.
#[derive(Deserialize)]
struct ConstantCase {
    #[serde(rename = "type")]
    type_: String,
    data: Vec<u8>,
}

/// A constant's type token from its name: a primitive type, `signer`, or a
/// vector of one.
fn constant_type(text: &str) -> Result<SignatureToken> {
    use SignatureToken as T;
    Ok(match text {
        "bool" => T::Bool,
        "u8" => T::U8,
        "u16" => T::U16,
        "u32" => T::U32,
        "u64" => T::U64,
        "u128" => T::U128,
        "u256" => T::U256,
        "i8" => T::I8,
        "i16" => T::I16,
        "i32" => T::I32,
        "i64" => T::I64,
        "i128" => T::I128,
        "i256" => T::I256,
        "address" => T::Address,
        "signer" => T::Signer,
        _ => match text
            .strip_prefix("vector<")
            .and_then(|r| r.strip_suffix('>'))
        {
            Some(elem) => T::Vector(Box::new(constant_type(elem)?)),
            None => bail!("unsupported constant type {}", text),
        },
    })
}

/// A module whose constant pool holds just the given constant.
fn constant_module(index: usize, case: &ConstantCase) -> Result<CompiledModule> {
    let name = format!("constant_{}", index);
    let source = format!(
        "module 0xcc::{}\n\nfun f(): u64\n    ld_u64 1\n    ret\n",
        name
    );
    let mut module = move_bytecode_lean_export::assemble_module(&name, &source, &[])?;
    module.constant_pool.push(Constant {
        type_: constant_type(&case.type_)?,
        data: case.data.clone(),
    });
    Ok(module)
}

/// A normalized outcome.
#[derive(Serialize)]
#[serde(tag = "kind")]
enum VmOutcome {
    #[serde(rename = "returned")]
    Returned { values: Vec<Value> },
    #[serde(rename = "aborted")]
    Aborted { code: String },
    #[serde(rename = "failed")]
    Failed { status: String },
    #[serde(rename = "exhausted")]
    Exhausted { resource: String },
    #[serde(rename = "error")]
    Error { status: String },
}

fn integer(width: u16, signed: bool, value: String) -> Value {
    Value::Integer {
        width,
        signed,
        value,
    }
}

fn move_value(v: MoveValue) -> Result<Value, String> {
    Ok(match v {
        MoveValue::Bool(value) => Value::Bool { value },
        MoveValue::U8(x) => integer(8, false, x.to_string()),
        MoveValue::U16(x) => integer(16, false, x.to_string()),
        MoveValue::U32(x) => integer(32, false, x.to_string()),
        MoveValue::U64(x) => integer(64, false, x.to_string()),
        MoveValue::U128(x) => integer(128, false, x.to_string()),
        MoveValue::U256(x) => integer(256, false, x.to_string()),
        MoveValue::I8(x) => integer(8, true, x.to_string()),
        MoveValue::I16(x) => integer(16, true, x.to_string()),
        MoveValue::I32(x) => integer(32, true, x.to_string()),
        MoveValue::I64(x) => integer(64, true, x.to_string()),
        MoveValue::I128(x) => integer(128, true, x.to_string()),
        MoveValue::I256(x) => integer(256, true, x.to_string()),
        MoveValue::Address(a) => Value::Address {
            value: a.to_hex_literal(),
        },
        MoveValue::Vector(elements) => Value::Vector {
            elements: elements
                .into_iter()
                .map(move_value)
                .collect::<Result<_, _>>()?,
        },
        MoveValue::Struct(_) | MoveValue::Signer(_) | MoveValue::Closure(_) => {
            return Err("unsupported return value".to_string())
        },
    })
}

fn parse_function(text: &str) -> Option<(AccountAddress, Identifier, Identifier)> {
    let mut parts = text.split("::");
    let address = AccountAddress::from_hex_literal(parts.next()?).ok()?;
    let module = Identifier::new(parts.next()?).ok()?;
    let function = Identifier::new(parts.next()?).ok()?;
    match parts.next() {
        None => Some((address, module, function)),
        Some(_) => None,
    }
}

fn execute(
    data_storage: &impl ResourceResolver,
    module_storage: &impl ModuleStorage,
    module_id: &ModuleId,
    function_name: &IdentStr,
    args: Vec<Vec<u8>>,
) -> VMResult<SerializedReturnValues> {
    let mut data_cache = TransactionDataCache::empty();
    let mut gas_meter = UnmeteredGasMeter;
    let traversal_storage = TraversalStorage::new();
    let mut traversal_context = TraversalContext::new(&traversal_storage);
    dispatch_loader!(module_storage, loader, {
        let func = loader.load_instantiated_function(
            &LegacyLoaderConfig::unmetered(),
            &mut gas_meter,
            &mut traversal_context,
            module_id,
            function_name,
            &[],
        )?;
        MoveVM::execute_loaded_function(
            func,
            args,
            &mut MoveVmDataCacheAdapter::new(&mut data_cache, data_storage, &loader),
            &mut gas_meter,
            &mut traversal_context,
            &mut NativeContextExtensions::default(),
            &loader,
        )
    })
}

fn legacy_error(error: &VMError) -> VmOutcome {
    let status = error.major_status();
    if status == StatusCode::ABORTED {
        return VmOutcome::Aborted {
            code: error.sub_status().unwrap_or(0).to_string(),
        };
    }
    let name = format!("{:?}", status);
    match status.status_type() {
        StatusType::Execution => VmOutcome::Failed { status: name },
        StatusType::Validation
        | StatusType::Verification
        | StatusType::InvariantViolation
        | StatusType::Deserialization
        | StatusType::Unknown => VmOutcome::Error { status: name },
    }
}

fn run_legacy(modules: &[CompiledModule], call: &Call) -> VmOutcome {
    let error = |status: &str| VmOutcome::Error {
        status: status.to_string(),
    };
    if !call.signers.is_empty() {
        return error("signers are not supported");
    }
    let Some((address, module, function)) = parse_function(&call.function) else {
        return error("malformed function reference");
    };
    let args = match call
        .args
        .iter()
        .map(encode_bcs)
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(args) => args,
        Err(message) => return error(&message),
    };
    let mut storage = InMemoryStorage::new();
    for m in modules {
        let mut blob = vec![];
        if m.serialize(&mut blob).is_err() {
            return error("module serialization failed");
        }
        storage.add_module_bytes(m.self_addr(), m.self_name(), blob.into());
    }
    let module_storage = storage.as_unsync_module_storage();
    let module_id = ModuleId::new(address, module);
    match execute(&storage, &module_storage, &module_id, &function, args) {
        Ok(returned) => {
            let values = returned
                .return_values
                .into_iter()
                .map(|(bytes, layout)| {
                    MoveValue::simple_deserialize(&bytes, &layout)
                        .map_err(|e| format!("{e}"))
                        .and_then(move_value)
                })
                .collect::<Result<Vec<_>, _>>();
            match values {
                Ok(values) => VmOutcome::Returned { values },
                Err(message) => error(&message),
            }
        },
        Err(e) => legacy_error(&e),
    }
}

fn monovm(outcome: MonoOutcome) -> VmOutcome {
    match outcome {
        MonoOutcome::Returned { values, .. } => VmOutcome::Returned { values },
        MonoOutcome::Aborted { code, .. } => VmOutcome::Aborted {
            code: code.to_string(),
        },
        MonoOutcome::Exhausted { resource } => VmOutcome::Exhausted {
            resource: format!("{:?}", resource),
        },
        MonoOutcome::Failed { failure, .. } => VmOutcome::Failed { status: failure },
        MonoOutcome::Error { stage, .. } => VmOutcome::Error {
            status: format!("{:?}", stage),
        },
    }
}

fn status_of(result: VMResult<()>) -> String {
    match result {
        Ok(()) => "ok".to_string(),
        Err(e) => format!("{:?}", e.major_status()),
    }
}

/// The Rust verifier's decision on each module: local verification, then
/// link checks against the program's other modules.
fn rust_verifier(config: &VerifierConfig, modules: &[CompiledModule]) -> Vec<serde_json::Value> {
    modules
        .iter()
        .map(|m| {
            let others = modules.iter().filter(|d| d.self_id() != m.self_id());
            json!({
                "module": format!("{}::{}", m.self_addr().to_hex_literal(), m.self_name()),
                "local": status_of(verify_module_with_config(config, m)),
                "linking": status_of(dependencies::verify_module(config, m, others)),
            })
        })
        .collect()
}

/// The modules of a release bundle, in the bundle's order.
fn bundle_modules(path: &Path) -> Result<Vec<CompiledModule>> {
    let bundle = ReleaseBundle::read(path.to_path_buf())?;
    bundle
        .code()
        .into_iter()
        .enumerate()
        .map(|(index, bytes)| {
            CompiledModule::deserialize(bytes)
                .map_err(|e| anyhow::anyhow!("{}: module {}: {:?}", path.display(), index, e))
        })
        .collect()
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let mut output: Option<String> = None;
    let mut spec_path: Option<String> = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--output" | "-o" => output = Some(args.next().context("--output needs a path")?),
            _ if spec_path.is_none() => spec_path = Some(arg),
            _ => bail!("usage: move-bytecode-lean-vm-oracle SPEC.json [--output FILE]"),
        }
    }
    let spec_path = spec_path.context("usage: move-bytecode-lean-vm-oracle SPEC.json")?;
    let spec: Spec = serde_json::from_str(
        &fs::read_to_string(&spec_path).with_context(|| format!("reading {}", spec_path))?,
    )
    .with_context(|| format!("parsing {}", spec_path))?;
    let base = Path::new(&spec_path).parent().unwrap_or(Path::new("."));
    let inputs: Vec<String> = spec
        .inputs
        .iter()
        .map(|input| base.join(input).to_string_lossy().into_owned())
        .collect();
    let mut modules = vec![];
    for bundle in &spec.bundles {
        modules.extend(bundle_modules(&base.join(bundle))?);
    }
    let (input_modules, scripts) = move_bytecode_lean_export::load_inputs(&inputs)?;
    modules.extend(input_modules);
    if !scripts.is_empty() {
        bail!("scripts are not supported yet");
    }
    for (index, case) in spec.constants.iter().enumerate() {
        modules.push(constant_module(index, case)?);
    }

    let limits = Limits {
        gas: MONOVM_GAS,
        heap: None,
    };
    let mono_outcomes = if spec.calls.is_empty() {
        vec![]
    } else {
        run_modules(&modules, &limits, &spec.calls).map_err(anyhow::Error::msg)?
    };
    let calls: Vec<serde_json::Value> = spec
        .calls
        .iter()
        .zip(mono_outcomes)
        .map(|(call, mono)| {
            json!({
                "function": call.function,
                "args": call.args,
                "legacy": run_legacy(&modules, call),
                "monovm": monovm(mono),
            })
        })
        .collect();

    let mut document = move_bytecode_lean_export::document(&modules, &[]);
    document["format"] = json!(FORMAT);
    document["rust_verifier"] = json!(rust_verifier(
        &move_bytecode_lean_config_export::production_config(),
        &modules
    ));
    document["calls"] = json!(calls);
    move_bytecode_lean_export::write_output(output, document)
}
