// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(dead_code)]

use crate::tdbg;
use aptos_cached_packages::aptos_stdlib::code_publish_package_txn;
use aptos_crypto::HashValue;
use aptos_framework::natives::code::{ModuleMetadata, PackageDep, PackageMetadata, UpgradePolicy};
use aptos_language_e2e_tests::{account::Account, executor::FakeExecutor};
use aptos_types::{
    block_executor::transaction_slice_metadata::TransactionSliceMetadata,
    transaction::{
        ExecutionStatus, RawTransaction, SignedTransaction, TransactionOutput, TransactionPayload,
        TransactionStatus,
    },
};
use arbitrary::Arbitrary;
use fuzzer::UserAccount;
use libfuzzer_sys::Corpus;
use move_binary_format::{
    access::ModuleAccess,
    deserializer::DeserializerConfig,
    errors::VMError,
    file_format::{CompiledModule, CompiledScript, FunctionDefinitionIndex},
};
use move_core_types::{
    identifier::Identifier,
    language_storage::ModuleId,
    vm_status::{StatusCode, StatusType, VMStatus},
};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    sync::atomic::{AtomicU64, Ordering},
};

pub const BYTECODE_VERSION: u32 = 8;

// Used to fuzz the MoveVM
#[derive(Debug, Arbitrary, Eq, PartialEq, Clone)]
pub enum FuzzerRunnableAuthenticator {
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

impl FuzzerRunnableAuthenticator {
    pub fn sender(&self) -> UserAccount {
        match self {
            FuzzerRunnableAuthenticator::Ed25519 { sender } => *sender,
            FuzzerRunnableAuthenticator::MultiAgent {
                sender,
                secondary_signers: _,
            } => *sender,
            FuzzerRunnableAuthenticator::FeePayer {
                sender,
                secondary_signers: _,
                fee_payer: _,
            } => *sender,
        }
    }
}

// used for ordering modules topologically
pub(crate) fn sort_by_deps(
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

// Small, hot helpers — eligible for inlining
#[inline]
pub(crate) fn select_or_create_block_index<T>(
    exec_group: u64,
    blocks: &mut Vec<Vec<T>>,
    group_to_block_index: &mut HashMap<u64, usize>,
) -> usize {
    if exec_group == 0 {
        blocks.push(Vec::new());
        blocks.len() - 1
    } else if let Some(&idx) = group_to_block_index.get(&exec_group) {
        idx
    } else {
        let idx = blocks.len();
        blocks.push(Vec::new());
        group_to_block_index.insert(exec_group, idx);
        idx
    }
}

#[inline]
pub(crate) fn ensure_account_present(
    vm: &mut FakeExecutor,
    accounts_by_addr: &mut HashMap<move_core_types::account_address::AccountAddress, Account>,
    addr: move_core_types::account_address::AccountAddress,
) {
    accounts_by_addr
        .entry(addr)
        .or_insert_with(|| vm.new_account_at(addr));
}

#[inline]
pub(crate) fn ensure_accounts_present(
    vm: &mut FakeExecutor,
    accounts_by_addr: &mut HashMap<move_core_types::account_address::AccountAddress, Account>,
    addrs: &[move_core_types::account_address::AccountAddress],
) {
    for addr in addrs.iter().copied() {
        accounts_by_addr
            .entry(addr)
            .or_insert_with(|| vm.new_account_at(addr));
    }
}

#[inline]
pub(crate) fn resolve_function_name(
    module: &CompiledModule,
    def_idx: FunctionDefinitionIndex,
) -> Result<Identifier, Corpus> {
    let f_handle_idx = module
        .function_defs
        .get(def_idx.0 as usize)
        .ok_or(Corpus::Reject)?
        .function;
    let ident_idx = module
        .function_handles
        .get(f_handle_idx.0 as usize)
        .ok_or(Corpus::Reject)?
        .name;
    Ok(module
        .identifiers
        .get(ident_idx.0 as usize)
        .ok_or(Corpus::Reject)?
        .clone())
}

// Verification wrappers (avoid duplicating serialize/deserialize boilerplate)
pub(crate) fn verify_module_fast(
    module: &CompiledModule,
    verifier_config: &move_bytecode_verifier::VerifierConfig,
    deserializer_config: &DeserializerConfig,
) -> Result<(), Corpus> {
    let mut module_code = vec![];
    module
        .serialize_for_version(Some(BYTECODE_VERSION), &mut module_code)
        .map_err(|_| Corpus::Reject)?;
    let m_de = CompiledModule::deserialize_with_config(&module_code, deserializer_config)
        .map_err(|_| Corpus::Reject)?;
    move_bytecode_verifier::verify_module_with_config(verifier_config, &m_de).map_err(|e| {
        check_for_invariant_violation_vmerror(e);
        Corpus::Reject
    })
}

pub(crate) fn verify_script_fast(
    script: &CompiledScript,
    verifier_config: &move_bytecode_verifier::VerifierConfig,
    deserializer_config: &DeserializerConfig,
) -> Result<(), Corpus> {
    let mut script_code = vec![];
    script
        .serialize_for_version(Some(BYTECODE_VERSION), &mut script_code)
        .map_err(|_| Corpus::Reject)?;
    let s_de = CompiledScript::deserialize_with_config(&script_code, deserializer_config)
        .map_err(|_| Corpus::Reject)?;
    move_bytecode_verifier::verify_script_with_config(verifier_config, &s_de).map_err(|e| {
        check_for_invariant_violation_vmerror(e);
        Corpus::Reject
    })
}

pub(crate) fn group_modules_by_address_topo(
    dep_modules: Vec<CompiledModule>,
) -> Result<Vec<Vec<CompiledModule>>, Corpus> {
    let all_modules = dep_modules;
    let map = all_modules
        .into_iter()
        .map(|m| (m.self_id(), m))
        .collect::<BTreeMap<_, _>>();
    let mut order = vec![];
    for id in map.keys() {
        let mut visited = HashSet::new();
        sort_by_deps(&map, &mut order, id.clone(), &mut visited)?;
    }

    // group same address modules in packages. keep local ordering.
    let mut packages: Vec<Vec<CompiledModule>> = Vec::new();
    let mut remaining_modules_map = map.clone();
    for module_id_to_start_package in &order {
        if !remaining_modules_map.contains_key(module_id_to_start_package) {
            continue;
        }
        let package_address = module_id_to_start_package.address();
        let mut current_package_for_address: Vec<CompiledModule> = Vec::new();
        for module_id_in_global_order in &order {
            if module_id_in_global_order.address() == package_address {
                if let Some(module) = remaining_modules_map.remove(module_id_in_global_order) {
                    current_package_for_address.push(module);
                }
            }
        }
        if !current_package_for_address.is_empty() {
            packages.push(current_package_for_address);
        }
    }
    Ok(packages)
}

fn publish_transaction_payload(modules: &[CompiledModule]) -> TransactionPayload {
    let modules_metadatas: Vec<_> = modules
        .iter()
        .map(|cm| ModuleMetadata {
            name: cm.name().to_string(),
            source: vec![],
            source_map: vec![],
            extension: None,
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
        extension: None,
    };
    let pkg_metadata = bcs::to_bytes(&metadata).expect("PackageMetadata must serialize");
    let mut pkg_code: Vec<Vec<u8>> = vec![];
    for module in modules {
        let mut module_code: Vec<u8> = vec![];
        module
            .serialize_for_version(Some(BYTECODE_VERSION), &mut module_code)
            .expect("Module must serialize");
        pkg_code.push(module_code);
    }
    code_publish_package_txn(pkg_metadata, pkg_code)
}

// List of known false positive messages for invariant violations
// If some invariant violation do not come with a message, we need to attach a message to it at throwing site.
const KNOWN_FALSE_POSITIVES_VMSTATUS: &[&str] = &["moving container with dangling references"];

// panic to catch invariant violations
pub(crate) fn check_for_invariant_violation(e: VMStatus) {
    let is_known_false_positive = e.message().is_some_and(|msg| {
        KNOWN_FALSE_POSITIVES_VMSTATUS
            .iter()
            .any(|known| msg.starts_with(known))
    });

    if !is_known_false_positive {
        panic!(
            "invariant violation {:?}\n{}{:?} {}",
            e,
            "RUST_BACKTRACE=1 DEBUG_VM_STATUS=",
            e.status_code(),
            "./fuzz.sh run move_aptosvm_publish_and_run <ARTIFACT>"
        );
    }
}

// For VMError (verifier) invariants.
const KNOWN_FALSE_POSITIVES_VMERROR: &[&str] =
    &["too many type parameters/arguments in the program"];

pub(crate) fn check_for_invariant_violation_vmerror(e: VMError) {
    if e.status_type() == StatusType::InvariantViolation {
        let is_known_false_positive = e.message().is_some_and(|msg| {
            KNOWN_FALSE_POSITIVES_VMERROR
                .iter()
                .any(|known| msg.starts_with(known))
        });

        if !is_known_false_positive {
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

// Optional block execution wrapper — useful for consistent error mapping
#[inline]
pub(crate) fn execute_block_or_keep(
    vm: &FakeExecutor,
    block: Vec<aptos_types::transaction::SignedTransaction>,
) -> Result<Vec<TransactionOutput>, Corpus> {
    vm.execute_block(block).map_err(|e| {
        check_for_invariant_violation(e);
        Corpus::Keep
    })
}

#[inline]
pub(crate) fn sign_single_or_multi(
    raw_tx: RawTransaction,
    sender_acc: &Account,
    secondary_addrs: &[move_core_types::account_address::AccountAddress],
    accounts_by_addr: &HashMap<move_core_types::account_address::AccountAddress, Account>,
) -> Result<SignedTransaction, Corpus> {
    if secondary_addrs.is_empty() {
        return raw_tx
            .sign(
                &sender_acc.privkey,
                sender_acc.pubkey.as_ed25519().expect("ed25519 public key"),
            )
            .map_err(|_| Corpus::Reject)
            .map(|c| c.into_inner());
    }

    let secondary_accounts: Vec<_> = secondary_addrs
        .iter()
        .map(|a| accounts_by_addr.get(a).expect("secondary account exists"))
        .collect();
    let secondary_privs = secondary_accounts.iter().map(|a| &a.privkey).collect();
    raw_tx
        .sign_multi_agent(
            &sender_acc.privkey,
            secondary_addrs.to_vec(),
            secondary_privs,
        )
        .map_err(|_| Corpus::Reject)
        .map(|c| c.into_inner())
}

// Shared sequential block metadata across all cached publishes and executions in this fuzzer.
// Ensures AptosModuleCacheManager sees consecutive (parent, child) pairs and preserves caches.
static NEXT_BLOCK_ID: AtomicU64 = AtomicU64::new(1);

#[inline]
pub(crate) fn next_block_metadata() -> TransactionSliceMetadata {
    let child = NEXT_BLOCK_ID.fetch_add(1, Ordering::Relaxed);
    TransactionSliceMetadata::block(HashValue::from_u64(child - 1), HashValue::from_u64(child))
}

pub(crate) fn publish_group(
    vm: &mut FakeExecutor,
    acc: &Account,
    group: &[CompiledModule],
    sequence_number: u64,
) -> Result<(), Corpus> {
    tdbg!("publishing");
    let tx = acc
        .transaction()
        .gas_unit_price(100)
        .sequence_number(sequence_number)
        .payload(publish_transaction_payload(group))
        .sign();

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
                panic!(
                    "invariant violation via TransactionStatus: {:?}, {:?}",
                    e,
                    res.auxiliary_data()
                );
            }
            return Err(Corpus::Keep);
        },
        _ => return Err(Corpus::Keep),
    };
    // apply write set to commit published packages
    vm.apply_write_set(res.write_set());
    match tdbg!(status) {
        ExecutionStatus::Success => Ok(()),
        ExecutionStatus::MiscellaneousError(e) => {
            if let Some(e) = e {
                if e.status_type() == StatusType::InvariantViolation
                    && *e != StatusCode::VERIFICATION_ERROR
                {
                    panic!(
                        "invariant violation via ExecutionStatus: {:?}, {:?}",
                        e,
                        res.auxiliary_data()
                    );
                }
            }
            Err(Corpus::Keep)
        },
        _ => Err(Corpus::Keep),
    }
}
