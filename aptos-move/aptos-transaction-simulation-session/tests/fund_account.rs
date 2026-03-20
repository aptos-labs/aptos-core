// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::Result;
use aptos_transaction_simulation::{Account, SimulationStateStore};
use aptos_transaction_simulation_session::Session;

#[test]
fn test_fund_account() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let mut session = Session::init(temp_dir.path())?;

    let account = Account::new();
    session.fund_account(*account.address(), 1_000_000)?;

    let balance = session.state_store().get_apt_balance(*account.address())?;
    assert_eq!(balance, 1_000_000);

    Ok(())
}

#[test]
fn test_fund_account_twice_accumulates() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let mut session = Session::init(temp_dir.path())?;

    let account = Account::new();
    session.fund_account(*account.address(), 500)?;
    session.fund_account(*account.address(), 300)?;

    let balance = session.state_store().get_apt_balance(*account.address())?;
    assert_eq!(balance, 800);

    Ok(())
}
