// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_address::AccountAddress,
    block_executor::config::BlockExecutorConfigFromOnchain,
    chain_id::ChainId,
    transaction::{
        authenticator::AccountAuthenticator,
        signature_verified_transaction::{
            into_signature_verified_block, SignatureVerifiedTransaction,
        },
        RawTransaction, RawTransactionWithData, Script, SignedTransaction, Transaction,
        TransactionPayload,
    },
};
use velor_crypto::{ed25519::*, traits::*};

const MAX_GAS_AMOUNT: u64 = 1_000_000;
const TEST_GAS_PRICE: u64 = 100;

// The block executor onchain config (gas limit parameters) for executor tests
pub const TEST_BLOCK_EXECUTOR_ONCHAIN_CONFIG: BlockExecutorConfigFromOnchain =
    BlockExecutorConfigFromOnchain::on_but_large_for_test();

static EMPTY_SCRIPT: &[u8] = include_bytes!("empty_script.mv");

// Create an expiration time 'seconds' after now
fn expiration_time(seconds: u64) -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .expect("System time is before the UNIX_EPOCH")
        .as_secs()
        + seconds
}

// Test helper for transaction creation
pub fn get_test_signed_transaction(
    sender: AccountAddress,
    sequence_number: u64,
    private_key: &Ed25519PrivateKey,
    public_key: Ed25519PublicKey,
    payload: Option<TransactionPayload>,
    expiration_timestamp_secs: u64,
    gas_unit_price: u64,
    max_gas_amount: Option<u64>,
) -> SignedTransaction {
    let raw_txn = RawTransaction::new(
        sender,
        sequence_number,
        payload.unwrap_or_else(|| {
            TransactionPayload::Script(Script::new(EMPTY_SCRIPT.to_vec(), vec![], vec![]))
        }),
        max_gas_amount.unwrap_or(MAX_GAS_AMOUNT),
        gas_unit_price,
        expiration_timestamp_secs,
        ChainId::test(),
    );

    let signature = private_key.sign(&raw_txn).unwrap();

    SignedTransaction::new(raw_txn, public_key, signature)
}

// Test helper for creating transactions for which the signature hasn't been checked.
pub fn get_test_unchecked_transaction(
    sender: AccountAddress,
    sequence_number: u64,
    private_key: &Ed25519PrivateKey,
    public_key: Ed25519PublicKey,
    payload: TransactionPayload,
    expiration_time: u64,
    gas_unit_price: u64,
    max_gas_amount: Option<u64>,
) -> SignedTransaction {
    get_test_unchecked_transaction_(
        sender,
        sequence_number,
        private_key,
        public_key,
        payload,
        expiration_time,
        gas_unit_price,
        max_gas_amount,
        ChainId::test(),
    )
}

// Test helper for creating transactions for which the signature hasn't been checked.
fn get_test_unchecked_transaction_(
    sender: AccountAddress,
    sequence_number: u64,
    private_key: &Ed25519PrivateKey,
    public_key: Ed25519PublicKey,
    payload: TransactionPayload,
    expiration_timestamp_secs: u64,
    gas_unit_price: u64,
    max_gas_amount: Option<u64>,
    chain_id: ChainId,
) -> SignedTransaction {
    let raw_txn = RawTransaction::new(
        sender,
        sequence_number,
        payload,
        max_gas_amount.unwrap_or(MAX_GAS_AMOUNT),
        gas_unit_price,
        expiration_timestamp_secs,
        chain_id,
    );

    let signature = private_key.sign(&raw_txn).unwrap();

    SignedTransaction::new(raw_txn, public_key, signature)
}

// Test helper for transaction creation. Short version for get_test_signed_transaction
// Omits some fields
pub fn get_test_signed_txn(
    sender: AccountAddress,
    sequence_number: u64,
    private_key: &Ed25519PrivateKey,
    public_key: Ed25519PublicKey,
    payload: Option<TransactionPayload>,
) -> SignedTransaction {
    let expiration_time = expiration_time(10);
    get_test_signed_transaction(
        sender,
        sequence_number,
        private_key,
        public_key,
        payload,
        expiration_time,
        TEST_GAS_PRICE,
        None,
    )
}

pub fn get_test_unchecked_txn(
    sender: AccountAddress,
    sequence_number: u64,
    private_key: &Ed25519PrivateKey,
    public_key: Ed25519PublicKey,
    payload: TransactionPayload,
) -> SignedTransaction {
    let expiration_time = expiration_time(10);
    get_test_unchecked_transaction(
        sender,
        sequence_number,
        private_key,
        public_key,
        payload,
        expiration_time,
        TEST_GAS_PRICE,
        None,
    )
}

pub fn get_test_unchecked_multi_agent_txn(
    sender: AccountAddress,
    secondary_signers: Vec<AccountAddress>,
    sequence_number: u64,
    sender_private_key: &Ed25519PrivateKey,
    sender_public_key: Ed25519PublicKey,
    secondary_private_keys: Vec<&Ed25519PrivateKey>,
    secondary_public_keys: Vec<Ed25519PublicKey>,
    script: Option<Script>,
) -> SignedTransaction {
    let expiration_time = expiration_time(10);
    let raw_txn = RawTransaction::new(
        sender,
        sequence_number,
        TransactionPayload::Script(
            script.unwrap_or_else(|| Script::new(EMPTY_SCRIPT.to_vec(), vec![], Vec::new())),
        ),
        MAX_GAS_AMOUNT,
        TEST_GAS_PRICE,
        expiration_time,
        ChainId::test(),
    );
    let message =
        RawTransactionWithData::new_multi_agent(raw_txn.clone(), secondary_signers.clone());

    let sender_signature = sender_private_key.sign(&message).unwrap();
    let sender_authenticator = AccountAuthenticator::ed25519(sender_public_key, sender_signature);

    let mut secondary_authenticators = vec![];
    for i in 0..secondary_public_keys.len() {
        let signature = secondary_private_keys[i].sign(&message).unwrap();
        secondary_authenticators.push(AccountAuthenticator::ed25519(
            secondary_public_keys[i].clone(),
            signature,
        ));
    }

    SignedTransaction::new_multi_agent(
        raw_txn,
        sender_authenticator,
        secondary_signers,
        secondary_authenticators,
    )
}

pub fn get_test_txn_with_chain_id(
    sender: AccountAddress,
    sequence_number: u64,
    private_key: &Ed25519PrivateKey,
    public_key: Ed25519PublicKey,
    chain_id: ChainId,
) -> SignedTransaction {
    let expiration_time = expiration_time(10);
    let raw_txn = RawTransaction::new_script(
        sender,
        sequence_number,
        Script::new(EMPTY_SCRIPT.to_vec(), vec![], Vec::new()),
        MAX_GAS_AMOUNT,
        TEST_GAS_PRICE,
        expiration_time,
        chain_id,
    );

    let signature = private_key.sign(&raw_txn).unwrap();

    SignedTransaction::new(raw_txn, public_key, signature)
}

pub fn block(user_txns: Vec<Transaction>) -> Vec<SignatureVerifiedTransaction> {
    into_signature_verified_block(user_txns)
}

pub fn get_test_raw_transaction(
    sender: AccountAddress,
    sequence_number: u64,
    payload: Option<TransactionPayload>,
    expiration_timestamp_secs: Option<u64>,
    gas_unit_price: Option<u64>,
    max_gas_amount: Option<u64>,
) -> RawTransaction {
    RawTransaction::new(
        sender,
        sequence_number,
        payload.unwrap_or_else(|| {
            TransactionPayload::Script(Script::new(EMPTY_SCRIPT.to_vec(), vec![], vec![]))
        }),
        max_gas_amount.unwrap_or(MAX_GAS_AMOUNT),
        gas_unit_price.unwrap_or(TEST_GAS_PRICE),
        expiration_timestamp_secs.unwrap_or(expiration_time(10)),
        ChainId::test(),
    )
}
