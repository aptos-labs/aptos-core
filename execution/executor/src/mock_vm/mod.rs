// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod mock_vm_test;

use aptos_crypto::{ed25519::Ed25519PrivateKey, PrivateKey, Uniform};
use aptos_state_view::StateView;
use aptos_types::{
    access_path::AccessPath,
    account_address::AccountAddress,
    account_config::CORE_CODE_ADDRESS,
    chain_id::ChainId,
    contract_event::ContractEvent,
    event::EventKey,
    on_chain_config::{
        access_path_for_config, new_epoch_event_key, ConfigurationResource, OnChainConfig,
        ValidatorSet,
    },
    state_store::state_key::StateKey,
    transaction::{
        ChangeSet, ExecutionStatus, RawTransaction, Script, SignedTransaction, Transaction,
        TransactionArgument, TransactionOutput, TransactionPayload, TransactionStatus,
        WriteSetPayload,
    },
    vm_status::{StatusCode, VMStatus},
    write_set::{WriteOp, WriteSet, WriteSetMut},
};
use aptos_vm::VMExecutor;
use move_deps::move_core_types::{language_storage::TypeTag, move_resource::MoveResource};
use once_cell::sync::Lazy;
use std::collections::HashMap;

#[derive(Debug)]
enum MockVMTransaction {
    Mint {
        sender: AccountAddress,
        amount: u64,
    },
    Payment {
        sender: AccountAddress,
        recipient: AccountAddress,
        amount: u64,
    },
}

pub static KEEP_STATUS: Lazy<TransactionStatus> =
    Lazy::new(|| TransactionStatus::Keep(ExecutionStatus::Success));

// We use 10 as the assertion error code for insufficient balance within the Aptos coin contract.
pub static DISCARD_STATUS: Lazy<TransactionStatus> =
    Lazy::new(|| TransactionStatus::Discard(StatusCode::INSUFFICIENT_BALANCE_FOR_TRANSACTION_FEE));

pub struct MockVM;

impl VMExecutor for MockVM {
    fn execute_block(
        transactions: Vec<Transaction>,
        state_view: &impl StateView,
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        if state_view.is_genesis() {
            assert_eq!(
                transactions.len(),
                1,
                "Genesis block should have only one transaction."
            );
            let output = TransactionOutput::new(
                gen_genesis_writeset(),
                // mock the validator set event
                vec![ContractEvent::new(
                    new_epoch_event_key(),
                    0,
                    TypeTag::Bool,
                    bcs::to_bytes(&0).unwrap(),
                )],
                0,
                KEEP_STATUS.clone(),
            );
            return Ok(vec![output]);
        }

        // output_cache is used to store the output of transactions so they are visible to later
        // transactions.
        let mut output_cache = HashMap::new();
        let mut outputs = vec![];

        for txn in transactions {
            if matches!(txn, Transaction::StateCheckpoint(_)) {
                outputs.push(TransactionOutput::new(
                    WriteSet::default(),
                    vec![],
                    0,
                    KEEP_STATUS.clone(),
                ));
                continue;
            }

            if matches!(txn, Transaction::GenesisTransaction(_)) {
                read_state_value_from_storage(
                    state_view,
                    &access_path_for_config(ValidatorSet::CONFIG_ID),
                );
                read_state_value_from_storage(
                    state_view,
                    &AccessPath::new(CORE_CODE_ADDRESS, ConfigurationResource::resource_path()),
                );
                outputs.push(TransactionOutput::new(
                    // WriteSet cannot be empty so use genesis writeset only for testing.
                    gen_genesis_writeset(),
                    // mock the validator set event
                    vec![ContractEvent::new(
                        new_epoch_event_key(),
                        0,
                        TypeTag::Bool,
                        bcs::to_bytes(&0).unwrap(),
                    )],
                    0,
                    KEEP_STATUS.clone(),
                ));
                continue;
            }

            match decode_transaction(txn.as_signed_user_txn().unwrap()) {
                MockVMTransaction::Mint { sender, amount } => {
                    let old_balance = read_balance(&output_cache, state_view, sender);
                    let new_balance = old_balance + amount;
                    let old_seqnum = read_seqnum(&output_cache, state_view, sender);
                    let new_seqnum = old_seqnum + 1;

                    output_cache.insert(balance_ap(sender), new_balance);
                    output_cache.insert(seqnum_ap(sender), new_seqnum);

                    let write_set = gen_mint_writeset(sender, new_balance, new_seqnum);
                    let events = gen_events(sender);
                    outputs.push(TransactionOutput::new(
                        write_set,
                        events,
                        0,
                        KEEP_STATUS.clone(),
                    ));
                }
                MockVMTransaction::Payment {
                    sender,
                    recipient,
                    amount,
                } => {
                    let sender_old_balance = read_balance(&output_cache, state_view, sender);
                    let recipient_old_balance = read_balance(&output_cache, state_view, recipient);
                    if sender_old_balance < amount {
                        outputs.push(TransactionOutput::new(
                            WriteSet::default(),
                            vec![],
                            0,
                            DISCARD_STATUS.clone(),
                        ));
                        continue;
                    }

                    let sender_old_seqnum = read_seqnum(&output_cache, state_view, sender);
                    let sender_new_seqnum = sender_old_seqnum + 1;
                    let sender_new_balance = sender_old_balance - amount;
                    let recipient_new_balance = recipient_old_balance + amount;

                    output_cache.insert(balance_ap(sender), sender_new_balance);
                    output_cache.insert(seqnum_ap(sender), sender_new_seqnum);
                    output_cache.insert(balance_ap(recipient), recipient_new_balance);

                    let write_set = gen_payment_writeset(
                        sender,
                        sender_new_balance,
                        sender_new_seqnum,
                        recipient,
                        recipient_new_balance,
                    );
                    let events = gen_events(sender);
                    outputs.push(TransactionOutput::new(
                        write_set,
                        events,
                        0,
                        TransactionStatus::Keep(ExecutionStatus::Success),
                    ));
                }
            }
        }

        Ok(outputs)
    }
}

fn read_balance(
    output_cache: &HashMap<AccessPath, u64>,
    state_view: &impl StateView,
    account: AccountAddress,
) -> u64 {
    let balance_access_path = balance_ap(account);
    match output_cache.get(&balance_access_path) {
        Some(balance) => *balance,
        None => read_balance_from_storage(state_view, &balance_access_path),
    }
}

fn read_seqnum(
    output_cache: &HashMap<AccessPath, u64>,
    state_view: &impl StateView,
    account: AccountAddress,
) -> u64 {
    let seqnum_access_path = seqnum_ap(account);
    match output_cache.get(&seqnum_access_path) {
        Some(seqnum) => *seqnum,
        None => read_seqnum_from_storage(state_view, &seqnum_access_path),
    }
}

fn read_balance_from_storage(state_view: &impl StateView, balance_access_path: &AccessPath) -> u64 {
    read_u64_from_storage(state_view, balance_access_path)
}

fn read_seqnum_from_storage(state_view: &impl StateView, seqnum_access_path: &AccessPath) -> u64 {
    read_u64_from_storage(state_view, seqnum_access_path)
}

fn read_u64_from_storage(state_view: &impl StateView, access_path: &AccessPath) -> u64 {
    state_view
        .get_state_value(&StateKey::AccessPath(access_path.clone()))
        .expect("Failed to query storage.")
        .map_or(0, |bytes| decode_bytes(&bytes))
}

fn read_state_value_from_storage(
    state_view: &impl StateView,
    access_path: &AccessPath,
) -> Option<Vec<u8>> {
    state_view
        .get_state_value(&StateKey::AccessPath(access_path.clone()))
        .expect("Failed to query storage.")
}

fn decode_bytes(bytes: &[u8]) -> u64 {
    let mut buf = [0; 8];
    buf.copy_from_slice(bytes);
    u64::from_le_bytes(buf)
}

fn balance_ap(account: AccountAddress) -> AccessPath {
    AccessPath::new(account, b"balance".to_vec())
}

fn seqnum_ap(account: AccountAddress) -> AccessPath {
    AccessPath::new(account, b"seqnum".to_vec())
}

fn gen_genesis_writeset() -> WriteSet {
    let mut write_set = WriteSetMut::default();
    let validator_set_ap = access_path_for_config(ValidatorSet::CONFIG_ID);
    write_set.insert((
        StateKey::AccessPath(validator_set_ap),
        WriteOp::Modification(bcs::to_bytes(&ValidatorSet::new(vec![])).unwrap()),
    ));
    write_set.insert((
        StateKey::AccessPath(AccessPath::new(
            CORE_CODE_ADDRESS,
            ConfigurationResource::resource_path(),
        )),
        WriteOp::Modification(bcs::to_bytes(&ConfigurationResource::default()).unwrap()),
    ));
    write_set
        .freeze()
        .expect("genesis writeset should be valid")
}

fn gen_mint_writeset(sender: AccountAddress, balance: u64, seqnum: u64) -> WriteSet {
    let mut write_set = WriteSetMut::default();
    write_set.insert((
        StateKey::AccessPath(balance_ap(sender)),
        WriteOp::Modification(balance.to_le_bytes().to_vec()),
    ));
    write_set.insert((
        StateKey::AccessPath(seqnum_ap(sender)),
        WriteOp::Modification(seqnum.to_le_bytes().to_vec()),
    ));
    write_set.freeze().expect("mint writeset should be valid")
}

fn gen_payment_writeset(
    sender: AccountAddress,
    sender_balance: u64,
    sender_seqnum: u64,
    recipient: AccountAddress,
    recipient_balance: u64,
) -> WriteSet {
    let mut write_set = WriteSetMut::default();
    write_set.insert((
        StateKey::AccessPath(balance_ap(sender)),
        WriteOp::Modification(sender_balance.to_le_bytes().to_vec()),
    ));
    write_set.insert((
        StateKey::AccessPath(seqnum_ap(sender)),
        WriteOp::Modification(sender_seqnum.to_le_bytes().to_vec()),
    ));
    write_set.insert((
        StateKey::AccessPath(balance_ap(recipient)),
        WriteOp::Modification(recipient_balance.to_le_bytes().to_vec()),
    ));
    write_set
        .freeze()
        .expect("payment write set should be valid")
}

fn gen_events(sender: AccountAddress) -> Vec<ContractEvent> {
    vec![ContractEvent::new(
        EventKey::new(111, sender),
        0,
        TypeTag::Vector(Box::new(TypeTag::U8)),
        b"event_data".to_vec(),
    )]
}

pub fn encode_mint_program(amount: u64) -> Script {
    let argument = TransactionArgument::U64(amount);
    Script::new(vec![], vec![], vec![argument])
}

pub fn encode_transfer_program(recipient: AccountAddress, amount: u64) -> Script {
    let argument1 = TransactionArgument::Address(recipient);
    let argument2 = TransactionArgument::U64(amount);
    Script::new(vec![], vec![], vec![argument1, argument2])
}

pub fn encode_mint_transaction(sender: AccountAddress, amount: u64) -> Transaction {
    encode_transaction(sender, encode_mint_program(amount))
}

pub fn encode_transfer_transaction(
    sender: AccountAddress,
    recipient: AccountAddress,
    amount: u64,
) -> Transaction {
    encode_transaction(sender, encode_transfer_program(recipient, amount))
}

fn encode_transaction(sender: AccountAddress, program: Script) -> Transaction {
    let raw_transaction = RawTransaction::new_script(sender, 0, program, 0, 0, 0, ChainId::test());

    let privkey = Ed25519PrivateKey::generate_for_testing();
    Transaction::UserTransaction(
        raw_transaction
            .sign(&privkey, privkey.public_key())
            .expect("Failed to sign raw transaction.")
            .into_inner(),
    )
}

pub fn encode_reconfiguration_transaction() -> Transaction {
    Transaction::GenesisTransaction(WriteSetPayload::Direct(ChangeSet::new(
        WriteSet::default(),
        vec![],
    )))
}

fn decode_transaction(txn: &SignedTransaction) -> MockVMTransaction {
    let sender = txn.sender();
    match txn.payload() {
        TransactionPayload::Script(script) => {
            assert!(script.code().is_empty(), "Code should be empty.");
            match script.args().len() {
                1 => match script.args()[0] {
                    TransactionArgument::U64(amount) => MockVMTransaction::Mint { sender, amount },
                    _ => unimplemented!(
                        "Only one integer argument is allowed for mint transactions."
                    ),
                },
                2 => match (&script.args()[0], &script.args()[1]) {
                    (TransactionArgument::Address(recipient), TransactionArgument::U64(amount)) => {
                        MockVMTransaction::Payment {
                            sender,
                            recipient: *recipient,
                            amount: *amount,
                        }
                    }
                    _ => unimplemented!(
                        "The first argument for payment transaction must be recipient address \
                         and the second argument must be amount."
                    ),
                },
                _ => unimplemented!("Transaction must have one or two arguments."),
            }
        }
        TransactionPayload::EntryFunction(_) => {
            // TODO: we need to migrate Script to EntryFunction later
            unimplemented!("MockVM does not support entry function transaction payload.")
        }
        TransactionPayload::ModuleBundle(_) => {
            unimplemented!("MockVM does not support Module transaction payload.")
        }
    }
}
