// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod mock_vm_test;

use anyhow::Result;
use aptos_block_executor::txn_provider::{default::DefaultTxnProvider, TxnProvider};
use aptos_crypto::{ed25519::Ed25519PrivateKey, PrivateKey, Uniform};
use aptos_types::{
    account_address::AccountAddress,
    account_config::NEW_EPOCH_EVENT_V2_MOVE_TYPE_TAG,
    block_executor::{
        config::BlockExecutorConfigFromOnchain, partitioner::PartitionedTransactions,
        transaction_slice_metadata::TransactionSliceMetadata,
    },
    bytes::NumToBytes,
    chain_id::ChainId,
    contract_event::ContractEvent,
    event::EventKey,
    on_chain_config::{ConfigurationResource, ValidatorSet},
    state_store::{state_key::StateKey, StateView},
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, BlockEndInfo, BlockOutput,
        ChangeSet, ExecutionStatus, RawTransaction, Script, SignedTransaction, Transaction,
        TransactionArgument, TransactionAuxiliaryData, TransactionExecutableRef, TransactionOutput,
        TransactionStatus, WriteSetPayload,
    },
    vm_status::{StatusCode, VMStatus},
    write_set::{WriteOp, WriteSet, WriteSetMut},
};
use aptos_vm::{
    sharded_block_executor::{executor_client::ExecutorClient, ShardedBlockExecutor},
    VMBlockExecutor,
};
use move_core_types::language_storage::TypeTag;
use once_cell::sync::Lazy;
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

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

pub static DISCARD_STATUS: Lazy<TransactionStatus> =
    Lazy::new(|| TransactionStatus::Discard(StatusCode::INSUFFICIENT_BALANCE_FOR_TRANSACTION_FEE));

pub static RETRY_STATUS: Lazy<TransactionStatus> = Lazy::new(|| TransactionStatus::Retry);

pub struct MockVM;

impl VMBlockExecutor for MockVM {
    fn new() -> Self {
        Self
    }

    fn execute_block(
        &self,
        txn_provider: &DefaultTxnProvider<SignatureVerifiedTransaction>,
        state_view: &impl StateView,
        _onchain_config: BlockExecutorConfigFromOnchain,
        transaction_slice_metadata: TransactionSliceMetadata,
    ) -> Result<BlockOutput<SignatureVerifiedTransaction, TransactionOutput>, VMStatus> {
        // output_cache is used to store the output of transactions so they are visible to later
        // transactions.
        let mut output_cache = HashMap::new();
        let mut outputs = vec![];

        let mut skip_rest = false;
        for idx in 0..txn_provider.num_txns() {
            if skip_rest {
                outputs.push(TransactionOutput::new(
                    WriteSet::default(),
                    vec![],
                    0,
                    RETRY_STATUS.clone(),
                    TransactionAuxiliaryData::default(),
                ));
                continue;
            }

            let txn = txn_provider.get_txn(idx as u32).expect_valid();
            if matches!(
                txn,
                Transaction::StateCheckpoint(_) | Transaction::BlockEpilogue(_)
            ) {
                outputs.push(TransactionOutput::new(
                    WriteSet::default(),
                    vec![],
                    0,
                    KEEP_STATUS.clone(),
                    TransactionAuxiliaryData::default(),
                ));
                continue;
            }

            if matches!(txn, Transaction::GenesisTransaction(_)) {
                read_state_value_from_storage(
                    state_view,
                    &StateKey::on_chain_config::<ValidatorSet>().unwrap(),
                );
                read_state_value_from_storage(
                    state_view,
                    &StateKey::on_chain_config::<ConfigurationResource>().unwrap(),
                );
                outputs.push(TransactionOutput::new(
                    // WriteSet cannot be empty so use genesis writeset only for testing.
                    gen_genesis_writeset(),
                    // mock the validator set event
                    vec![ContractEvent::new_v2(
                        NEW_EPOCH_EVENT_V2_MOVE_TYPE_TAG.clone(),
                        bcs::to_bytes(&0).unwrap(),
                    )
                    .unwrap()],
                    0,
                    KEEP_STATUS.clone(),
                    TransactionAuxiliaryData::default(),
                ));
                skip_rest = true;
                continue;
            }

            match decode_transaction(txn.try_as_signed_user_txn().unwrap()) {
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
                        TransactionAuxiliaryData::default(),
                    ));
                },
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
                            TransactionAuxiliaryData::default(),
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
                        TransactionAuxiliaryData::default(),
                    ));
                },
            }
        }

        let mut block_epilogue_txn = None;
        if !skip_rest {
            if let Some(block_id) = transaction_slice_metadata.append_state_checkpoint_to_block() {
                block_epilogue_txn = Some(Transaction::block_epilogue_v0(
                    block_id,
                    BlockEndInfo::new_empty(),
                ));
                outputs.push(TransactionOutput::new_empty_success());
            }
        }

        Ok(BlockOutput::new(
            outputs,
            block_epilogue_txn.map(Into::into),
            BTreeMap::new(),
        ))
    }

    fn execute_block_sharded<S: StateView + Sync + Send + 'static, E: ExecutorClient<S>>(
        _sharded_block_executor: &ShardedBlockExecutor<S, E>,
        _transactions: PartitionedTransactions,
        _state_view: Arc<S>,
        _onchain_config: BlockExecutorConfigFromOnchain,
    ) -> std::result::Result<Vec<TransactionOutput>, VMStatus> {
        todo!()
    }
}

fn read_balance(
    output_cache: &HashMap<Vec<u8>, u64>,
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
    output_cache: &HashMap<Vec<u8>, u64>,
    state_view: &impl StateView,
    account: AccountAddress,
) -> u64 {
    let seqnum_access_path = seqnum_ap(account);
    match output_cache.get(&seqnum_access_path) {
        Some(seqnum) => *seqnum,
        None => read_seqnum_from_storage(state_view, &seqnum_access_path),
    }
}

fn read_balance_from_storage(state_view: &impl StateView, balance_access_path: &[u8]) -> u64 {
    read_u64_from_storage(state_view, balance_access_path)
}

fn read_seqnum_from_storage(state_view: &impl StateView, seqnum_access_path: &[u8]) -> u64 {
    read_u64_from_storage(state_view, seqnum_access_path)
}

fn read_u64_from_storage(state_view: &impl StateView, access_path: &[u8]) -> u64 {
    state_view
        .get_state_value_bytes(&StateKey::raw(access_path))
        .expect("Failed to query storage.")
        .map_or(0, |bytes| decode_bytes(&bytes))
}

fn read_state_value_from_storage(
    state_view: &impl StateView,
    state_key: &StateKey,
) -> Option<Vec<u8>> {
    state_view
        .get_state_value_bytes(state_key)
        .expect("Failed to query storage.")
        .map(|bytes| bytes.to_vec())
}

fn decode_bytes(bytes: &[u8]) -> u64 {
    let mut buf = [0; 8];
    buf.copy_from_slice(bytes);
    u64::from_le_bytes(buf)
}

fn balance_ap(account: AccountAddress) -> Vec<u8> {
    let mut path = account.to_vec();
    path.extend(b"balance");
    path
}

fn seqnum_ap(account: AccountAddress) -> Vec<u8> {
    let mut path = account.to_vec();
    path.extend(b"seqnum");
    path
}

fn gen_genesis_writeset() -> WriteSet {
    let mut write_set = WriteSetMut::default();
    write_set.insert((
        StateKey::on_chain_config::<ValidatorSet>().unwrap(),
        WriteOp::legacy_modification(bcs::to_bytes(&ValidatorSet::new(vec![])).unwrap().into()),
    ));
    write_set.insert((
        StateKey::on_chain_config::<ConfigurationResource>().unwrap(),
        WriteOp::legacy_modification(
            bcs::to_bytes(&ConfigurationResource::default())
                .unwrap()
                .into(),
        ),
    ));
    write_set
        .freeze()
        .expect("genesis writeset should be valid")
}

fn gen_mint_writeset(sender: AccountAddress, balance: u64, seqnum: u64) -> WriteSet {
    let mut write_set = WriteSetMut::default();
    write_set.insert((
        StateKey::raw(&balance_ap(sender)),
        WriteOp::legacy_modification(balance.le_bytes()),
    ));
    write_set.insert((
        StateKey::raw(&seqnum_ap(sender)),
        WriteOp::legacy_modification(seqnum.le_bytes()),
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
        StateKey::raw(&balance_ap(sender)),
        WriteOp::legacy_modification(sender_balance.le_bytes()),
    ));
    write_set.insert((
        StateKey::raw(&seqnum_ap(sender)),
        WriteOp::legacy_modification(sender_seqnum.le_bytes()),
    ));
    write_set.insert((
        StateKey::raw(&balance_ap(recipient)),
        WriteOp::legacy_modification(recipient_balance.le_bytes()),
    ));
    write_set
        .freeze()
        .expect("payment write set should be valid")
}

fn gen_events(sender: AccountAddress) -> Vec<ContractEvent> {
    vec![ContractEvent::new_v1(
        EventKey::new(111, sender),
        0,
        TypeTag::Vector(Box::new(TypeTag::U8)),
        b"event_data".to_vec(),
    )
    .unwrap()]
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
    let script_to_mock_vm_txn = |script: &Script| {
        assert!(script.code().is_empty(), "Code should be empty.");
        match script.args().len() {
            1 => match script.args()[0] {
                TransactionArgument::U64(amount) => MockVMTransaction::Mint { sender, amount },
                _ => unimplemented!("Only one integer argument is allowed for mint transactions."),
            },
            2 => match (&script.args()[0], &script.args()[1]) {
                (TransactionArgument::Address(recipient), TransactionArgument::U64(amount)) => {
                    MockVMTransaction::Payment {
                        sender,
                        recipient: *recipient,
                        amount: *amount,
                    }
                },
                _ => unimplemented!(
                    "The first argument for payment transaction must be recipient address \
                        and the second argument must be amount."
                ),
            },
            _ => unimplemented!("Transaction must have one or two arguments."),
        }
    };
    match txn.payload().executable_ref() {
        Ok(TransactionExecutableRef::Script(script)) => script_to_mock_vm_txn(script),
        Ok(TransactionExecutableRef::EntryFunction(_)) => {
            unimplemented!("MockVM does not support multisig transaction payload.")
        },
        Ok(TransactionExecutableRef::Empty) => {
            unimplemented!("MockVM does not support empty transaction payload.")
        },
        Err(_) => unimplemented!("MockVM does not support given transaction payload."),
    }
}
