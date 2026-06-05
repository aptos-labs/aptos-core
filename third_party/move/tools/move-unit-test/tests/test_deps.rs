// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
    value::MoveValue,
};
use move_unit_test::{self, test_reporter::UnitTestFactoryWithCostTable, UnitTestingConfig};
use std::{fs, path::PathBuf};
use tempfile::tempdir;

const TWO_ROW_SOURCE: &str = r#"
    address 0x1 {
    module M {
        #[test(addr = @0x1)]
        #[test(addr = @0x2)]
        fun foo(addr: signer) {
            let _ = addr;
        }
    }
    }
"#;

fn build_test_plan_from_source(source: &str) -> legacy_move_compiler::unit_test::TestPlan {
    let temp = tempdir().unwrap();
    let source_path = temp.path().join("case_identity.move");
    fs::write(&source_path, source).unwrap();

    let mut config = UnitTestingConfig::default()
        .with_named_addresses(move_stdlib::move_stdlib_named_addresses());
    config.source_files = vec![source_path.to_string_lossy().into_owned()];
    config.dep_files = move_stdlib::move_stdlib_files();
    config.build_test_plan().unwrap()
}

fn run_source(source: &str, filter: Option<&str>, report_statistics: bool) -> String {
    let plan = build_test_plan_from_source(source);
    let config = UnitTestingConfig {
        filter: filter.map(str::to_string),
        num_threads: 1,
        report_statistics,
        ..UnitTestingConfig::default()
    };
    let (output, ok) = config
        .run_and_report_unit_tests(
            plan,
            None,
            None,
            Vec::new(),
            UnitTestFactoryWithCostTable::new(None, None),
            false,
            false,
        )
        .unwrap();
    assert!(ok);
    String::from_utf8(output).unwrap()
}

// Make sure the compiled bytecode for dependencies is included, but the tests in them are not run.
#[test]
fn test_deps_arent_tested() {
    let mut testing_config = UnitTestingConfig::default()
        .with_named_addresses(move_stdlib::move_stdlib_named_addresses());
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let a_path = path.join("tests/sources/A.move");
    let b_path = path.join("tests/sources/B.move");
    let mut deps = move_stdlib::move_stdlib_files();
    deps.push(a_path.to_string_lossy().to_string());

    testing_config.source_files = vec![b_path.to_str().unwrap().to_owned()];
    testing_config.dep_files = deps;

    let test_plan = testing_config.build_test_plan().unwrap();

    let mut iter = test_plan.module_tests.into_iter();
    let (mod_id, _) = iter.next().unwrap();
    let expected_mod_id = ModuleId::new(
        AccountAddress::from_hex_literal("0x1").unwrap(),
        Identifier::new("B").unwrap(),
    );
    assert!(mod_id == expected_mod_id);
    assert!(iter.next().is_none());
}

#[test]
fn parametric_rows_separate_case_and_function_identity_in_source_order() {
    let source = r#"
        address 0x1 {
        module M {
            #[test(addr = @0x0)]
            #[test(addr = @0x1)]
            #[test(addr = @0x2)]
            #[test(addr = @0x3)]
            #[test(addr = @0x4)]
            #[test(addr = @0x5)]
            #[test(addr = @0x6)]
            #[test(addr = @0x7)]
            #[test(addr = @0x8)]
            #[test(addr = @0x9)]
            #[test(addr = @0xa)]
            fun ordered(addr: signer) {
                let _ = addr;
            }
        }
        }
    "#;

    let plan = build_test_plan_from_source(source);
    let module = plan.module_tests.values().next().unwrap();

    assert_eq!(module.tests.len(), 11);
    for index in 0..11 {
        let test_case = module.tests.get(&format!("ordered@row{index}")).unwrap();
        assert_eq!(test_case.function_name, "ordered");
        assert_eq!(test_case.arguments, vec![MoveValue::Signer(
            AccountAddress::from_hex_literal(&format!("0x{index:x}")).unwrap()
        )]);
    }
}

#[test]
fn single_row_keeps_unsuffixed_case_and_function_identity() {
    let source = r#"
        address 0x1 {
        module M {
            #[test(addr = @0x1)]
            fun single(addr: signer) {
                let _ = addr;
            }
        }
        }
    "#;

    let plan = build_test_plan_from_source(source);
    let module = plan.module_tests.values().next().unwrap();
    let test_case = module.tests.get("single").unwrap();

    assert_eq!(module.tests.len(), 1);
    assert_eq!(test_case.function_name, "single");
}

#[test]
fn parametric_case_filter_selects_one_row() {
    let output = run_source(TWO_ROW_SOURCE, Some("foo@row1"), false);
    assert!(output.contains("::foo@row1"));
    assert!(!output.contains("::foo@row0"));
    assert!(output.contains("Total tests: 1; passed: 1; failed: 0"));
}

#[test]
fn plain_function_name_filter_selects_all_rows() {
    let output = run_source(TWO_ROW_SOURCE, Some("foo"), false);
    assert!(output.contains("::foo@row0"));
    assert!(output.contains("::foo@row1"));
    assert!(output.contains("Total tests: 2; passed: 2; failed: 0"));
}

#[test]
fn partial_row_suffix_filter_matches_nothing() {
    let output = run_source(TWO_ROW_SOURCE, Some("foo@row"), false);
    assert!(!output.contains("::foo@row0"));
    assert!(!output.contains("::foo@row1"));
    assert!(output.contains("Total tests: 0"));
}

#[test]
fn parametric_statistics_use_case_identity() {
    let output = run_source(TWO_ROW_SOURCE, None, true);
    let statistics = output.split("Test Statistics:").nth(1).unwrap();

    assert!(statistics.contains("::foo@row0"));
    assert!(statistics.contains("::foo@row1"));
    assert_eq!(statistics.matches("::foo@row").count(), 2);
}
