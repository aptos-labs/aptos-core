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
use aptos_gas_profiling::HtmlReportOptions;
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

    // Generate the HTML report with default options (auto-load trace)
    let report_dir = Path::new("gas-profiling").join("gas_profiling_report");
    gas_log
        .generate_html_report(&report_dir, "Gas Profiling Test Report".to_string())
        .expect("Failed to generate report");

    // Verify the HTML report
    let html = std::fs::read_to_string(report_dir.join("index.html")).expect("read report");

    // Verify trace.html is generated
    let trace_path = report_dir.join("assets").join("trace.html");
    assert!(trace_path.exists(), "trace.html should be generated");
    let trace_html = std::fs::read_to_string(&trace_path).expect("read trace.html");
    assert!(
        trace_html.contains("trace-dimensions"),
        "trace.html should contain postMessage code"
    );

    // Verify style.css is generated
    let style_path = report_dir.join("assets").join("style.css");
    assert!(style_path.exists(), "style.css should be generated");

    // Structural checks
    assert!(html.contains("<h2>Summary</h2>"));
    assert!(html.contains("Flamegraphs"));
    assert!(html.contains("Cost Break-down"));

    // Verify auto-load path: iframe should have src attribute (not lazy-loaded)
    assert!(
        html.contains(r#"<iframe id="trace-frame" src="assets/trace.html""#),
        "With default threshold, trace should auto-load via iframe src"
    );
    assert!(
        !html.contains("trace-is-large"),
        "With default threshold, trace should not be marked as large"
    );

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

    // Test lazy-load path with a very low threshold
    let lazy_report_dir = Path::new("gas-profiling").join("gas_profiling_report_lazy");
    gas_log
        .generate_html_report_with_options(
            &lazy_report_dir,
            "Gas Profiling Test Report (Lazy)".to_string(),
            HtmlReportOptions {
                trace_lazy_load_threshold: 1, // Force lazy loading
            },
        )
        .expect("Failed to generate lazy report");

    let lazy_html =
        std::fs::read_to_string(lazy_report_dir.join("index.html")).expect("read lazy report");

    // Verify lazy-load path: should have Load Trace button
    assert!(
        lazy_html.contains("Load Trace"),
        "With low threshold, should show Load Trace button"
    );
    assert!(
        lazy_html.contains("Large trace"),
        "With low threshold, should show large trace message"
    );
    assert!(
        lazy_html.contains("lines)"),
        "With low threshold, should include line count"
    );

    // Verify trace.html is also generated for lazy report
    assert!(
        lazy_report_dir.join("assets").join("trace.html").exists(),
        "trace.html should be generated for lazy report"
    );
}
