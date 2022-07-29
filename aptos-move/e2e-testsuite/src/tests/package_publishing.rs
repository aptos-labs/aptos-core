// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::access_path::{AccessPath, Path};
use aptos_types::transaction::ScriptFunction;
use aptos_types::vm_status::StatusCode;
use aptos_types::{
    on_chain_config::VMPublishingOption,
    transaction::{ExecutionStatus, TransactionStatus},
};
use aptos_vm::move_vm_ext::{ModuleMetadata, PackageMetadata, PackageRegistry, UpgradePolicy};
use itertools::Itertools;
use language_e2e_tests::{
    account::Account, compile::compile_module, current_function_name, executor::FakeExecutor,
};
use move_deps::move_core_types::account_address::AccountAddress;
use move_deps::move_core_types::identifier::Identifier;
use move_deps::move_core_types::language_storage::{ModuleId, StructTag, TypeTag};

// =============================================
// Test Fixture

// TODO: pull this out and generalize, perhaps Move into executor
// TODO: support .move and not just .mvir syntax

struct Fixture {
    executor: FakeExecutor,
    package: PackageMetadata,
    code: Vec<Vec<u8>>,
    txn_seq_no: u64,
}

impl Fixture {
    /// Creates new test for package of given name.
    fn new(name: &str, upgrade_policy: UpgradePolicy) -> Self {
        Self {
            executor: FakeExecutor::from_genesis_with_options(VMPublishingOption::open()),
            package: PackageMetadata {
                name: name.to_string(),
                upgrade_policy,
                manifest: "".to_string(),
                modules: vec![],
            },
            code: vec![],
            txn_seq_no: 10,
        }
    }

    /// Clear the configure package
    fn clear(&mut self) {
        self.package.modules.clear();
        self.code.clear()
    }

    /// Creates an account
    fn new_account(&mut self) -> Account {
        let data = self.executor.create_raw_account_data(1_000_000, 10);
        self.executor.add_account_data(&data);
        data.account().clone()
    }

    /// Adds a module.
    fn add_module(&mut self, addr: &AccountAddress, name: &str, decls: &[&str]) {
        let program = format!(
            "
        module 0x{}.{} {{
          {}
        }}
        ",
            addr,
            name,
            decls.iter().join("\n  ")
        );
        self.package.modules.push(ModuleMetadata {
            name: name.to_string(),
            source: program.clone(),
            source_map: vec![],
            abi: vec![],
        });
        self.code.push(compile_module(&program).1.into_inner());
    }

    /// Runs a transaction.
    fn run_txn(
        &mut self,
        account: &Account,
        addr: AccountAddress,
        module: &str,
        fun: &str,
        ty_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
    ) -> TransactionStatus {
        let seq_no = self.txn_seq_no;
        self.txn_seq_no += 1;
        let txn = account
            .transaction()
            .sequence_number(seq_no)
            .gas_unit_price(1)
            .script_function(ScriptFunction::new(
                ModuleId::new(addr, Identifier::new(module).unwrap()),
                Identifier::new(fun).unwrap(),
                ty_args,
                args,
            ))
            .sign();
        let output = self.executor.execute_transaction(txn);
        if output.status().status() == Ok(ExecutionStatus::Success) {
            self.executor.apply_write_set(output.write_set());
        }
        output.status().to_owned()
    }

    /// Publishes the package as configured so far by this fixture.
    fn publish_package(&mut self, account: &Account) -> TransactionStatus {
        self.run_txn(
            account,
            AccountAddress::ONE,
            "code",
            "publish_package_txn",
            vec![],
            vec![
                bcs::to_bytes(&bcs::to_bytes(&self.package).unwrap()).unwrap(),
                bcs::to_bytes(&self.code).unwrap(),
            ],
        )
    }

    /// Reads a resource.
    ///
    /// TODO: would like to do bcs::from_bytes::<T> in here, but it appears it requires that T
    /// can borrow from the blob, so any blob from which you deserialize in bcs seems need to
    /// outlive the value. It would make sense if someone wants to do "lazy" deserialization, but
    /// it is indeed a usability killer; we should consider pushing for changing this.
    fn read_resource(
        &self,
        addr: AccountAddress,
        code_addr: AccountAddress,
        module: &str,
        name: &str,
    ) -> Vec<u8> {
        let path = AccessPath::new(
            addr,
            bcs::to_bytes(&Path::Resource(StructTag {
                address: code_addr,
                module: Identifier::new(module).unwrap(),
                name: Identifier::new(name).unwrap(),
                type_params: vec![],
            }))
            .unwrap(),
        );
        self.executor.read_from_access_path(&path).unwrap()
    }
}

// =============================================
// Tests

#[test]
fn basic() {
    let mut tfx = Fixture::new("my_pack", UpgradePolicy::no_compat());
    tfx.executor.set_golden_file(current_function_name!());
    let acc = tfx.new_account();
    tfx.add_module(
        acc.address(),
        "my_mod",
        &["public entry hello(s: &signer) {
              label l0:
              return;
             }"],
    );
    assert_eq!(
        tfx.publish_package(&acc),
        TransactionStatus::Keep(ExecutionStatus::Success)
    );

    // Validate metadata present as expected
    let registry_bytes = tfx.read_resource(
        *acc.address(),
        AccountAddress::ONE,
        "code",
        "PackageRegistry",
    );
    let registry = bcs::from_bytes::<PackageRegistry>(&registry_bytes).unwrap();
    assert_eq!(registry.packages.len(), 1);
    assert_eq!(registry.packages[0].name, "my_pack");
    assert_eq!(registry.packages[0].modules.len(), 1);
    assert_eq!(registry.packages[0].modules[0].name, "my_mod");

    // Validate code loaded as expected. We consider it as proven if we can call the
    // hello transaction.
    assert_eq!(
        tfx.run_txn(&acc, *acc.address(), "my_mod", "hello", vec![], vec![]),
        TransactionStatus::Keep(ExecutionStatus::Success)
    )
}

#[test]
fn upgrade_success_no_compat() {
    let mut tfx = Fixture::new("my_pack", UpgradePolicy::no_compat());
    tfx.executor.set_golden_file(current_function_name!());
    let acc = tfx.new_account();

    for i in 1..3 {
        // Create different function each iteration
        let fun = format!(
            "public entry hello{}(s: &signer) {{
              label l0:
              return;
             }}",
            i
        );
        tfx.add_module(acc.address(), "my_mod", &[&fun]);
        // Expect upgrade successful
        let status = tfx.publish_package(&acc);
        assert_eq!(status, TransactionStatus::Keep(ExecutionStatus::Success));
        tfx.clear()
    }
}

#[test]
fn upgrade_fail_compat() {
    let mut tfx = Fixture::new("my_pack", UpgradePolicy::compat());
    tfx.executor.set_golden_file(current_function_name!());
    let acc = tfx.new_account();

    for i in 1..3 {
        // Create different function each iteration
        let fun = format!(
            "public entry hello{}(s: &signer) {{
              label l0:
              return;
             }}",
            i
        );
        tfx.add_module(acc.address(), "my_mod", &[&fun]);
        let status = tfx.publish_package(&acc);
        if i == 1 {
            assert_eq!(status, TransactionStatus::Keep(ExecutionStatus::Success));
        } else {
            // Expect 2nd upgrade to produce error
            assert_eq!(
                status,
                TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
                    StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE
                )))
            )
        }
        tfx.clear()
    }
}

#[test]
fn upgrade_success_compat() {
    let mut tfx = Fixture::new("my_pack", UpgradePolicy::compat());
    tfx.executor.set_golden_file(current_function_name!());
    let acc = tfx.new_account();

    for i in 1..3 {
        // Create downwards compatible functions hello_0..hello_i
        let mut decls = vec![];
        for j in 0..i {
            decls.push(format!(
                "public entry hello{}(s: &signer) {{
              label l0:
              return;
             }}",
                j
            ));
        }
        tfx.add_module(
            acc.address(),
            "my_mod",
            &decls.iter().map(|s| s.as_str()).collect::<Vec<&str>>(),
        );
        let status = tfx.publish_package(&acc);
        assert_eq!(status, TransactionStatus::Keep(ExecutionStatus::Success));
        tfx.clear()
    }
}

#[test]
fn upgrade_fail_immutable() {
    let mut tfx = Fixture::new("my_pack", UpgradePolicy::immutable());
    tfx.executor.set_golden_file(current_function_name!());
    let acc = tfx.new_account();

    for i in 1..3 {
        // Same function each time
        tfx.add_module(
            acc.address(),
            "my_mod",
            &["public entry hello(s: &signer) {
              label l0:
              return;
             }"],
        );
        let status = tfx.publish_package(&acc);
        if i == 1 {
            assert_eq!(status, TransactionStatus::Keep(ExecutionStatus::Success));
        } else {
            // Expect 2nd upgrade to produce error
            assert!(matches!(
                status,
                TransactionStatus::Keep(ExecutionStatus::MoveAbort { .. })
            ));
        }
        tfx.clear()
    }
}

#[test]
fn upgrade_fail_overlapping_module() {
    let mut tfx = Fixture::new("my_pack", UpgradePolicy::no_compat());
    tfx.executor.set_golden_file(current_function_name!());
    let acc = tfx.new_account();

    for i in 1..3 {
        tfx.add_module(
            acc.address(),
            "my_mod",
            &["public entry hello(s: &signer) {
              label l0:
              return;
             }"],
        );
        let status = tfx.publish_package(&acc);
        if i == 1 {
            assert_eq!(status, TransactionStatus::Keep(ExecutionStatus::Success));
        } else {
            // Expect 2nd upgrade to produce error
            assert!(matches!(
                status,
                TransactionStatus::Keep(ExecutionStatus::MoveAbort { .. })
            ));
        }
        // for the next round, change package name. We should not be allowed to
        // publish 'my_mod' in different packages at the same address
        tfx.package.name = "my_other_pack".to_string()
    }
}
