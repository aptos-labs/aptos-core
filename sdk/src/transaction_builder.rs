// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    move_types::account_address::AccountAddress,
    types::{
        chain_id::ChainId,
        transaction::{authenticator::AuthenticationKey, RawTransaction, TransactionPayload},
    },
};
use aptos_crypto::ed25519::Ed25519PublicKey;
use aptos_global_constants::{GAS_UNIT_PRICE, MAX_GAS_AMOUNT};
use aptos_types::transaction::{
    authenticator::AuthenticationKeyPreimage, EntryFunction, ModuleBundle, Script,
};
pub use cached_packages::aptos_stdlib;

pub struct TransactionBuilder {
    sender: Option<AccountAddress>,
    sequence_number: Option<u64>,
    payload: TransactionPayload,
    max_gas_amount: u64,
    gas_unit_price: u64,
    expiration_timestamp_secs: u64,
    chain_id: ChainId,
}

impl TransactionBuilder {
    pub fn new(
        payload: TransactionPayload,
        expiration_timestamp_secs: u64,
        chain_id: ChainId,
    ) -> Self {
        Self {
            payload,
            chain_id,
            expiration_timestamp_secs,
            // TODO(Gas): double check this
            max_gas_amount: MAX_GAS_AMOUNT,
            gas_unit_price: std::cmp::max(GAS_UNIT_PRICE, 1),
            sender: None,
            sequence_number: None,
        }
    }

    pub fn sender(mut self, sender: AccountAddress) -> Self {
        self.sender = Some(sender);
        self
    }

    pub fn sequence_number(mut self, sequence_number: u64) -> Self {
        self.sequence_number = Some(sequence_number);
        self
    }

    pub fn max_gas_amount(mut self, max_gas_amount: u64) -> Self {
        self.max_gas_amount = max_gas_amount;
        self
    }

    pub fn gas_unit_price(mut self, gas_unit_price: u64) -> Self {
        self.gas_unit_price = gas_unit_price;
        self
    }

    pub fn chain_id(mut self, chain_id: ChainId) -> Self {
        self.chain_id = chain_id;
        self
    }

    pub fn expiration_timestamp_secs(mut self, expiration_timestamp_secs: u64) -> Self {
        self.expiration_timestamp_secs = expiration_timestamp_secs;
        self
    }

    pub fn build(self) -> RawTransaction {
        RawTransaction::new(
            self.sender.expect("sender must have been set"),
            self.sequence_number
                .expect("sequence number must have been set"),
            self.payload,
            self.max_gas_amount,
            self.gas_unit_price,
            self.expiration_timestamp_secs,
            self.chain_id,
        )
    }
}

#[derive(Clone, Debug)]
pub struct TransactionFactory {
    max_gas_amount: u64,
    gas_unit_price: u64,
    transaction_expiration_time: u64,
    chain_id: ChainId,
}

impl TransactionFactory {
    pub fn new(chain_id: ChainId) -> Self {
        Self {
            // TODO(Gas): double check if this right
            max_gas_amount: MAX_GAS_AMOUNT,
            gas_unit_price: GAS_UNIT_PRICE,
            transaction_expiration_time: 30,
            chain_id,
        }
    }

    pub fn with_max_gas_amount(mut self, max_gas_amount: u64) -> Self {
        self.max_gas_amount = max_gas_amount;
        self
    }

    pub fn with_gas_unit_price(mut self, gas_unit_price: u64) -> Self {
        self.gas_unit_price = gas_unit_price;
        self
    }

    pub fn with_transaction_expiration_time(mut self, transaction_expiration_time: u64) -> Self {
        self.transaction_expiration_time = transaction_expiration_time;
        self
    }

    pub fn with_chain_id(mut self, chain_id: ChainId) -> Self {
        self.chain_id = chain_id;
        self
    }

    pub fn payload(&self, payload: TransactionPayload) -> TransactionBuilder {
        self.transaction_builder(payload)
    }

    pub fn module(&self, code: Vec<u8>) -> TransactionBuilder {
        self.payload(TransactionPayload::ModuleBundle(ModuleBundle::singleton(
            code,
        )))
    }

    pub fn entry_function(&self, func: EntryFunction) -> TransactionBuilder {
        self.payload(TransactionPayload::EntryFunction(func))
    }

    pub fn create_user_account(&self, public_key: &Ed25519PublicKey) -> TransactionBuilder {
        let preimage = AuthenticationKeyPreimage::ed25519(public_key);
        self.payload(aptos_stdlib::aptos_account_create_account(
            AuthenticationKey::from_preimage(&preimage).derived_address(),
        ))
    }

    pub fn implicitly_create_user_account_and_transfer(
        &self,
        public_key: &Ed25519PublicKey,
        amount: u64,
    ) -> TransactionBuilder {
        let preimage = AuthenticationKeyPreimage::ed25519(public_key);
        self.payload(aptos_stdlib::aptos_account_transfer(
            AuthenticationKey::from_preimage(&preimage).derived_address(),
            amount,
        ))
    }

    pub fn transfer(&self, to: AccountAddress, amount: u64) -> TransactionBuilder {
        self.payload(aptos_stdlib::aptos_coin_transfer(to, amount))
    }

    pub fn mint(&self, to: AccountAddress, amount: u64) -> TransactionBuilder {
        self.payload(aptos_stdlib::aptos_coin_mint(to, amount))
    }

    //
    // Internal Helpers
    //

    pub fn script(&self, script: Script) -> TransactionBuilder {
        self.payload(TransactionPayload::Script(script))
    }

    fn transaction_builder(&self, payload: TransactionPayload) -> TransactionBuilder {
        TransactionBuilder {
            sender: None,
            sequence_number: None,
            payload,
            max_gas_amount: self.max_gas_amount,
            gas_unit_price: self.gas_unit_price,
            expiration_timestamp_secs: self.expiration_timestamp(),
            chain_id: self.chain_id,
        }
    }

    fn expiration_timestamp(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + self.transaction_expiration_time
    }
}

pub struct DualAttestationMessage {
    message: Box<[u8]>,
}

impl DualAttestationMessage {
    pub fn new<M: Into<Vec<u8>>>(metadata: M, reciever: AccountAddress, amount: u64) -> Self {
        let mut message = metadata.into();
        bcs::serialize_into(&mut message, &reciever).unwrap();
        bcs::serialize_into(&mut message, &amount).unwrap();
        message.extend(b"@@$$APTOS_ATTEST$$@@");

        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &[u8] {
        &self.message
    }
}
