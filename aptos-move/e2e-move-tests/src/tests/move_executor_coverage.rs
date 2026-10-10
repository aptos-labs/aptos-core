// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for FakeExecutor test coverage collection

// Note: this module uses parameterized tests via the
// [`rstest` crate](https://crates.io/crates/rstest)
// to test for multiple feature combinations.

use crate::{assert_success, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_package_builder::PackageBuilder;
use aptos_types::account_address::AccountAddress;
use aptos_vm::function_usage::{
    install_function_usage_sink, FunctionUsageSink, TransactionFunctionUsage, UsageCallKind,
};
use move_core_types::{ident_str, identifier::Identifier, language_storage::ModuleId};
use move_coverage::coverage_map::CoverageMap;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[test]
fn test_coverage() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x815").unwrap());
    let mut builder = PackageBuilder::new("Package");
    let source = r#"
        module 0x815::m {
            entry fun foo(c: bool) {
              if (c) bar() else baz()
            }
            fun bar() {}
            fun baz() {}
        }
    "#;
    builder.add_source("m.move", source);
    let path = builder.write_to_temp().unwrap();
    let coverage_path = path.path().join("test_coverage").display().to_string();
    assert!(PathBuf::from(&coverage_path).parent().unwrap().exists());
    h.executor.enable_code_coverage(&coverage_path);

    assert_success!(h.publish_package_with_options(&acc, path.path(), BuildOptions::move_2()));

    // First run: expected to cover bar code
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x815::m::foo").unwrap(),
        vec![],
        vec![bcs::to_bytes(&true).unwrap(),]
    ));
    h.executor.save_code_coverage(&coverage_path).unwrap();
    assert!(is_function_called(&coverage_path, "bar"));
    assert!(!is_function_called(&coverage_path, "baz"));

    // Second run: expected to cover also baz code
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x815::m::foo").unwrap(),
        vec![],
        vec![bcs::to_bytes(&false).unwrap(),]
    ));
    h.executor.save_code_coverage(&coverage_path).unwrap();
    assert!(is_function_called(&coverage_path, "bar"));
    assert!(is_function_called(&coverage_path, "baz"));
}

#[test]
fn test_function_usage_observer() {
    struct RecordingSink {
        target: ModuleId,
        transactions: Mutex<Vec<TransactionFunctionUsage>>,
    }

    impl FunctionUsageSink for RecordingSink {
        fn is_target_module(&self, module_id: &ModuleId) -> bool {
            module_id == &self.target
        }

        fn record_transaction(&self, usage: TransactionFunctionUsage) {
            self.transactions.lock().unwrap().push(usage);
        }
    }

    let mut h = MoveHarness::new();
    let module_address = AccountAddress::from_hex_literal("0xc011ec7").unwrap();
    let acc = h.new_account_at(module_address);
    let mut builder = PackageBuilder::new("FunctionUsageObserver");
    builder.add_source(
        "usage.move",
        r#"
        module 0xc011ec7::usage {
            entry fun root() {
                nested();
                generic<u64>();
                let closure = || closure_target();
                closure();
            }
            fun nested() {}
            fun generic<T>() {}
            fun closure_target() {}
        }
        "#,
    );
    let path = builder.write_to_temp().unwrap();
    assert_success!(h.publish_package_with_options(&acc, path.path(), BuildOptions::move_2()));

    let sink = Arc::new(RecordingSink {
        target: ModuleId::new(module_address, ident_str!("usage").to_owned()),
        transactions: Mutex::new(vec![]),
    });
    let guard = install_function_usage_sink(sink.clone()).unwrap();
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xc011ec7::usage::root").unwrap(),
        vec![],
        vec![],
    ));
    drop(guard);

    let transactions = sink.transactions.lock().unwrap();
    assert_eq!(transactions.len(), 1);
    let usage = &transactions[0];
    assert_eq!(usage.sender, module_address);
    assert_eq!(usage.root_function.as_ref().unwrap().function_name, "root");
    assert_eq!(usage.calls.len(), 4);
    assert_eq!(usage.calls[0].callee.function_name, "root");
    assert_eq!(usage.calls[0].kind, UsageCallKind::Entrypoint);
    assert!(usage.calls[0].caller.is_none());
    let nested = usage
        .calls
        .iter()
        .find(|call| call.callee.function_name == "nested")
        .unwrap();
    assert_eq!(nested.kind, UsageCallKind::Call);
    assert_eq!(nested.caller.as_ref().unwrap().function_name, "root");
    let generic = usage
        .calls
        .iter()
        .find(|call| call.callee.function_name == "generic")
        .unwrap();
    assert_eq!(generic.kind, UsageCallKind::CallGeneric);
    let closure = usage
        .calls
        .iter()
        .find(|call| call.callee.function_name == "closure_target")
        .unwrap();
    assert_eq!(closure.kind, UsageCallKind::CallClosure);
}

fn is_function_called(coverage_path: &str, func_name: &str) -> bool {
    let map = CoverageMap::from_binary_file(&format!("{}.mvcov", coverage_path))
        .expect("coverage map file")
        .to_unified_exec_map();
    let module_addr = AccountAddress::from_hex_literal("0x815").unwrap();
    let module_id = ident_str!("m").to_owned();
    let module_map = map
        .module_maps
        .get(&(module_addr, module_id))
        .expect("module 0x815:m coverage");
    let fun_ident = Identifier::new_unchecked(func_name);
    module_map.function_maps.contains_key(&fun_ident)
}
