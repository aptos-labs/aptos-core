// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::Result;
use aptos_transaction_simulation::{Account, SimulationStateStore};
use aptos_transaction_simulation_session::Session;
use aptos_types::account_address::AccountAddress;
use move_core_types::{identifier::Identifier, language_storage::ModuleId};
use std::path::Path;

#[test]
fn test_view_function_get_sequence_number() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let mut session = Session::init(temp_dir.path())?;

    let account = Account::new();
    session
        .state_store()
        .store_and_fund_account(account.clone(), 0, 42)?;

    let result = session.execute_view_function(
        ModuleId::new(AccountAddress::ONE, Identifier::new("account")?),
        Identifier::new("get_sequence_number")?,
        vec![],
        vec![bcs::to_bytes(account.address())?],
        false,
    )?;

    assert_eq!(result.len(), 1);
    // The view function returns a JSON string of the u64 value.
    assert_eq!(result[0], serde_json::json!("42"));

    Ok(())
}

#[test]
fn test_view_function_nonexistent_module() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let mut session = Session::init(temp_dir.path())?;

    let result = session.execute_view_function(
        ModuleId::new(AccountAddress::ONE, Identifier::new("nonexistent_module")?),
        Identifier::new("some_function")?,
        vec![],
        vec![],
        false,
    );

    assert!(result.is_err(), "calling a nonexistent module should fail");

    Ok(())
}

#[test]
fn test_view_function_with_gas_profiling() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let mut session = Session::init(temp_dir.path())?;

    let account = Account::new();
    session
        .state_store()
        .store_and_fund_account(account.clone(), 0, 42)?;

    let result = session.execute_view_function(
        ModuleId::new(AccountAddress::ONE, Identifier::new("account")?),
        Identifier::new("get_sequence_number")?,
        vec![],
        vec![bcs::to_bytes(account.address())?],
        true,
    )?;

    // The view function should still return correct results when profiling.
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], serde_json::json!("42"));

    // A gas-report directory should have been generated under the session output.
    let report_dir = temp_dir
        .path()
        .join("[0] view 0x1::account::get_sequence_number")
        .join("gas-report");
    assert!(
        report_dir.is_dir(),
        "gas-report directory should exist at {}",
        report_dir.display()
    );
    assert!(
        has_html_files(&report_dir),
        "gas-report should contain HTML files"
    );

    Ok(())
}

/// Returns true if the directory (or any subdirectory) contains at least one `.html` file.
fn has_html_files(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "html") {
            return true;
        }
        if path.is_dir() && has_html_files(&path) {
            return true;
        }
    }
    false
}
