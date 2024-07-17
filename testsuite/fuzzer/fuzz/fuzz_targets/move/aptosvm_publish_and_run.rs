#![no_main]

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_cached_packages::aptos_stdlib::code_publish_package_txn;
use aptos_framework::natives::code::{
    ModuleMetadata, MoveOption, PackageDep, PackageMetadata, UpgradePolicy,
};
use aptos_language_e2e_tests::{
    account::Account, data_store::GENESIS_CHANGE_SET_HEAD, executor::FakeExecutor,
};
use aptos_types::{
    chain_id::ChainId,
    transaction::{
        EntryFunction, ExecutionStatus, Script, TransactionArgument, TransactionPayload,
        TransactionStatus,
    },
    write_set::WriteSet,
};
use aptos_vm::AptosVM;
use arbitrary::Arbitrary;
use libfuzzer_sys::{fuzz_target, Corpus};
use move_binary_format::{
    access::ModuleAccess,
    deserializer::DeserializerConfig,
    errors::VMError,
    file_format::{CompiledModule, CompiledScript, FunctionDefinitionIndex, SignatureToken},
};
use move_bytecode_verifier::VerifierConfig;
use move_core_types::{
    language_storage::{ModuleId, TypeTag},
    value::MoveValue,
    vm_status::{StatusCode, StatusType, VMStatus},
};
use once_cell::sync::Lazy;
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    convert::TryInto,
    sync::Arc,
};

#[derive(Debug, Arbitrary, Eq, PartialEq, Clone, Copy)]
pub enum FundAmount {
    Zero,
    Poor,
    Rich,
}

#[derive(Debug, Arbitrary, Eq, PartialEq, Clone, Copy)]
pub struct UserAccount {
    is_inited_and_funded: bool,
    fund: FundAmount,
}

#[derive(Debug, Arbitrary, Eq, PartialEq, Clone)]
pub enum Authenticator {
    Ed25519 {
        sender: UserAccount,
    },
    MultiAgent {
        sender: UserAccount,
        secondary_signers: Vec<UserAccount>,
    },
    FeePayer {
        sender: UserAccount,
        secondary_signers: Vec<UserAccount>,
        fee_payer: UserAccount,
    },
}

impl UserAccount {
    fn fund_amount(&self) -> u64 {
        match self.fund {
            FundAmount::Zero => 0,
            FundAmount::Poor => 1_000,
            FundAmount::Rich => 1_000_000_000_000_000,
        }
    }

    fn convert_account(&self, vm: &mut FakeExecutor) -> Account {
        if self.is_inited_and_funded {
            vm.create_accounts(1, self.fund_amount(), 0).remove(0)
        } else {
            Account::new()
        }
    }
}

impl Authenticator {
    fn sender(&self) -> UserAccount {
        match self {
            Authenticator::Ed25519 { sender } => *sender,
            Authenticator::MultiAgent {
                sender,
                secondary_signers: _,
            } => *sender,
            Authenticator::FeePayer {
                sender,
                secondary_signers: _,
                fee_payer: _,
            } => *sender,
        }
    }
}

#[derive(Debug, Arbitrary, Eq, PartialEq, Clone)]
pub enum ExecVariant {
    Script {
        script: CompiledScript,
        type_args: Vec<TypeTag>,
        args: Vec<MoveValue>,
    },
    CallFunction {
        module: ModuleId,
        function: FunctionDefinitionIndex,
        type_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
    },
}

#[derive(Debug, Arbitrary, Eq, PartialEq, Clone)]
pub struct RunnableState {
    pub dep_modules: Vec<CompiledModule>,
    pub exec_variant: ExecVariant,
    pub tx_auth_type: Authenticator,
}

// genesis write set generated once for each fuzzing session
static VM: Lazy<WriteSet> = Lazy::new(|| GENESIS_CHANGE_SET_HEAD.write_set().clone());

const FUZZER_CONCURRENCY_LEVEL: usize = 1;
static TP: Lazy<Arc<rayon::ThreadPool>> = Lazy::new(|| {
    Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(FUZZER_CONCURRENCY_LEVEL)
            .build()
            .unwrap(),
    )
});

// small debug macro which can be enabled or disabled
const DEBUG: bool = false;
macro_rules! tdbg {
    () => {
        ()
    };
    ($val:expr $(,)?) => {
        if DEBUG {
            dbg!($val)
        } else {
            ($val)
        }
    };
    ($($val:expr),+ $(,)?) => {
        if DEBUG {
            dbg!($(($val)),+,)
        } else {
            ($(($val)),+,)
        }
    };
}

const MAX_TYPE_PARAMETER_VALUE: u16 = 64 / 4 * 16; // third_party/move/move-bytecode-verifier/src/signature_v2.rs#L1306-L1312

// used for ordering modules topologically
fn sort_by_deps(
    map: &BTreeMap<ModuleId, CompiledModule>,
    order: &mut Vec<ModuleId>,
    id: ModuleId,
    visited: &mut HashSet<ModuleId>,
) -> Result<(), Corpus> {
    if visited.contains(&id) {
        return Err(Corpus::Keep);
    }
    visited.insert(id.clone());
    if order.contains(&id) {
        return Ok(());
    }
    let compiled = &map.get(&id).unwrap();
    for dep in compiled.immediate_dependencies() {
        // Only consider deps which are actually in this package. Deps for outside
        // packages are considered fine because of package deployment order. Note
        // that because of this detail, we can't use existing topsort from Move utils.
        if map.contains_key(&dep) {
            sort_by_deps(map, order, dep, visited)?;
        }
    }
    order.push(id);
    Ok(())
}

// panic to catch invariant violations
fn check_for_invariant_violation(e: VMStatus) {
    if e.status_type() == StatusType::InvariantViolation {
        // known false positive
        if e.message() != Some(&"moving container with dangling references".to_string()) {
            panic!("invariant violation {:?}", e);
        }
    }
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

fn publish_transaction_payload(modules: &[CompiledModule]) -> TransactionPayload {
    let modules_metadatas: Vec<_> = modules
        .iter()
        .map(|cm| ModuleMetadata {
            name: cm.name().to_string(),
            source: vec![],
            source_map: vec![],
            extension: MoveOption::default(),
        })
        .collect();

    let all_immediate_deps: Vec<_> = modules
        .iter()
        .flat_map(|cm| cm.immediate_dependencies())
        .map(|mi| PackageDep {
            account: mi.address,
            package_name: mi.name.to_string(),
        })
        .collect::<BTreeSet<_>>() // leave only uniques
        .into_iter()
        .filter(|c| &c.account != modules[0].address()) // filter out package itself
        .collect::<Vec<_>>();

    let metadata = PackageMetadata {
        name: "fuzz_package".to_string(),
        upgrade_policy: UpgradePolicy::compat(), // TODO: currently does not matter. Maybe fuzz compat checks specifically at some point.
        upgrade_number: 1,
        source_digest: "".to_string(),
        manifest: vec![],
        modules: modules_metadatas,
        deps: all_immediate_deps,
        extension: MoveOption::default(),
    };
    let pkg_metadata = bcs::to_bytes(&metadata).expect("PackageMetadata must serialize");
    let mut pkg_code: Vec<Vec<u8>> = vec![];
    for module in modules {
        let mut module_code: Vec<u8> = vec![];
        module
            .serialize(&mut module_code)
            .expect("Module must serialize");
        pkg_code.push(module_code);
    }
    code_publish_package_txn(pkg_metadata, pkg_code)
}

fn run_case(mut input: RunnableState) -> Result<(), Corpus> {
    tdbg!(&input);

    // filter modules
    filter_modules(&input)?;

    let verifier_config = VerifierConfig::production();
    let deserializer_config = DeserializerConfig::default();

    for m in input.dep_modules.iter_mut() {
        // m.metadata = vec![]; // we could optimize metadata to only contain aptos metadata
        // m.version = VERSION_MAX;

        // reject bad modules fast
        let mut module_code: Vec<u8> = vec![];
        m.serialize(&mut module_code).map_err(|_| Corpus::Keep)?;
        let m_de = CompiledModule::deserialize_with_config(&module_code, &deserializer_config)
            .map_err(|_| Corpus::Keep)?;
        move_bytecode_verifier::verify_module_with_config(&verifier_config, &m_de).map_err(|e| {
            check_for_invariant_violation_vmerror(e);
            Corpus::Keep
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
            .map_err(|_| Corpus::Keep)?;
        move_bytecode_verifier::verify_script_with_config(&verifier_config, &s_de).map_err(|e| {
            check_for_invariant_violation_vmerror(e);
            Corpus::Keep
        })?
    }

    // check no duplicates
    let mset: HashSet<_> = input.dep_modules.iter().map(|m| m.self_id()).collect();
    if mset.len() != input.dep_modules.len() {
        return Err(Corpus::Keep);
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
        &VM,
        ChainId::mainnet(),
        Arc::clone(&TP),
    )
    .set_not_parallel();

    // publish all packages
    for group in packages {
        let sender = *group[0].address();
        let acc = vm.new_account_at(sender);
        let tx = acc
            .transaction()
            .gas_unit_price(100)
            .sequence_number(0)
            .payload(publish_transaction_payload(&group))
            .sign();

        tdbg!("publishing");
        let res = vm
            .execute_block(vec![tx])
            .map_err(|e| {
                check_for_invariant_violation(e);
                Corpus::Keep
            })?
            .pop()
            .expect("expected 1 output");
        // if error exit gracefully
        tdbg!(&res);
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
        tdbg!(&status);
        // apply write set to commit published packages
        vm.apply_write_set(res.write_set());
        match tdbg!(status) {
            ExecutionStatus::Success => (),
            ExecutionStatus::MiscellaneousError(e) => {
                if let Some(e) = e {
                    if e.status_type() == StatusType::InvariantViolation {
                        panic!("invariant violation {:?}", e);
                    }
                }
                return Err(Corpus::Keep);
            },
            _ => return Err(Corpus::Keep),
        };

        tdbg!("published");
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
                .map_err(|_| Corpus::Keep)?;
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
                        .map_err(|_| Corpus::Keep)?,
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
                .ok_or(Corpus::Keep)?;
            let fhi = cm
                .function_defs
                .get(function.0 as usize)
                .ok_or(Corpus::Keep)?
                .function;
            let function_identifier_index = cm
                .function_handles
                .get(fhi.0 as usize)
                .ok_or(Corpus::Keep)?
                .name;
            let function_name = cm
                .identifiers
                .get(function_identifier_index.0 as usize)
                .ok_or(Corpus::Keep)?
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
        Authenticator::Ed25519 { sender: _ } => raw_tx
            .sign(&sender_acc.privkey, sender_acc.pubkey.as_ed25519().unwrap())
            .map_err(|_| Corpus::Keep)?
            .into_inner(),
        Authenticator::MultiAgent {
            sender: _,
            secondary_signers,
        } => {
            // higher number here slows down fuzzer significatly due to slow signing process.
            if secondary_signers.len() > 10 {
                return Err(Corpus::Keep);
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
                .map_err(|_| Corpus::Keep)?
                .into_inner()
        },
        Authenticator::FeePayer {
            sender: _,
            secondary_signers,
            fee_payer,
        } => {
            // higher number here slows down fuzzer significatly due to slow signing process.
            if secondary_signers.len() > 10 {
                return Err(Corpus::Keep);
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
                .map_err(|_| Corpus::Keep)?
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
    let res = vm.execute_block(vec![tx]);
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

    Ok(())
}

fuzz_target!(|fuzz_data: RunnableState| -> Corpus {
    run_case(fuzz_data).err().unwrap_or(Corpus::Keep)
});
