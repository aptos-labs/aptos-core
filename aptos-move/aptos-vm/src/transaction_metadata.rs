// Copyright (c) 2024 Supra.
// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::HashValue;
use aptos_gas_algebra::{FeePerGasUnit, Gas, NumBytes};
use aptos_types::transaction::automated_transaction::AutomatedTransaction;
use aptos_types::{
    account_address::AccountAddress,
    chain_id::ChainId,
    transaction::{
        user_transaction_context::UserTransactionContext, EntryFunction, Multisig,
        SignedTransaction, TransactionPayload,
    },
};
use aptos_types::transaction::user_transaction_context::{PayloadTypeReference, PayloadTypeReferenceContext};

pub type PayloadTypeReferenceMeta = PayloadTypeReference<EntryFunction, Multisig>;
pub struct TransactionMetadata {
    pub sender: AccountAddress,
    pub authentication_key: Vec<u8>,
    pub secondary_signers: Vec<AccountAddress>,
    pub secondary_authentication_keys: Vec<Vec<u8>>,
    pub sequence_number: u64,
    pub fee_payer: Option<AccountAddress>,
    pub fee_payer_authentication_key: Option<Vec<u8>>,
    pub max_gas_amount: Gas,
    pub gas_unit_price: FeePerGasUnit,
    pub transaction_size: NumBytes,
    pub expiration_timestamp_secs: u64,
    pub chain_id: ChainId,
    pub script_hash: Vec<u8>,
    pub script_size: NumBytes,
    pub is_keyless: bool,
    pub payload_type_reference: PayloadTypeReferenceMeta,
    pub txn_app_hash: Vec<u8>,
}

impl TransactionMetadata {
    pub fn new(txn: &SignedTransaction) -> Self {
        let payload_type_reference = match txn.payload() {
            TransactionPayload::Script(_) |
            TransactionPayload::ModuleBundle(_) => PayloadTypeReferenceMeta::Other,
            TransactionPayload::EntryFunction(e) => PayloadTypeReferenceMeta::UserEntryFunction(e.clone()),
            TransactionPayload::Multisig(m) => PayloadTypeReferenceMeta::Multisig(m.clone()),
            TransactionPayload::AutomationRegistration(_) => PayloadTypeReferenceMeta::AutomationRegistration,
        };
        Self {
            sender: txn.sender(),
            authentication_key: txn.authenticator().sender().authentication_key().to_vec(),
            secondary_signers: txn.authenticator().secondary_signer_addresses(),
            secondary_authentication_keys: txn
                .authenticator()
                .secondary_signers()
                .iter()
                .map(|account_auth| account_auth.authentication_key().to_vec())
                .collect(),
            sequence_number: txn.sequence_number(),
            fee_payer: txn.authenticator_ref().fee_payer_address(),
            fee_payer_authentication_key: txn
                .authenticator()
                .fee_payer_signer()
                .map(|signer| signer.authentication_key().to_vec()),
            max_gas_amount: txn.max_gas_amount().into(),
            gas_unit_price: txn.gas_unit_price().into(),
            transaction_size: (txn.raw_txn_bytes_len() as u64).into(),
            expiration_timestamp_secs: txn.expiration_timestamp_secs(),
            chain_id: txn.chain_id(),
            script_hash: match txn.payload() {
                TransactionPayload::Script(s) => HashValue::sha3_256_of(s.code()).to_vec(),
                TransactionPayload::EntryFunction(_) => vec![],
                TransactionPayload::Multisig(_) => vec![],

                // Deprecated. Return an empty vec because we cannot do anything
                // else here, only `unreachable!` otherwise.
                TransactionPayload::ModuleBundle(_) => vec![],
                TransactionPayload::AutomationRegistration(_) => vec![],
            },
            script_size: match txn.payload() {
                TransactionPayload::Script(s) => (s.code().len() as u64).into(),
                _ => NumBytes::zero(),
            },
            is_keyless: aptos_types::keyless::get_authenticators(txn)
                .map(|res| !res.is_empty())
                .unwrap_or(false),
            payload_type_reference,
            txn_app_hash: HashValue::keccak_256_of(
                &bcs::to_bytes(&txn).expect("Unable to serialize SignedTransaction"),
            )
            .to_vec(),
        }
    }

    pub fn max_gas_amount(&self) -> Gas {
        self.max_gas_amount
    }

    pub fn gas_unit_price(&self) -> FeePerGasUnit {
        self.gas_unit_price
    }

    pub fn fee_payer(&self) -> Option<AccountAddress> {
        self.fee_payer.to_owned()
    }

    pub fn script_size(&self) -> NumBytes {
        self.script_size
    }

    pub fn sender(&self) -> AccountAddress {
        self.sender.to_owned()
    }

    pub fn secondary_signers(&self) -> Vec<AccountAddress> {
        self.secondary_signers.to_owned()
    }

    pub fn senders(&self) -> Vec<AccountAddress> {
        let mut senders = vec![self.sender()];
        senders.extend(self.secondary_signers());
        senders
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
        !self.secondary_signers.is_empty() || self.fee_payer.is_some()
    }

    pub fn is_keyless(&self) -> bool {
        self.is_keyless
    }

    pub fn entry_function_payload(&self) -> Option<EntryFunction> {
        self.payload_type_reference.entry_function_payload()
    }

    pub fn multisig_payload(&self) -> Option<Multisig> {
        self.payload_type_reference.multisig_payload()
    }

    pub fn as_user_transaction_context(&self) -> UserTransactionContext {
        let payload_type_reference = match &self.payload_type_reference {
            PayloadTypeReferenceMeta::Other => PayloadTypeReferenceContext::Other,
            PayloadTypeReferenceMeta::UserEntryFunction(e) => PayloadTypeReferenceContext::UserEntryFunction(e.as_entry_function_payload()),
            PayloadTypeReferenceMeta::Multisig(m) => PayloadTypeReferenceContext::Multisig(m.as_multisig_payload()),
            PayloadTypeReferenceMeta::AutomationRegistration => PayloadTypeReferenceContext::AutomationRegistration,
        };
        UserTransactionContext::new(
            self.sender,
            self.secondary_signers.clone(),
            self.fee_payer.unwrap_or(self.sender),
            self.max_gas_amount.into(),
            self.gas_unit_price.into(),
            self.chain_id.id(),
            payload_type_reference,
        )
    }
}

impl From<&AutomatedTransaction> for TransactionMetadata {
    fn from(txn: &AutomatedTransaction) -> Self {
        Self {
            sender: txn.sender(),
            authentication_key: txn.authenticator().to_vec(),
            secondary_signers: vec![],
            secondary_authentication_keys: vec![],
            sequence_number: txn.sequence_number(),
            fee_payer: None,
            fee_payer_authentication_key: None,
            max_gas_amount: txn.max_gas_amount().into(),
            gas_unit_price: txn.gas_unit_price().into(),
            transaction_size: (txn.raw_txn_bytes_len() as u64).into(),
            expiration_timestamp_secs: txn.expiration_timestamp_secs(),
            chain_id: txn.chain_id(),
            script_hash: vec![],
            script_size: NumBytes::zero(),
            is_keyless: false,
            payload_type_reference: PayloadTypeReferenceMeta::UserEntryFunction(txn.payload().clone().into_entry_function()),
            txn_app_hash: txn.hash().to_vec(),
        }
    }
}
