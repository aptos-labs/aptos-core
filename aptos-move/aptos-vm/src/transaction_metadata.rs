// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::{ed25519::Ed25519PrivateKey, HashValue, PrivateKey};
use aptos_gas::{FeePerGasUnit, Gas, NumBytes};
use aptos_types::transaction::authenticator::AuthenticationKey;
use aptos_types::{
    account_address::AccountAddress,
    chain_id::ChainId,
    transaction::{SignedTransaction, TransactionPayload},
};
use std::convert::TryFrom;

pub struct TransactionMetadata {
    pub sender: AccountAddress,
    pub authentication_key: Vec<u8>,
    pub secondary_signers: Vec<AccountAddress>,
    pub secondary_authentication_keys: Vec<Vec<u8>>,
    pub sequence_number: u64,
    pub max_gas_amount: Gas,
    pub gas_unit_price: FeePerGasUnit,
    pub transaction_size: NumBytes,
    pub expiration_timestamp_secs: u64,
    pub chain_id: ChainId,
    pub script_hash: Vec<u8>,
}

impl TransactionMetadata {
    pub fn new(txn: &SignedTransaction) -> Self {
        Self {
            sender: txn.sender(),
            authentication_key: txn.authenticator().sender().authentication_key().to_vec(),
            secondary_signers: txn.authenticator().secondary_signer_addreses(),
            secondary_authentication_keys: txn
                .authenticator()
                .secondary_signers()
                .iter()
                .map(|account_auth| account_auth.authentication_key().to_vec())
                .collect(),
            sequence_number: txn.sequence_number(),
            max_gas_amount: txn.max_gas_amount().into(),
            gas_unit_price: txn.gas_unit_price().into(),
            transaction_size: (txn.raw_txn_bytes_len() as u64).into(),
            expiration_timestamp_secs: txn.expiration_timestamp_secs(),
            chain_id: txn.chain_id(),
            script_hash: match txn.payload() {
                TransactionPayload::Script(s) => HashValue::sha3_256_of(s.code()).to_vec(),
                TransactionPayload::EntryFunction(_) => vec![],
                TransactionPayload::ModuleBundle(_) => vec![],
            },
        }
    }

    pub fn max_gas_amount(&self) -> Gas {
        self.max_gas_amount
    }

    pub fn gas_unit_price(&self) -> FeePerGasUnit {
        self.gas_unit_price
    }

    pub fn sender(&self) -> AccountAddress {
        self.sender.to_owned()
    }

    pub fn secondary_signers(&self) -> Vec<AccountAddress> {
        self.secondary_signers.to_owned()
    }

    pub fn authentication_key(&self) -> &[u8] {
        &self.authentication_key
    }

    pub fn sequence_number(&self) -> u64 {
        self.sequence_number
    }

    pub fn transaction_size(&self) -> NumBytes {
        self.transaction_size
    }

    pub fn expiration_timestamp_secs(&self) -> u64 {
        self.expiration_timestamp_secs
    }

    pub fn chain_id(&self) -> ChainId {
        self.chain_id
    }

    pub fn is_multi_agent(&self) -> bool {
        !self.secondary_signers.is_empty()
    }
}

impl Default for TransactionMetadata {
    fn default() -> Self {
        let mut buf = [0u8; Ed25519PrivateKey::LENGTH];
        buf[Ed25519PrivateKey::LENGTH - 1] = 1;
        let public_key = Ed25519PrivateKey::try_from(&buf[..]).unwrap().public_key();
        TransactionMetadata {
            sender: AccountAddress::ZERO,
            authentication_key: AuthenticationKey::ed25519(&public_key).to_vec(),
            secondary_signers: vec![],
            secondary_authentication_keys: vec![],
            sequence_number: 0,
            max_gas_amount: 100_000_000.into(),
            gas_unit_price: 0.into(),
            transaction_size: 0.into(),
            expiration_timestamp_secs: 0,
            chain_id: ChainId::test(),
            script_hash: vec![],
        }
    }
}
