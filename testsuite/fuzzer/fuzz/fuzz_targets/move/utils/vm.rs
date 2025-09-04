// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(dead_code)]

use crate::tdbg;
use velor_cached_packages::velor_stdlib::code_publish_package_txn;
use velor_framework::natives::code::{
    ModuleMetadata, MoveOption, PackageDep, PackageMetadata, UpgradePolicy,
};
use velor_language_e2e_tests::{account::Account, executor::FakeExecutor};
use velor_types::transaction::{ExecutionStatus, TransactionPayload, TransactionStatus};
use arbitrary::Arbitrary;
use fuzzer::UserAccount;
use libfuzzer_sys::Corpus;
use move_binary_format::{access::ModuleAccess, file_format::CompiledModule};
use move_core_types::{
    language_storage::ModuleId,
    vm_status::{StatusCode, StatusType, VMStatus},
};
use std::collections::{BTreeMap, BTreeSet, HashSet};

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
    let is_known_false_positive = e.message().map_or(false, |msg| {
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
            "./fuzz.sh run move_velorvm_publish_and_run <ARTIFACT>"
        );
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
    tdbg!(&status);
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
