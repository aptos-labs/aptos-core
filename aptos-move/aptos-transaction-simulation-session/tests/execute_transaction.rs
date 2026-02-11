// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::Result;
use aptos_transaction_simulation::{Account, SimulationStateStore};
use aptos_transaction_simulation_session::Session;
use aptos_types::{
    account_address::AccountAddress,
    account_config::AccountResource,
    transaction::{EntryFunction, SignedTransaction, TransactionPayload},
    vm_status::VMStatus,
};
use move_core_types::{identifier::Identifier, language_storage::ModuleId};

/// Creates a signed transaction that calls `0x1::aptos_account::transfer`.
fn transfer_txn(sender: &Account, recipient: AccountAddress, amount: u64) -> SignedTransaction {
    let payload = TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(
            AccountAddress::ONE,
            Identifier::new("aptos_account").unwrap(),
        ),
        Identifier::new("transfer").unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&recipient).unwrap(),
            bcs::to_bytes(&amount).unwrap(),
        ],
    ));
    sender
        .transaction()
        .sequence_number(0)
        .gas_unit_price(100)
        .payload(payload)
        .sign()
}

#[test]
fn test_execute_transfer() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let mut session = Session::init(temp_dir.path())?;

    let sender = Account::new();
    let recipient = Account::new();

    // Create account and fund with 1 APT.
    session
        .state_store()
        .store_and_fund_account(sender.clone(), 100_000_000, 0)?;

    let txn = transfer_txn(&sender, *recipient.address(), 1_000);
    let (vm_status, output) = session.execute_transaction(txn, false)?;

    assert_eq!(vm_status, VMStatus::Executed, "transfer should succeed");
    assert!(output.gas_used() > 0, "should use some gas");

    Ok(())
}

#[test]
fn test_execute_transaction_increments_sequence_number() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let mut session = Session::init(temp_dir.path())?;

    let sender = Account::new();
    session
        .state_store()
        .store_and_fund_account(sender.clone(), 100_000_000, 0)?;

    let txn = transfer_txn(&sender, *sender.address(), 100);
    session.execute_transaction(txn, false)?;

    // After one transaction, sequence number should be 1.
    let account_resource: AccountResource = session
        .state_store()
        .get_resource(*sender.address())?
        .expect("account resource should exist");
    assert_eq!(account_resource.sequence_number(), 1);

    Ok(())
}

#[test]
fn test_execute_transaction_with_gas_profiling() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let session_path = temp_dir.path();
    let mut session = Session::init(session_path)?;

    let sender = Account::new();
    let recipient = Account::new();

    session
        .state_store()
        .store_and_fund_account(sender.clone(), 100_000_000, 0)?;

    let txn = transfer_txn(&sender, *recipient.address(), 1_000);
    let (vm_status, output) = session.execute_transaction(txn, true)?;

    assert_eq!(vm_status, VMStatus::Executed, "transfer should succeed");
    assert!(output.gas_used() > 0, "should use some gas");

    // Verify the gas-report directory was created under the transaction output.
    let gas_report_dir = session_path.join("[0] execute 0x1::aptos_account::transfer/gas-report");
    assert!(
        gas_report_dir.exists(),
        "gas-report directory should exist: {}",
        gas_report_dir.display()
    );
    assert!(
        gas_report_dir.join("index.html").exists(),
        "gas-report/index.html should exist"
    );
    assert!(
        gas_report_dir.join("assets").exists(),
        "gas-report/assets directory should exist"
    );

    Ok(())
}
