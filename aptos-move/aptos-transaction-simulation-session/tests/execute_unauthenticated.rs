// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::Result;
use aptos_crypto::{ed25519::Ed25519PrivateKey, PrivateKey, Uniform};
use aptos_transaction_simulation::{Account, SimulationStateStore};
use aptos_transaction_simulation_session::Session;
use aptos_types::{
    account_address::AccountAddress,
    account_config::AccountResource,
    chain_id::ChainId,
    transaction::{
        authenticator::AccountAuthenticator, EntryFunction, ExecutionStatus, RawTransaction,
        SignedTransaction, TransactionPayload, TransactionStatus,
    },
    vm_status::{StatusCode, VMStatus},
};
use move_core_types::{identifier::Identifier, language_storage::ModuleId};

const GAS_UNIT_PRICE: u64 = 100;
const MAX_GAS: u64 = 2_000_000;
const EXPIRATION: u64 = 4_000_000;

fn transfer_payload(recipient: AccountAddress, amount: u64) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
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
    ))
}

fn unauthenticated_transfer(
    sender: AccountAddress,
    recipient: AccountAddress,
    amount: u64,
    sequence_number: u64,
    chain_id: ChainId,
) -> SignedTransaction {
    let raw = RawTransaction::new(
        sender,
        sequence_number,
        transfer_payload(recipient, amount),
        MAX_GAS,
        GAS_UNIT_PRICE,
        EXPIRATION,
        chain_id,
    );
    SignedTransaction::new_single_sender(raw, AccountAuthenticator::NoAccountAuthenticator)
}

fn authenticated_transfer_with_key(
    sender_address: AccountAddress,
    signer_key: &Ed25519PrivateKey,
    recipient: AccountAddress,
    amount: u64,
    sequence_number: u64,
    chain_id: ChainId,
) -> SignedTransaction {
    let raw = RawTransaction::new(
        sender_address,
        sequence_number,
        transfer_payload(recipient, amount),
        MAX_GAS,
        GAS_UNIT_PRICE,
        EXPIRATION,
        chain_id,
    );
    raw.sign(signer_key, signer_key.public_key())
        .unwrap()
        .into_inner()
}

/// Fund creates an APT balance without storing a private key for the address.
/// `store_and_fund_account` is used so the Account resource (sequence number)
/// exists; the private key is discarded after setup and never used to sign.
fn fund_address_without_using_key(
    session: &mut Session,
    amount: u64,
) -> Result<AccountAddress> {
    let account = Account::new();
    let address = *account.address();
    session
        .state_store()
        .store_and_fund_account(account, amount, 0)?;
    // Drop the Account (and its key) — subsequent txs use NoAccountAuthenticator.
    Ok(address)
}

#[test]
fn test_unauthenticated_transfer_updates_balance_and_sequence() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let mut session = Session::init(temp_dir.path())?;

    let funded = 1_000_000_000u64;
    let transfer_amount = 1u64;
    let sender = fund_address_without_using_key(&mut session, funded)?;
    let chain_id = session.state_store().get_chain_id()?;

    let txn = unauthenticated_transfer(sender, AccountAddress::ONE, transfer_amount, 0, chain_id);
    let (vm_status, output) = session.execute_unauthenticated_transaction(txn)?;
    assert_eq!(vm_status, VMStatus::Executed);
    assert!(matches!(
        output.status(),
        TransactionStatus::Keep(ExecutionStatus::Success)
    ));

    let balance_after = session.state_store().get_apt_balance(sender)?;
    let gas_cost = output.gas_used() * GAS_UNIT_PRICE;
    assert_eq!(
        balance_after,
        funded - transfer_amount - gas_cost,
        "balance should be funded - 1 - gas"
    );

    let account_resource: AccountResource = session
        .state_store()
        .get_resource(sender)?
        .expect("account resource should exist");
    assert_eq!(account_resource.sequence_number(), 1);

    // Second unauthenticated transfer uses the incremented sequence number.
    let txn2 = unauthenticated_transfer(sender, AccountAddress::ONE, transfer_amount, 1, chain_id);
    let (vm_status2, output2) = session.execute_unauthenticated_transaction(txn2)?;
    assert_eq!(vm_status2, VMStatus::Executed);
    assert!(matches!(
        output2.status(),
        TransactionStatus::Keep(ExecutionStatus::Success)
    ));

    let balance_after_2 = session.state_store().get_apt_balance(sender)?;
    let gas_cost_2 = output2.gas_used() * GAS_UNIT_PRICE;
    assert_eq!(
        balance_after_2,
        balance_after - transfer_amount - gas_cost_2
    );

    let account_resource: AccountResource = session
        .state_store()
        .get_resource(sender)?
        .expect("account resource should exist");
    assert_eq!(account_resource.sequence_number(), 2);

    Ok(())
}

#[test]
fn test_unauthenticated_abort_keeps_gas_only() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let mut session = Session::init(temp_dir.path())?;

    let funded = 1_000_000_000u64;
    let sender = fund_address_without_using_key(&mut session, funded)?;
    let chain_id = session.state_store().get_chain_id()?;

    // Transfer more than the funded balance so the Move abort path is taken.
    // Simulation VM keeps Move aborts and charges gas; user-level transfer
    // amount must not be applied beyond the gas fee.
    let txn = unauthenticated_transfer(sender, AccountAddress::ONE, funded + 1, 0, chain_id);
    let (_vm_status, output) = session.execute_unauthenticated_transaction(txn)?;

    assert!(
        matches!(
            output.status(),
            TransactionStatus::Keep(ExecutionStatus::MoveAbort { .. })
        ),
        "expected Keep(MoveAbort), got {:?}",
        output.status()
    );

    let balance_after = session.state_store().get_apt_balance(sender)?;
    let gas_cost = output.gas_used() * GAS_UNIT_PRICE;
    assert_eq!(
        balance_after,
        funded - gas_cost,
        "Move abort should charge gas but not transfer the amount"
    );

    // Sequence number is bumped on kept (including abort) transactions.
    let account_resource: AccountResource = session
        .state_store()
        .get_resource(sender)?
        .expect("account resource should exist");
    assert_eq!(account_resource.sequence_number(), 1);

    Ok(())
}

#[test]
fn test_authenticated_wrong_key_still_invalid_auth_key() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let mut session = Session::init(temp_dir.path())?;

    let funded = 1_000_000_000u64;
    let sender = Account::new();
    let sender_address = *sender.address();
    session
        .state_store()
        .store_and_fund_account(sender, funded, 0)?;

    let wrong_key = Ed25519PrivateKey::generate(&mut rand::thread_rng());
    let chain_id = session.state_store().get_chain_id()?;
    let txn = authenticated_transfer_with_key(
        sender_address,
        &wrong_key,
        AccountAddress::ONE,
        1,
        0,
        chain_id,
    );

    let (vm_status, output) = session.execute_transaction(txn, false, false)?;
    assert!(
        matches!(
            output.status(),
            TransactionStatus::Discard(StatusCode::INVALID_AUTH_KEY)
        ) || matches!(
            vm_status,
            VMStatus::Error {
                status_code: StatusCode::INVALID_AUTH_KEY,
                ..
            }
        ),
        "expected INVALID_AUTH_KEY discard, got status={:?} vm_status={:?}",
        output.status(),
        vm_status
    );

    let balance_after = session.state_store().get_apt_balance(sender_address)?;
    assert_eq!(
        balance_after, funded,
        "discarded INVALID_AUTH_KEY must not pay the transfer (or gas)"
    );

    Ok(())
}

#[test]
fn test_simulate_transaction_does_not_mutate_session() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let mut session = Session::init(temp_dir.path())?;

    let funded = 1_000_000_000u64;
    let sender = fund_address_without_using_key(&mut session, funded)?;
    let chain_id = session.state_store().get_chain_id()?;

    let txn = unauthenticated_transfer(sender, AccountAddress::ONE, 1, 0, chain_id);
    let (vm_status, output) = session.simulate_transaction(txn)?;
    assert_eq!(vm_status, VMStatus::Executed);
    assert!(matches!(
        output.status(),
        TransactionStatus::Keep(ExecutionStatus::Success)
    ));
    assert!(output.gas_used() > 0);

    // Dry-run must leave balance and sequence number unchanged.
    assert_eq!(session.state_store().get_apt_balance(sender)?, funded);
    let account_resource: AccountResource = session
        .state_store()
        .get_resource(sender)?
        .expect("account resource should exist");
    assert_eq!(account_resource.sequence_number(), 0);

    Ok(())
}
