// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(dead_code)]

use super::helpers::UserAccount;
use crate::tdbg;
use aptos_cached_packages::aptos_stdlib::code_publish_package_txn;
use aptos_framework::natives::code::{
    ModuleMetadata, MoveOption, PackageDep, PackageMetadata, UpgradePolicy,
};
use aptos_language_e2e_tests::{account::Account, executor::FakeExecutor};
use aptos_types::transaction::{ExecutionStatus, TransactionPayload, TransactionStatus};
use arbitrary::Arbitrary;
use libfuzzer_sys::Corpus;
use move_binary_format::{
    access::ModuleAccess,
    file_format::{CompiledModule, CompiledScript, FunctionDefinitionIndex},
};
use move_core_types::{
    language_storage::{ModuleId, TypeTag},
    value::MoveValue,
    vm_status::{StatusType, VMStatus},
};
use std::collections::{BTreeMap, BTreeSet, HashSet};

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
    pub tx_auth_type: FuzzerRunnableAuthenticator,
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

// panic to catch invariant violations
pub(crate) fn check_for_invariant_violation(e: VMStatus) {
    if e.status_type() == StatusType::InvariantViolation {
        // known false positive
        if e.message() != Some(&"moving container with dangling references".to_string()) {
            panic!("invariant violation {:?}", e);
        }
    }
}

pub(crate) fn publish_group(
    vm: &mut FakeExecutor,
    acc: &Account,
    group: &[CompiledModule],
    sequence_number: u64,
) -> Result<(), Corpus> {
    let tx = acc
        .transaction()
        .gas_unit_price(100)
        .sequence_number(sequence_number)
        .payload(publish_transaction_payload(group))
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
        ExecutionStatus::Success => Ok(()),
        ExecutionStatus::MiscellaneousError(e) => {
            if let Some(e) = e {
                if e.status_type() == StatusType::InvariantViolation {
                    panic!("invariant violation {:?}", e);
                }
            }
            Err(Corpus::Keep)
        },
        _ => Err(Corpus::Keep),
    }
}
