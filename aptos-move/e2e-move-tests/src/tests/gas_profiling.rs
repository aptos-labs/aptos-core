// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for gas profiling and report generation.
//!
//! Run with:
//!   RUST_MIN_STACK=8388608 cargo test -p e2e-move-tests test_gas_profiling -- --nocapture
//!
//! The test generates an HTML report that can be viewed in a browser at:
//!   aptos-move/e2e-move-tests/gas-profiling/gas_profiling_report/index.html

use crate::{assert_success, tests::common::test_dir_path, MoveHarness};
use aptos_types::{
    account_address::AccountAddress,
    transaction::{EntryFunction, TransactionPayload},
};
use move_core_types::{ident_str, language_storage::ModuleId};
use std::path::Path;

/// Extracts the numeric value from a summary table row in the HTML report.
/// Looks for pattern: <td class="category-label">{label}</td> followed by <td ...>{value}</td>
fn extract_summary_value(html: &str, label: &str) -> Option<f64> {
    let label_tag = format!("<td class=\"category-label\">{}</td>", label);
    let label_pos = html.find(&label_tag)?;
    let after_label = &html[label_pos + label_tag.len()..];
    let td_start = after_label.find("<td")? + 3;
    let after_td_open = &after_label[td_start..];
    let content_start = after_td_open.find('>')? + 1;
    let content = &after_td_open[content_start..];
    let content_end = content.find("</td>")?;
    content[..content_end].trim().parse().ok()
}

/// Test that exercises the gas profiler with a custom Move module.
///
/// This test runs a single transaction that:
/// - Has non-system dependencies (custom module)
/// - Emits events
/// - Writes to storage
/// - Deletes storage (triggers refunds)
///
/// To view the report, open in a browser:
///   aptos-move/e2e-move-tests/gas-profiling/gas_profiling_report/index.html
#[test]
fn test_gas_profiling_report() {
    let mut harness = MoveHarness::new();

    let mod_addr = AccountAddress::from_hex_literal("0xcafe").unwrap();
    let account = harness.new_account_at(mod_addr);

    // Publish the test module
    assert_success!(harness.publish_package(&account, &test_dir_path("gas_profiling.data/pack")));

    // Setup: Create a counter (not profiled)
    assert_success!(harness.run_entry_function(
        &account,
        str::parse("0xcafe::gas_profiling_test::setup").unwrap(),
        vec![],
        vec![],
    ));

    // Profiled transaction: replace counter (delete + create = refund + write), emit event
    let module_id = ModuleId::new(mod_addr, ident_str!("gas_profiling_test").to_owned());
    let (gas_log, _gas_used, _fee_statement) = harness.evaluate_gas_with_profiler(
        &account,
        TransactionPayload::EntryFunction(EntryFunction::new(
            module_id,
            ident_str!("replace").to_owned(),
            vec![],
            vec![],
        )),
    );

    // Generate the HTML report
    let report_dir = Path::new("gas-profiling").join("gas_profiling_report");
    gas_log
        .generate_html_report(&report_dir, "Gas Profiling Test Report".to_string())
        .expect("Failed to generate report");

    // Verify the HTML report
    let html = std::fs::read_to_string(report_dir.join("index.html")).expect("read report");

    // Structural checks
    assert!(html.contains("<h2>Summary</h2>"));
    assert!(html.contains("Flamegraphs"));
    assert!(html.contains("Cost Break-down"));

    // Verify gas values from HTML are > 0
    let execution_gas = extract_summary_value(&html, "Execution Gas").unwrap();
    assert!(execution_gas > 0.0, "Execution Gas should be > 0");

    let io_gas = extract_summary_value(&html, "IO Gas").unwrap();
    assert!(io_gas > 0.0, "IO Gas should be > 0");

    let storage_fee = extract_summary_value(&html, "Storage Fee").unwrap();
    assert!(storage_fee > 0.0, "Storage Fee should be > 0");

    let storage_refund = extract_summary_value(&html, "Storage Refund").unwrap();
    assert!(storage_refund > 0.0, "Storage Refund should be > 0");

    // Verify dependencies section shows our custom module
    assert!(
        html.contains("gas_profiling_test"),
        "Report should show custom module dependency"
    );

    // Verify events section exists (our transaction emits an event)
    assert!(html.contains("Events"), "Report should have Events section");
}
