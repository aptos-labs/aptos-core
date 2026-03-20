// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::Result;
use aptos_transaction_simulation::{Account, SimulationStateStore};
use aptos_transaction_simulation_session::Session;
use aptos_types::account_address::AccountAddress;
use move_core_types::language_storage::StructTag;
use std::str::FromStr;

#[test]
fn test_view_resource_exists() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let mut session = Session::init(temp_dir.path())?;

    let account = Account::new();
    session
        .state_store()
        .store_and_fund_account(account.clone(), 0, 0)?;

    let tag = StructTag::from_str("0x1::account::Account")?;
    let result = session.view_resource(*account.address(), &tag)?;

    assert!(result.is_some(), "account resource should exist");

    Ok(())
}

#[test]
fn test_view_resource_not_found() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let mut session = Session::init(temp_dir.path())?;

    let tag = StructTag::from_str("0x1::account::Account")?;
    // Random address that has no account.
    let result = session.view_resource(AccountAddress::from_hex_literal("0x12345")?, &tag)?;

    assert!(
        result.is_none(),
        "resource should not exist for random address"
    );

    Ok(())
}

#[test]
fn test_view_resource_group_for_funded_account() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let mut session = Session::init(temp_dir.path())?;

    // Fund an account — this creates an ObjectGroup with a FungibleStore at
    // the primary APT store address (derived from account address + 0xA).
    let account = Account::new();
    session.fund_account(*account.address(), 1_000_000)?;

    let group_tag = StructTag::from_str("0x1::object::ObjectGroup")?;
    let result =
        session.view_resource_group(*account.address(), &group_tag, Some(AccountAddress::TEN))?;

    assert!(
        result.is_some(),
        "ObjectGroup should exist at the primary APT store address"
    );

    Ok(())
}
