// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(dead_code)]

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
    value::{MoveStructLayout, MoveTypeLayout, MoveValue},
    vm_status::{StatusType, VMStatus},
};
use std::collections::{BTreeMap, BTreeSet, HashSet};

#[macro_export]
macro_rules! tdbg {
    () => {
        ()
    };
    ($val:expr $(,)?) => {
        {
            if std::env::var("DEBUG").is_ok() {
                dbg!($val)
            } else {
                $val
            }
        }
    };
    ($($val:expr),+ $(,)?) => {
        {
            if std::env::var("DEBUG").is_ok() {
                dbg!($($val),+)
            } else {
                ($($val),+)
            }
        }
    };
}

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
    pub fn fund_amount(&self) -> u64 {
        match self.fund {
            FundAmount::Zero => 0,
            FundAmount::Poor => 1_000,
            FundAmount::Rich => 1_000_000_000_000_000,
        }
    }

    pub fn convert_account(&self, vm: &mut FakeExecutor) -> Account {
        if self.is_inited_and_funded {
            vm.create_accounts(1, self.fund_amount(), 0).remove(0)
        } else {
            Account::new()
        }
    }
}

impl Authenticator {
    pub fn sender(&self) -> UserAccount {
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

pub(crate) fn is_valid_layout(layout: &MoveTypeLayout) -> bool {
    use MoveTypeLayout as L;

    match layout {
        L::Bool | L::U8 | L::U16 | L::U32 | L::U64 | L::U128 | L::U256 | L::Address | L::Signer => {
            true
        },

        L::Vector(layout) | L::Native(_, layout) => is_valid_layout(layout),
        L::Struct(MoveStructLayout::RuntimeVariants(variants)) => {
            variants.iter().all(|v| v.iter().all(is_valid_layout))
        },
        L::Struct(MoveStructLayout::Runtime(fields)) => {
            if fields.is_empty() {
                return false;
            }
            fields.iter().all(is_valid_layout)
        },
        L::Struct(_) => {
            // decorated layouts not supported
            false
        },
    }
}

pub(crate) fn compiled_module_serde(module: &CompiledModule) -> Result<(), ()> {
    let mut blob = vec![];
    module.serialize(&mut blob).map_err(|_| ())?;
    CompiledModule::deserialize(&blob).map_err(|_| ())?;
    Ok(())
}
