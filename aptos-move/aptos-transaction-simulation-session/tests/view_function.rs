// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::Result;
use aptos_transaction_simulation::{Account, SimulationStateStore};
use aptos_transaction_simulation_session::Session;
use aptos_types::account_address::AccountAddress;
use move_core_types::{identifier::Identifier, language_storage::ModuleId};

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
    );

    assert!(result.is_err(), "calling a nonexistent module should fail");

    Ok(())
}
