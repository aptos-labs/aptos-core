#![no_main]

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_language_e2e_tests::{account::Account, executor::FakeExecutor};
use aptos_transaction_simulation::GENESIS_CHANGE_SET_HEAD;
use aptos_types::{
    chain_id::ChainId,
    on_chain_config::Features,
    transaction::{
        EntryFunction, ExecutionStatus, Script, TransactionArgument, TransactionPayload,
        TransactionStatus,
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
use once_cell::sync::Lazy;
use std::{
    collections::{BTreeMap, HashSet},
    sync::Arc,
    time::Instant,
};
mod utils;
use utils::vm::{
    check_for_invariant_violation, publish_group, sort_by_deps, ExecVariant,
    FuzzerRunnableAuthenticator, RunnableState,
};

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

const EXECUTION_TIME_GAS_RATIO: u8 = 50;

#[inline(always)]
fn is_coverage_enabled() -> bool {
    cfg!(coverage_enabled) || std::env::var("LLVM_PROFILE_FILE").is_ok()
}

fn check_for_invariant_violation_vmerror(e: VMError) {
    if e.status_type() == StatusType::InvariantViolation
        // ignore known false positive
        && !e
            .message()
            .is_some_and(|m| m.starts_with("too many type parameters/arguments in the program"))
    {
        panic!("invariant violation {:?}", e);
    }
}

// filter modules
fn filter_modules(input: &RunnableState) -> Result<(), Corpus> {
    // reject any TypeParameter exceeds the maximum allowed value (Avoid known Ivariant Violation)
    if let ExecVariant::Script { script, .. } = input.exec_variant.clone() {
        for signature in script.signatures {
            for sign_token in signature.0.iter() {
                if let SignatureToken::TypeParameter(idx) = sign_token {
                    if *idx > MAX_TYPE_PARAMETER_VALUE {
                        return Err(Corpus::Reject);
                    }
                } else if let SignatureToken::Vector(inner) = sign_token {
                    if let SignatureToken::TypeParameter(idx) = inner.as_ref() {
                        if *idx > MAX_TYPE_PARAMETER_VALUE {
                            return Err(Corpus::Reject);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn run_case(mut input: RunnableState) -> Result<(), Corpus> {
    tdbg!(&input);

    // filter modules
    filter_modules(&input)?;

    let verifier_config = prod_configs::aptos_prod_verifier_config(&Features::default());
    let deserializer_config = DeserializerConfig::new(8, 255);

    for m in input.dep_modules.iter_mut() {
        // m.metadata = vec![]; // we could optimize metadata to only contain aptos metadata
        // m.version = VERSION_MAX;

        // reject bad modules fast
        let mut module_code: Vec<u8> = vec![];
        m.serialize(&mut module_code).map_err(|_| Corpus::Keep)?;
        let m_de = CompiledModule::deserialize_with_config(&module_code, &deserializer_config)
            .map_err(|_| Corpus::Reject)?;
        move_bytecode_verifier::verify_module_with_config(&verifier_config, &m_de).map_err(|e| {
            check_for_invariant_violation_vmerror(e);
            Corpus::Reject
        })?
    }

    if let ExecVariant::Script {
        script: s,
        type_args: _,
        args: _,
    } = &input.exec_variant
    {
        // reject bad scripts fast
        let mut script_code: Vec<u8> = vec![];
        s.serialize(&mut script_code).map_err(|_| Corpus::Keep)?;
        let s_de = CompiledScript::deserialize_with_config(&script_code, &deserializer_config)
            .map_err(|_| Corpus::Reject)?;
        move_bytecode_verifier::verify_script_with_config(&verifier_config, &s_de).map_err(|e| {
            check_for_invariant_violation_vmerror(e);
            Corpus::Reject
        })?
    }

    // check no duplicates
    let mset: HashSet<_> = input.dep_modules.iter().map(|m| m.self_id()).collect();
    if mset.len() != input.dep_modules.len() {
        return Err(Corpus::Reject);
    }

    // topologically order modules {
    let all_modules = input.dep_modules.clone();
    let mut map = all_modules
        .into_iter()
        .map(|m| (m.self_id(), m))
        .collect::<BTreeMap<_, _>>();
    let mut order = vec![];
    for id in map.keys() {
        let mut visited = HashSet::new();
        sort_by_deps(&map, &mut order, id.clone(), &mut visited)?;
    }
    // }

    // group same address modules in packages. keep local ordering.
    let mut packages = vec![];
    for cur_package_id in order.iter() {
        let mut cur = vec![];
        if !map.contains_key(cur_package_id) {
            continue;
        }
        // this makes sure we keep the order in packages
        for id in order.iter() {
            // check if part of current package
            if id.address() == cur_package_id.address() {
                if let Some(module) = map.remove(cur_package_id) {
                    cur.push(module);
                }
            }
        }
        packages.push(cur)
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

    let sender_acc = if true {
        // create sender pub/priv key. initialize and fund account
        vm.create_accounts(1, input.tx_auth_type.sender().fund_amount(), 0)
            .remove(0)
    } else {
        // only create sender pub/priv key. do not initialize
        Account::new()
    };

    // build tx
    let tx = match input.exec_variant.clone() {
        ExecVariant::Script {
            script,
            type_args,
            args,
        } => {
            let mut script_bytes = vec![];
            script
                .serialize(&mut script_bytes)
                .map_err(|_| Corpus::Reject)?;
            sender_acc
                .transaction()
                .gas_unit_price(100)
                .max_gas_amount(1000)
                .sequence_number(0)
                .payload(TransactionPayload::Script(Script::new(
                    script_bytes,
                    type_args,
                    args.into_iter()
                        .map(|x| x.try_into())
                        .collect::<Result<Vec<TransactionArgument>, _>>()
                        .map_err(|_| Corpus::Reject)?,
                )))
        },
        ExecVariant::CallFunction {
            module,
            function,
            type_args,
            args,
        } => {
            // convert FunctionDefinitionIndex to function name... {
            let cm = input
                .dep_modules
                .iter()
                .find(|m| m.self_id() == module)
                .ok_or(Corpus::Reject)?;
            let fhi = cm
                .function_defs
                .get(function.0 as usize)
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
                    module,
                    function_name,
                    type_args,
                    args,
                )))
        },
    };
    let raw_tx = tx.raw();
    let tx = match input.tx_auth_type {
        FuzzerRunnableAuthenticator::Ed25519 { sender: _ } => raw_tx
            .sign(&sender_acc.privkey, sender_acc.pubkey.as_ed25519().unwrap())
            .map_err(|_| Corpus::Reject)?
            .into_inner(),
        FuzzerRunnableAuthenticator::MultiAgent {
            sender: _,
            secondary_signers,
        } => {
            // higher number here slows down fuzzer significatly due to slow signing process.
            if secondary_signers.len() > 10 {
                return Err(Corpus::Reject);
            }
            let secondary_accs: Vec<_> = secondary_signers
                .iter()
                .map(|acc| acc.convert_account(&mut vm))
                .collect();
            let secondary_signers = secondary_accs.iter().map(|acc| *acc.address()).collect();
            let secondary_private_keys = secondary_accs.iter().map(|acc| &acc.privkey).collect();
            raw_tx
                .sign_multi_agent(
                    &sender_acc.privkey,
                    secondary_signers,
                    secondary_private_keys,
                )
                .map_err(|_| Corpus::Reject)?
                .into_inner()
        },
        FuzzerRunnableAuthenticator::FeePayer {
            sender: _,
            secondary_signers,
            fee_payer,
        } => {
            // higher number here slows down fuzzer significatly due to slow signing process.
            if secondary_signers.len() > 10 {
                return Err(Corpus::Reject);
            }
            let secondary_accs: Vec<_> = secondary_signers
                .iter()
                .map(|acc| acc.convert_account(&mut vm))
                .collect();

            let secondary_signers = secondary_accs.iter().map(|acc| *acc.address()).collect();
            let secondary_private_keys = secondary_accs.iter().map(|acc| &acc.privkey).collect();
            let fee_payer_acc = fee_payer.convert_account(&mut vm);
            raw_tx
                .sign_fee_payer(
                    &sender_acc.privkey,
                    secondary_signers,
                    secondary_private_keys,
                    *fee_payer_acc.address(),
                    &fee_payer_acc.privkey,
                )
                .map_err(|_| Corpus::Reject)?
                .into_inner()
        },
    };

    // exec tx
    tdbg!("exec start");
    let mut old_res = None;
    const N_EXTRA_RERUNS: usize = 0;
    #[allow(clippy::reversed_empty_ranges)]
    for _ in 0..N_EXTRA_RERUNS {
        let res = vm.execute_block(vec![tx.clone()]);
        if let Some(old_res) = old_res {
            assert!(old_res == res);
        }
        old_res = Some(res);
    }

    let now = Instant::now();
    let res = vm.execute_block(vec![tx.clone()]);
    let elapsed = now.elapsed();

    // check main execution as well
    if let Some(old_res) = old_res {
        assert!(old_res == res);
    }
    let res = res
        .map_err(|e| {
            check_for_invariant_violation(e);
            Corpus::Keep
        })?
        .pop()
        .expect("expect 1 output");
    tdbg!("exec end");

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
        ExecutionStatus::Success => (),
        ExecutionStatus::MiscellaneousError(e) => {
            if let Some(e) = e {
                if e.status_type() == StatusType::InvariantViolation
                    && *e != StatusCode::TYPE_RESOLUTION_FAILURE
                    && *e != StatusCode::STORAGE_ERROR
                {
                    panic!("invariant violation {:?}", e);
                }
            }
            return Err(Corpus::Keep);
        },
        _ => return Err(Corpus::Keep),
    };

    let fee = res.try_extract_fee_statement().unwrap().unwrap();

    // EXECUTION_TIME_GAS_RATIO is a ratio between execution time and gas used. If the ratio is higher than EXECUTION_TIME_GAS_ratio, we consider the gas usage as unexpected.
    // EXPERIMENTAL: This very sensible to excution enviroment, e.g. local run, OSS-Fuzz. It may cause false positive. Real data from production does not apply to this ratio.
    // We only want to catch big unexpected gas usage.
    if ((elapsed.as_millis() / (fee.execution_gas_used() + fee.io_gas_used()) as u128)
        > EXECUTION_TIME_GAS_RATIO as u128)
        && !is_coverage_enabled()
    {
        if std::env::var("DEBUG").is_ok() {
            tdbg!(
                "Potential unexpected gas usage detected. Execution time: {:?}, Gas burned: {:?}",
                elapsed,
                fee.execution_gas_used() + fee.io_gas_used()
            );
            tdbg!("Transaction: {:?}", tx);
        } else {
            panic!(
                "Potential unexpected gas usage detected. Execution time: {:?}, Gas burned: {:?}",
                elapsed,
                fee.execution_gas_used() + fee.io_gas_used()
            );
        }
    }

    Ok(())
}

fuzz_target!(|fuzz_data: RunnableState| -> Corpus {
    run_case(fuzz_data).err().unwrap_or(Corpus::Keep)
});
