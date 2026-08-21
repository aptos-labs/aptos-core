// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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
use fuzzer::{BlockExecVariantV2, RunnableBlockTransactionV2, UserAccount};
use libfuzzer_sys::Corpus;
use move_binary_format::{
    access::ModuleAccess,
    deserializer::DeserializerConfig,
    errors::VMError,
    file_format::{
        CompiledModule, CompiledScript, FunctionDefinitionIndex, Signature, SignatureToken,
        Visibility,
    },
    file_format_common::{VERSION_MAX, VERSION_MIN},
    internals::ModuleIndex,
};
use move_core_types::{
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::ModuleId,
    vm_status::{StatusCode, StatusType, VMStatus},
};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    sync::atomic::{AtomicU64, Ordering},
};

pub const BYTECODE_VERSION: u32 = 8;
const MAX_TYPE_PARAMETER_VALUE: u16 = 64 / 4 * 16;

fn supported_bytecode_version(version: u32) -> u32 {
    if (VERSION_MIN..=VERSION_MAX).contains(&version) {
        version
    } else {
        VERSION_MAX
    }
}

pub(crate) fn module_bytecode_version(module: &CompiledModule) -> u32 {
    supported_bytecode_version(module.version)
}

pub(crate) fn script_bytecode_version(script: &CompiledScript) -> u32 {
    supported_bytecode_version(script.version)
}

pub(crate) fn module_self_id(module: &CompiledModule) -> Option<ModuleId> {
    let handle = module
        .module_handles
        .get(module.self_module_handle_idx.into_index())?;
    let address = module
        .address_identifiers
        .get(handle.address.into_index())?;
    let name = module.identifiers.get(handle.name.into_index())?.to_owned();
    Some(ModuleId::new(*address, name))
}

pub(crate) fn module_self_id_or_keep(module: &CompiledModule) -> Result<ModuleId, Corpus> {
    module_self_id(module).ok_or(Corpus::Keep)
}

pub(crate) fn checked_module_self_id(module: &CompiledModule) -> ModuleId {
    module_self_id(module).expect("module self id checked at fuzz case boundary")
}

pub(crate) fn resolve_module_ref(
    modules: &[CompiledModule],
    module_idx: u8,
) -> Result<&CompiledModule, Corpus> {
    if modules.is_empty() {
        return Err(Corpus::Keep);
    }
    Ok(&modules[module_idx as usize % modules.len()])
}

pub(crate) fn resolve_module_refs(
    modules: &[CompiledModule],
    module_idxs: &[u8],
) -> Result<Vec<CompiledModule>, Corpus> {
    const MAX_MODULE_REFS: usize = 32;

    if module_idxs.len() > MAX_MODULE_REFS {
        return Err(Corpus::Keep);
    }
    if module_idxs.is_empty() {
        return Ok(vec![]);
    }
    if modules.is_empty() {
        return Err(Corpus::Keep);
    }

    let mut seen = BTreeSet::new();
    Ok(module_idxs
        .iter()
        .map(|module_idx| *module_idx as usize % modules.len())
        .filter(|idx| seen.insert(*idx))
        .map(|idx| modules[idx].clone())
        .collect())
}

pub(crate) fn normalize_module_for_fuzz(
    mut module: CompiledModule,
) -> Result<CompiledModule, Corpus> {
    module.version = VERSION_MAX;
    let is_entry_by_handle = module
        .function_handles
        .iter()
        .map(|handle| {
            module
                .signatures
                .get(handle.return_.into_index())
                .is_some_and(|signature| signature.0.is_empty())
        })
        .collect::<Vec<_>>();

    for definition in &mut module.function_defs {
        if definition.code.is_none() {
            return Err(Corpus::Keep);
        }
        definition.visibility = Visibility::Public;
        definition.is_entry = is_entry_by_handle
            .get(definition.function.into_index())
            .copied()
            .unwrap_or(false);
    }

    Ok(module)
}

pub(crate) fn serialize_module_for_version(
    module: &CompiledModule,
    bytecode_version: u32,
) -> Result<Vec<u8>, Corpus> {
    let mut bytes = vec![];
    module
        .serialize_for_version(Some(bytecode_version), &mut bytes)
        .map_err(|e| {
            tdbg!("module serialization failed", &e);
            Corpus::Keep
        })?;
    Ok(bytes)
}

fn signature_token_is_too_large(token: &SignatureToken) -> bool {
    match token {
        SignatureToken::TypeParameter(idx) => *idx > MAX_TYPE_PARAMETER_VALUE,
        SignatureToken::Vector(inner)
        | SignatureToken::Reference(inner)
        | SignatureToken::MutableReference(inner) => signature_token_is_too_large(inner),
        SignatureToken::StructInstantiation(_, tys) => tys.iter().any(signature_token_is_too_large),
        SignatureToken::Function(args, results, _) => {
            args.iter().any(signature_token_is_too_large)
                || results.iter().any(signature_token_is_too_large)
        },
        _ => false,
    }
}

fn has_oversized_signatures(signatures: &[Signature]) -> bool {
    signatures
        .iter()
        .any(|signature| signature.0.iter().any(signature_token_is_too_large))
}

pub(crate) fn filter_bad_modules(modules: &mut [CompiledModule]) -> Result<(), Corpus> {
    for module in modules {
        if module_self_id(module).is_none_or(|id| *id.address() == AccountAddress::ONE) {
            return Err(Corpus::Keep);
        }
        for definition in &mut module.function_defs {
            definition.is_entry = true;
        }
    }
    Ok(())
}

pub(crate) fn filter_bad_tx(exec_variant: &BlockExecVariantV2) -> Result<(), Corpus> {
    match exec_variant {
        BlockExecVariantV2::Script { _script, .. } => {
            if has_oversized_signatures(&_script.signatures) {
                return Err(Corpus::Keep);
            }
            Ok(())
        },
        BlockExecVariantV2::Publish { _module_idxs } => {
            if _module_idxs.is_empty() {
                return Err(Corpus::Keep);
            }
            Ok(())
        },
        BlockExecVariantV2::CallFunction { .. } => Ok(()),
        BlockExecVariantV2::SplitBlock => Ok(()),
    }
}

pub(crate) fn is_split_block(transaction: &RunnableBlockTransactionV2) -> bool {
    matches!(&transaction.exec_variant, BlockExecVariantV2::SplitBlock)
}

pub(crate) fn has_invalid_split_blocks(transactions: &[RunnableBlockTransactionV2]) -> bool {
    transactions.first().is_some_and(is_split_block)
        || transactions.last().is_some_and(is_split_block)
        || transactions
            .windows(2)
            .any(|window| window.iter().all(is_split_block))
}

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
    if order.contains(&id) {
        return Ok(());
    }
    if visited.contains(&id) {
        return Err(Corpus::Keep);
    }
    visited.insert(id.clone());
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
        .serialize_for_version(Some(module_bytecode_version(module)), &mut module_code)
        .map_err(|e| {
            tdbg!("module serialization failed", &e);
            Corpus::Reject
        })?;
    let m_de = CompiledModule::deserialize_with_config(&module_code, deserializer_config).map_err(
        |e| {
            tdbg!("module deserialization failed", &e);
            Corpus::Reject
        },
    )?;
    move_bytecode_verifier::verify_module_with_config(verifier_config, &m_de).map_err(|e| {
        tdbg!("module verification failed", &e);
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
        .serialize_for_version(Some(script_bytecode_version(script)), &mut script_code)
        .map_err(|e| {
            tdbg!("script serialization failed", &e);
            Corpus::Reject
        })?;
    let s_de = CompiledScript::deserialize_with_config(&script_code, deserializer_config).map_err(
        |e| {
            tdbg!("script deserialization failed", &e);
            Corpus::Reject
        },
    )?;
    move_bytecode_verifier::verify_script_with_config(verifier_config, &s_de).map_err(|e| {
        tdbg!("script verification failed", &e);
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
        .map(|m| module_self_id_or_keep(&m).map(|id| (id, m)))
        .collect::<Result<BTreeMap<_, _>, _>>()?;
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
            if module_id_in_global_order.address() == package_address
                && let Some(module) = remaining_modules_map.remove(module_id_in_global_order)
            {
                current_package_for_address.push(module);
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
            .serialize_for_version(Some(module_bytecode_version(module)), &mut module_code)
            .expect("Module must serialize");
        pkg_code.push(module_code);
    }
    code_publish_package_txn(pkg_metadata, pkg_code)
}

pub(crate) fn publish_transaction_payload_with_package_names(
    modules: &[CompiledModule],
    package_name: &str,
    package_names_by_module: &BTreeMap<ModuleId, String>,
) -> TransactionPayload {
    let modules_metadata = modules
        .iter()
        .map(|module| ModuleMetadata {
            name: module.name().to_string(),
            source: vec![],
            source_map: vec![],
            extension: None,
        })
        .collect();

    let deps = modules
        .iter()
        .flat_map(|module| module.immediate_dependencies())
        .map(|id| PackageDep {
            account: id.address,
            package_name: package_names_by_module
                .get(&id)
                .cloned()
                .unwrap_or_else(|| id.name.to_string()),
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|dep| &dep.account != modules[0].address())
        .collect();

    let metadata = PackageMetadata {
        name: package_name.to_string(),
        upgrade_policy: UpgradePolicy::compat(),
        upgrade_number: 1,
        source_digest: "".to_string(),
        manifest: vec![],
        modules: modules_metadata,
        deps,
        extension: None,
    };
    let metadata = bcs::to_bytes(&metadata).expect("PackageMetadata must serialize");
    let code = modules
        .iter()
        .map(|module| {
            let mut code = vec![];
            module
                .serialize_for_version(Some(module_bytecode_version(module)), &mut code)
                .expect("Module must serialize");
            code
        })
        .collect();
    code_publish_package_txn(metadata, code)
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
    let requested = block.len();
    vm.execute_block(block)
        .map(|mut outputs| {
            // Trim any appended BlockEpilogue to preserve outputs.len() == requested.
            if outputs.len() > requested {
                outputs.truncate(requested);
            }
            outputs
        })
        .map_err(|e| {
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

    let outputs = execute_block_or_keep(vm, vec![tx])?;
    let res = outputs.into_iter().next().expect("expected 1 output");
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
            if let Some(e) = e
                && e.status_type() == StatusType::InvariantViolation
                && *e != StatusCode::VERIFICATION_ERROR
            {
                panic!(
                    "invariant violation via ExecutionStatus: {:?}, {:?}",
                    e,
                    res.auxiliary_data()
                );
            }
            Err(Corpus::Keep)
        },
        _ => Err(Corpus::Keep),
    }
}
