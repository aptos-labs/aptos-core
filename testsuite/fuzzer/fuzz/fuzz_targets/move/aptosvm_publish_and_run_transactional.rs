#![no_main]

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_block_executor::code_cache_global_manager::AptosModuleCacheManager;
use aptos_language_e2e_tests::{account::Account, executor::FakeExecutor};
use aptos_transaction_simulation::GENESIS_CHANGE_SET_HEAD;
use aptos_types::{
    chain_id::ChainId,
    on_chain_config::{Features, TimedFeaturesBuilder},
    transaction::{
        EntryFunction, ExecutionStatus, Script, SignedTransaction, TransactionArgument,
        TransactionPayload, TransactionStatus,
    },
    write_set::WriteSet,
};
use aptos_vm::AptosVM;
use aptos_vm_environment::{prod_configs, prod_configs::LATEST_GAS_FEATURE_VERSION};
use libfuzzer_sys::{fuzz_target, Corpus};
use move_binary_format::{
    access::ModuleAccess,
    deserializer::DeserializerConfig,
    file_format::{CompiledModule, SignatureToken},
};
use move_core_types::{
    account_address::AccountAddress,
    vm_status::{StatusCode, StatusType},
};
use move_transactional_test_runner::transactional_ops::TransactionalOperation;
use once_cell::sync::Lazy;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
mod utils;
use fuzzer::RunnableStateWithOperations;
use utils::vm::{
    execute_block_or_keep, group_modules_by_address_topo, publish_group,
    select_or_create_block_index, verify_module_fast, verify_script_fast, BYTECODE_VERSION,
};

// genesis write set generated once for each fuzzing session
static VM_WRITE_SET: Lazy<WriteSet> = Lazy::new(|| GENESIS_CHANGE_SET_HEAD.write_set().clone());

const FUZZER_CONCURRENCY_LEVEL: usize = 4;
static TP: Lazy<Arc<rayon::ThreadPool>> = Lazy::new(|| {
    Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(FUZZER_CONCURRENCY_LEVEL)
            .build()
            .unwrap(),
    )
});

const MAX_TYPE_PARAMETER_VALUE: u16 = 64 / 4 * 16; // third_party/move/move-bytecode-verifier/src/signature_v2.rs#L1306-L1312

// filter modules
#[inline(always)]
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

    let timed_features = TimedFeaturesBuilder::enable_all().build();
    let verifier_config = prod_configs::aptos_prod_verifier_config(
        LATEST_GAS_FEATURE_VERSION,
        &Features::default(),
        &timed_features,
    );
    let deserializer_config = DeserializerConfig::new(BYTECODE_VERSION, 255);

    let mut dep_modules: Vec<CompiledModule> = vec![];
    // Collect dependency modules and later we will verify runs individually
    for operation in input.operations.iter() {
        if let TransactionalOperation::PublishModule { _module } = operation {
            dep_modules.push(_module.clone());
        }
    }

    tdbg!("verifying scripts");
    for operation in input.operations.iter() {
        if let TransactionalOperation::RunScript { _script, .. } = operation {
            verify_script_fast(_script, &verifier_config, &deserializer_config)?;
        }
    }

    tdbg!("verifying modules");
    for m in dep_modules.iter_mut() {
        verify_module_fast(m, &verifier_config, &deserializer_config)?;
    }

    tdbg!("checking no duplicates");
    // check no duplicates
    let mset: HashSet<_> = dep_modules.iter().map(|m| m.self_id()).collect();
    if mset.len() != dep_modules.len() {
        return Err(Corpus::Reject);
    }

    tdbg!("topologically ordering and grouping modules");
    let packages = group_modules_by_address_topo(dep_modules.clone())?;

    // Enable runtime reference-safety checks for the Move VM
    // prod_configs::set_paranoid_ref_checks(true);

    let module_cache_manager = AptosModuleCacheManager::new();
    AptosVM::set_concurrency_level_once(FUZZER_CONCURRENCY_LEVEL);
    let mut vm = FakeExecutor::from_genesis_with_existing_thread_pool(
        &VM_WRITE_SET,
        ChainId::mainnet(),
        Arc::clone(&TP),
        Some(module_cache_manager),
    )
    .set_parallel();

    // publish all packages
    for group in packages {
        let sender = *group[0].address();
        let acc = vm.new_account_at(sender);
        publish_group(&mut vm, &acc, &group, 0)?;
    }

    // Build blocks grouped by exec_group and sign with provided signers
    tdbg!("building grouped tx blocks");
    // Helper maps to reuse accounts and sequence numbers per sender
    let mut accounts_by_addr: HashMap<AccountAddress, Account> = HashMap::new();
    let mut next_seq_by_addr: HashMap<AccountAddress, u64> = HashMap::new();
    let mut blocks: Vec<Vec<SignedTransaction>> = Vec::new();
    let mut group_to_block_index: HashMap<u64, usize> = HashMap::new();

    // Fallback sender if no signers provided (rare)
    let mut fallback_sender: Option<Account> = None;

    // Convert operations into blocks
    for operation in input.operations.iter() {
        match operation {
            TransactionalOperation::PublishModule { .. } => (),
            TransactionalOperation::RunScript {
                _script,
                _type_args,
                _args,
                _signers,
                _exec_group,
            } => {
                // Determine block index for this operation
                let block_index = select_or_create_block_index(
                    *_exec_group,
                    &mut blocks,
                    &mut group_to_block_index,
                );

                // Build raw transaction
                let mut script_bytes = vec![];
                _script
                    .serialize_for_version(Some(BYTECODE_VERSION), &mut script_bytes)
                    .map_err(|_| Corpus::Reject)?;

                // Prepare sender and secondary signers
                let (sender_addr, secondary_addrs) = if !_signers.is_empty() {
                    (_signers[0], _signers[1..].to_vec())
                } else {
                    // Create fallback sender lazily
                    let f = fallback_sender.get_or_insert_with(|| {
                        vm.create_accounts(1, input.tx_auth_type.sender().fund_amount(), 0)
                            .remove(0)
                    });
                    (*f.address(), vec![])
                };

                // Ensure accounts exist for sender and secondary signers
                accounts_by_addr
                    .entry(sender_addr)
                    .or_insert_with(|| vm.new_account_at(sender_addr));
                for sec in secondary_addrs.iter() {
                    if !accounts_by_addr.contains_key(sec) {
                        let acc = vm.new_account_at(*sec);
                        accounts_by_addr.insert(*sec, acc);
                    }
                }

                let sender_acc = accounts_by_addr.get(&sender_addr).unwrap();
                let sequence_number = *next_seq_by_addr.get(&sender_addr).unwrap_or(&0u64);

                let tx_builder = sender_acc
                    .transaction()
                    .gas_unit_price(100)
                    .max_gas_amount(1000)
                    .sequence_number(sequence_number)
                    .payload(TransactionPayload::Script(Script::new(
                        script_bytes,
                        _type_args.clone(),
                        _args
                            .iter()
                            .map(|x| x.clone().try_into())
                            .collect::<Result<Vec<TransactionArgument>, _>>()
                            .map_err(|_| Corpus::Reject)?,
                    )));

                let raw_tx = tx_builder.raw();

                // Sign transaction: single or multi-agent
                let signed_tx = utils::vm::sign_single_or_multi(
                    raw_tx,
                    sender_acc,
                    &secondary_addrs,
                    &accounts_by_addr,
                )?;

                // Increment sender sequence
                next_seq_by_addr.insert(sender_addr, sequence_number + 1);

                blocks[block_index].push(signed_tx);
            },
            TransactionalOperation::CallFunction {
                _module,
                _function,
                _type_args,
                _args,
                _signers,
                _exec_group,
            } => {
                // Determine block index for this operation
                let block_index = select_or_create_block_index(
                    *_exec_group,
                    &mut blocks,
                    &mut group_to_block_index,
                );

                // Resolve function name from index
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

                // Prepare sender and secondary signers
                let (sender_addr, secondary_addrs) = if !_signers.is_empty() {
                    (_signers[0], _signers[1..].to_vec())
                } else {
                    // Create fallback sender lazily
                    let f = fallback_sender.get_or_insert_with(|| {
                        vm.create_accounts(1, input.tx_auth_type.sender().fund_amount(), 0)
                            .remove(0)
                    });
                    (*f.address(), vec![])
                };

                // Ensure accounts exist for sender and secondary signers
                accounts_by_addr
                    .entry(sender_addr)
                    .or_insert_with(|| vm.new_account_at(sender_addr));
                for sec in secondary_addrs.iter() {
                    if !accounts_by_addr.contains_key(sec) {
                        let acc = vm.new_account_at(*sec);
                        accounts_by_addr.insert(*sec, acc);
                    }
                }

                let sender_acc = accounts_by_addr.get(&sender_addr).unwrap();
                let sequence_number = *next_seq_by_addr.get(&sender_addr).unwrap_or(&0u64);

                let tx_builder = sender_acc
                    .transaction()
                    .gas_unit_price(100)
                    .max_gas_amount(1000)
                    .sequence_number(sequence_number)
                    .payload(TransactionPayload::EntryFunction(EntryFunction::new(
                        _module.clone(),
                        function_name,
                        _type_args.clone(),
                        _args.clone(),
                    )));

                let raw_tx = tx_builder.raw();

                // Sign transaction: single or multi-agent
                let signed_tx = utils::vm::sign_single_or_multi(
                    raw_tx,
                    sender_acc,
                    &secondary_addrs,
                    &accounts_by_addr,
                )?;

                // Increment sender sequence
                next_seq_by_addr.insert(sender_addr, sequence_number + 1);

                blocks[block_index].push(signed_tx);
            },
        }
    }

    // Execute blocks
    tdbg!("exec start");
    for block in blocks.into_iter() {
        if block.is_empty() {
            continue;
        }
        let outputs = execute_block_or_keep(&vm, block)?;

        // Check all transaction outputs and apply write sets on success
        for res in outputs.into_iter() {
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
    }
    tdbg!("exec end");

    Ok(())
}

fuzz_target!(|fuzz_data: RunnableStateWithOperations| -> Corpus {
    run_case(fuzz_data).err().unwrap_or(Corpus::Keep)
});
