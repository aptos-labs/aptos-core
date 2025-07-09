#![no_main]

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_language_e2e_tests::{account::Account, executor::FakeExecutor};
use aptos_transaction_simulation::GENESIS_CHANGE_SET_HEAD;
use aptos_types::{
    chain_id::ChainId,
    on_chain_config::Features,
    transaction::{
        EntryFunction, ExecutionStatus, Script, SignedTransaction, TransactionArgument,
        TransactionPayload, TransactionStatus,
    },
    write_set::WriteSet,
};
use aptos_vm::AptosVM;
use aptos_vm_environment::prod_configs;
use libfuzzer_sys::{fuzz_target, Corpus};
use move_binary_format::{
    access::ModuleAccess,
    deserializer::DeserializerConfig,
    errors::VMError,
    file_format::{CompiledModule, CompiledScript, SignatureToken},
};
use move_core_types::vm_status::{StatusCode, StatusType};
use move_transactional_test_runner::transactional_ops::TransactionalOperation;
use once_cell::sync::Lazy;
use std::{
    collections::{BTreeMap, HashSet},
    sync::Arc,
};
mod utils;
use fuzzer::{ExecVariant, RunnableStateWithOperations};
use utils::vm::{check_for_invariant_violation, publish_group, sort_by_deps, BYTECODE_VERSION};

// genesis write set generated once for each fuzzing session
static VM_WRITE_SET: Lazy<WriteSet> = Lazy::new(|| GENESIS_CHANGE_SET_HEAD.write_set().clone());

const FUZZER_CONCURRENCY_LEVEL: usize = 1;
static TP: Lazy<Arc<rayon::ThreadPool>> = Lazy::new(|| {
    Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(FUZZER_CONCURRENCY_LEVEL)
            .build()
            .unwrap(),
    )
});

const MAX_TYPE_PARAMETER_VALUE: u16 = 64 / 4 * 16; // third_party/move/move-bytecode-verifier/src/signature_v2.rs#L1306-L1312

// List of known false positive messages for invariant violations
// If some invariant violation do not come with a message, we need to attach a message to it at throwing site.
const KNOWN_FALSE_POSITIVES: &[&str] = &["too many type parameters/arguments in the program"];

#[inline(always)]
fn is_coverage_enabled() -> bool {
    cfg!(coverage_enabled) || std::env::var("LLVM_PROFILE_FILE").is_ok()
}

fn check_for_invariant_violation_vmerror(e: VMError) {
    if e.status_type() == StatusType::InvariantViolation {
        let is_known_false_positive = e.message().map_or(false, |msg| {
            KNOWN_FALSE_POSITIVES
                .iter()
                .any(|known| msg.starts_with(known))
        });

        if !is_known_false_positive && e.status_type() == StatusType::InvariantViolation {
            panic!(
                "invariant violation {:?}\n{}{:?} {}",
                e,
                "RUST_BACKTRACE=1 DEBUG_VM_STATUS=",
                e.major_status(),
                "./fuzz.sh run move_aptosvm_publish_and_run <ARTIFACT>"
            );
        }
    }
}

// filter modules
fn filter_modules(input: &RunnableStateWithOperations) -> Result<(), Corpus> {
    // reject any TypeParameter exceeds the maximum allowed value (Avoid known Ivariant Violation)
    for operation in input.operations.iter() {
        match operation {
            TransactionalOperation::PublishModule { _module } => {
                for signature in _module.signatures.iter() {
                    for sign_token in signature.0.iter() {
                        if let SignatureToken::TypeParameter(idx) = sign_token {
                            if *idx > MAX_TYPE_PARAMETER_VALUE {
                                return Err(Corpus::Reject);
                            }
                        }
                    }
                }
            },
            TransactionalOperation::RunScript { _script, .. } => {
                for signature in _script.signatures.iter() {
                    for sign_token in signature.0.iter() {
                        if let SignatureToken::TypeParameter(idx) = sign_token {
                            if *idx > MAX_TYPE_PARAMETER_VALUE {
                                return Err(Corpus::Reject);
                            }
                        }
                    }
                }
            },
            _ => (),
        }
    }
    Ok(())
}

#[allow(clippy::literal_string_with_formatting_args)]
fn run_case(input: RunnableStateWithOperations) -> Result<(), Corpus> {
    tdbg!(&input);

    // filter modules
    tdbg!("filtering modules");
    filter_modules(&input)?;

    let verifier_config = prod_configs::aptos_prod_verifier_config(&Features::default());
    let deserializer_config = DeserializerConfig::new(BYTECODE_VERSION, 255);

    let mut dep_modules: Vec<CompiledModule> = vec![];
    let mut exec_variant_opt: Option<ExecVariant> = None;

    for operation in input.operations.iter() {
        match operation {
            TransactionalOperation::PublishModule { _module } => {
                dep_modules.push(_module.clone());
            },
            TransactionalOperation::RunScript {
                _script,
                _type_args,
                _args,
            } => {
                if exec_variant_opt.is_none() {
                    exec_variant_opt = Some(ExecVariant::Script {
                        _script: _script.clone(),
                        _type_args: _type_args.clone(),
                        _args: _args.clone(),
                    });
                }
            },
            TransactionalOperation::CallFunction {
                _module,
                _function,
                _type_args,
                _args,
            } => {
                if exec_variant_opt.is_none() {
                    exec_variant_opt = Some(ExecVariant::CallFunction {
                        _module: _module.clone(),
                        _function: *_function,
                        _type_args: _type_args.clone(),
                        _args: _args.clone(),
                    });
                }
            },
        }
    }

    tdbg!("verifying scripts");
    for exec_variant in exec_variant_opt.iter() {
        match exec_variant {
            // reject bad scripts fast
            ExecVariant::Script {
                _script,
                _type_args: _,
                _args: _,
            } => {
                let mut script_code: Vec<u8> = vec![];
                tdbg!("serializing script");
                _script
                    .serialize_for_version(Some(BYTECODE_VERSION), &mut script_code)
                    .map_err(|_| Corpus::Reject)?;
                tdbg!("deserializing script");
                let s_de =
                    CompiledScript::deserialize_with_config(&script_code, &deserializer_config)
                        .map_err(|_| Corpus::Reject)?;
                tdbg!("verifying script");
                move_bytecode_verifier::verify_script_with_config(&verifier_config, &s_de).map_err(
                    |e| {
                        check_for_invariant_violation_vmerror(e);
                        Corpus::Reject
                    },
                )?
            },
            _ => (),
        }
    }

    tdbg!("verifying modules");
    for m in dep_modules.iter_mut() {
        // m.metadata = vec![]; // we could optimize metadata to only contain aptos metadata
        // m.version = VERSION_MAX;

        // reject bad modules fast
        let mut module_code: Vec<u8> = vec![];
        m.serialize_for_version(Some(BYTECODE_VERSION), &mut module_code)
            .map_err(|_| Corpus::Reject)?;
        let m_de = CompiledModule::deserialize_with_config(&module_code, &deserializer_config)
            .map_err(|_| Corpus::Reject)?;
        move_bytecode_verifier::verify_module_with_config(&verifier_config, &m_de).map_err(|e| {
            check_for_invariant_violation_vmerror(e);
            Corpus::Reject
        })?
    }

    tdbg!("checking no duplicates");
    // check no duplicates
    let mset: HashSet<_> = dep_modules.iter().map(|m| m.self_id()).collect();
    if mset.len() != dep_modules.len() {
        return Err(Corpus::Reject);
    }

    tdbg!("topologically ordering modules");
    // topologically order modules
    let all_modules = dep_modules.clone();
    let map = all_modules
        .into_iter()
        .map(|m| (m.self_id(), m))
        .collect::<BTreeMap<_, _>>();
    let mut order = vec![];
    for id in map.keys() {
        let mut visited = HashSet::new();
        sort_by_deps(&map, &mut order, id.clone(), &mut visited)?;
    }

    tdbg!("grouping same address modules in packages");
    // group same address modules in packages. keep local ordering.
    let mut packages: Vec<Vec<CompiledModule>> = Vec::new();
    let mut remaining_modules_map = map.clone(); // Clone the map as we'll be removing items

    for module_id_to_start_package in &order {
        // `order` is the globally sorted Vec<ModuleId>
        // If the module that could start a new package isn't in our remaining set,
        // it means its entire package has likely been processed already via an earlier module_id in `order`.
        if !remaining_modules_map.contains_key(module_id_to_start_package) {
            continue;
        }

        let package_address = module_id_to_start_package.address();
        let mut current_package_for_address: Vec<CompiledModule> = Vec::new();

        // Iterate through the globally sorted `order` list again.
        // This ensures that modules for `package_address` are added to `current_package_for_address`
        // in their correct topological sub-order.
        for module_id_in_global_order in &order {
            if module_id_in_global_order.address() == package_address {
                // If this module belongs to the current package_address,
                // try to remove it from `remaining_modules_map`.
                // If successful, it means it hasn't been added to any package yet.
                if let Some(module) = remaining_modules_map.remove(module_id_in_global_order) {
                    current_package_for_address.push(module);
                }
            }
        }

        if !current_package_for_address.is_empty() {
            packages.push(current_package_for_address);
        }
    }

    AptosVM::set_concurrency_level_once(FUZZER_CONCURRENCY_LEVEL);
    let mut vm = FakeExecutor::from_genesis_with_existing_thread_pool(
        &VM_WRITE_SET,
        ChainId::mainnet(),
        Arc::clone(&TP),
    )
    .set_not_parallel();

    // publish all packages
    for group in packages {
        let sender = *group[0].address();
        let acc = vm.new_account_at(sender);
        publish_group(&mut vm, &acc, &group, 0)?;
    }

    // TODO: use the sender from the input when added in future
    let sender_acc = if true {
        // create sender pub/priv key. initialize and fund account
        vm.create_accounts(1, input.tx_auth_type.sender().fund_amount(), 0)
            .remove(0)
    } else {
        // only create sender pub/priv key. do not initialize
        Account::new()
    };

    // build txs
    tdbg!("building txs");
    let mut txs = vec![];
    for exec_variant in exec_variant_opt.iter() {
        let tx = match exec_variant {
            ExecVariant::Script {
                _script,
                _type_args,
                _args,
            } => {
                let mut script_bytes = vec![];
                _script
                    .serialize_for_version(Some(BYTECODE_VERSION), &mut script_bytes)
                    .map_err(|_| Corpus::Reject)?;
                sender_acc
                    .transaction()
                    .gas_unit_price(100)
                    .max_gas_amount(1000)
                    .sequence_number(0)
                    .payload(TransactionPayload::Script(Script::new(
                        script_bytes,
                        _type_args.clone(),
                        _args
                            .iter()
                            .map(|x| x.clone().try_into())
                            .collect::<Result<Vec<TransactionArgument>, _>>()
                            .map_err(|_| Corpus::Reject)?,
                    )))
            },
            ExecVariant::CallFunction {
                _module,
                _function,
                _type_args,
                _args,
            } => {
                // convert FunctionDefinitionIndex to function name... {
                let cm = dep_modules
                    .iter()
                    .find(|m| m.self_id() == *_module)
                    .ok_or(Corpus::Reject)?;
                let fhi = cm
                    .function_defs
                    .get(_function.0 as usize)
                    .ok_or(Corpus::Reject)?
                    .function;
                let function_identifier_index = cm
                    .function_handles
                    .get(fhi.0 as usize)
                    .ok_or(Corpus::Reject)?
                    .name;
                let function_name = cm
                    .identifiers
                    .get(function_identifier_index.0 as usize)
                    .ok_or(Corpus::Reject)?
                    .clone();
                // }
                sender_acc
                    .transaction()
                    .gas_unit_price(100)
                    .max_gas_amount(1000)
                    .sequence_number(0)
                    .payload(TransactionPayload::EntryFunction(EntryFunction::new(
                        _module.clone(),
                        function_name,
                        _type_args.clone(),
                        _args.clone(),
                    )))
            },
        };
        txs.push(tx);
    }

    tdbg!("signing txs");
    let txs: Vec<_> = txs
        .into_iter()
        .map(|tx| {
            let raw_tx = tx.raw();
            raw_tx
                .sign(&sender_acc.privkey, sender_acc.pubkey.as_ed25519().unwrap())
                .map_err(|_| Corpus::Reject)
                .map(|signed_internal| signed_internal.into_inner())
        })
        .collect::<Result<Vec<SignedTransaction>, Corpus>>()?;

    // exec tx
    // Note: one tx per block.
    tdbg!("exec start");
    for tx in txs.iter() {
        let res = vm
            .execute_block(vec![tx.clone()])
            .map_err(|e| {
                check_for_invariant_violation(e);
                Corpus::Keep
            })?
            .pop()
            .expect("expect 1 output");

        // if error exit gracefully
        let status = match tdbg!(res.status()) {
            TransactionStatus::Keep(status) => status,
            TransactionStatus::Discard(e) => {
                if e.status_type() == StatusType::InvariantViolation {
                    panic!("invariant violation {:?}", e);
                }
                return Err(Corpus::Keep);
            },
            _ => return Err(Corpus::Keep),
        };
        match tdbg!(status) {
            ExecutionStatus::Success | ExecutionStatus::OutOfGas => {
                vm.apply_write_set(res.write_set())
            },
            ExecutionStatus::MiscellaneousError(e) => {
                if let Some(e) = e {
                    if e.status_type() == StatusType::InvariantViolation
                        && *e != StatusCode::TYPE_RESOLUTION_FAILURE
                        && *e != StatusCode::STORAGE_ERROR
                    {
                        panic!("invariant violation {:?}, {:?}", e, res.auxiliary_data());
                    }
                }
                return Err(Corpus::Keep);
            },
            _ => return Err(Corpus::Keep),
        };
    }
    tdbg!("exec end");

    Ok(())
}

fuzz_target!(|fuzz_data: RunnableStateWithOperations| -> Corpus {
    run_case(fuzz_data).err().unwrap_or(Corpus::Keep)
});
