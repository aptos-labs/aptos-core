// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    move_types::account_address::AccountAddress,
    types::{
        chain_id::ChainId,
        transaction::{authenticator::AuthenticationKey, RawTransaction, TransactionPayload},
    },
};
pub use aptos_cached_packages::aptos_stdlib;
use aptos_crypto::{ed25519::Ed25519PublicKey, HashValue};
use aptos_global_constants::{GAS_UNIT_PRICE, MAX_GAS_AMOUNT};
use aptos_types::{
    function_info::FunctionInfo,
    transaction::{EntryFunction, Script},
};
use rand::{Rng, thread_rng};

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

    pub fn has_nonce(&self) -> bool {
        self.payload.replay_protection_nonce().is_some()
    }

    pub fn upgrade_payload_with_rng(
        mut self,
        rng: &mut impl Rng,
        use_txn_payload_v2_format: bool,
        use_orderless_transactions: bool,
    ) -> Self {
        self.payload = self.payload.upgrade_payload_with_rng(
            rng,
            use_txn_payload_v2_format,
            use_orderless_transactions,
        );
        if use_orderless_transactions {
            self.sequence_number = self.sequence_number.map(|_| u64::MAX);
        }
        self
    }

    pub fn build(self) -> RawTransaction {
        let sequence_number = if self.has_nonce() {
            u64::MAX
        } else {
            self.sequence_number
                .expect("sequence number must have been set")
        };
        RawTransaction::new(
            self.sender.expect("sender must have been set"),
            sequence_number,
            self.payload,
            self.max_gas_amount,
            self.gas_unit_price,
            self.expiration_timestamp_secs,
            self.chain_id,
        )
    }
}

#[derive(Clone, Debug)]
enum TransactionExpiration {
    Relative { expiration_duration: u64 },
    Absolute { expiration_timestamp: u64 },
}

#[derive(Clone, Debug)]
pub struct TransactionFactory {
    max_gas_amount: u64,
    gas_unit_price: u64,
    transaction_expiration: TransactionExpiration,
    chain_id: ChainId,

    use_txn_payload_v2_format: bool,
    use_replay_protection_nonce: bool,
}

impl TransactionFactory {
    pub fn new(chain_id: ChainId) -> Self {
        Self {
            // TODO(Gas): double check if this right
            max_gas_amount: MAX_GAS_AMOUNT,
            gas_unit_price: GAS_UNIT_PRICE,
            transaction_expiration: TransactionExpiration::Relative {
                expiration_duration: 30,
            },
            chain_id,
            use_txn_payload_v2_format: false,
            use_replay_protection_nonce: false,
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
        self.transaction_expiration = TransactionExpiration::Relative {
            expiration_duration: transaction_expiration_time,
        };
        self
    }

    /// Sets the transaction expiration timestamp to an absolute value.
    /// Generally only useful for testing.
    pub fn with_absolute_transaction_expiration_timestamp(
        mut self,
        transaction_expiration_timestamp: u64,
    ) -> Self {
        self.transaction_expiration = TransactionExpiration::Absolute {
            expiration_timestamp: transaction_expiration_timestamp,
        };
        self
    }

    pub fn with_chain_id(mut self, chain_id: ChainId) -> Self {
        self.chain_id = chain_id;
        self
    }

    pub fn get_max_gas_amount(&self) -> u64 {
        self.max_gas_amount
    }

    pub fn get_gas_unit_price(&self) -> u64 {
        self.gas_unit_price
    }

    /// Returns relative duration of transaction expiration.
    pub fn get_transaction_expiration_time(&self) -> u64 {
        match self.transaction_expiration {
            TransactionExpiration::Relative {
                expiration_duration,
            } => expiration_duration,
            TransactionExpiration::Absolute { .. } => {
                unimplemented!()
            },
        }
    }

    pub fn get_chain_id(&self) -> ChainId {
        self.chain_id
    }

    pub fn payload(&self, payload: TransactionPayload) -> TransactionBuilder {
        self.transaction_builder(payload)
    }

    pub fn entry_function(&self, func: EntryFunction) -> TransactionBuilder {
        // TODO[Orderless]: Change this to use TransactionPayload::Payload once it's available
        self.payload(TransactionPayload::EntryFunction(func))
    }

    pub fn create_user_account(&self, public_key: &Ed25519PublicKey) -> TransactionBuilder {
        self.payload(aptos_stdlib::aptos_account_create_account(
            AuthenticationKey::ed25519(public_key).account_address(),
        ))
    }

    pub fn add_dispatchable_authentication_function(
        &self,
        function_info: FunctionInfo,
    ) -> TransactionBuilder {
        self.payload(
            aptos_stdlib::account_abstraction_add_authentication_function(
                function_info.module_address,
                function_info.module_name.into_bytes(),
                function_info.function_name.into_bytes(),
            ),
        )
    }

    pub fn implicitly_create_user_account_and_transfer(
        &self,
        public_key: &Ed25519PublicKey,
        amount: u64,
    ) -> TransactionBuilder {
        self.payload(aptos_stdlib::aptos_account_transfer(
            AuthenticationKey::ed25519(public_key).account_address(),
            amount,
        ))
    }

    pub fn transfer(&self, to: AccountAddress, amount: u64) -> TransactionBuilder {
        self.payload(aptos_stdlib::aptos_coin_transfer(to, amount))
    }

    pub fn account_transfer(&self, to: AccountAddress, amount: u64) -> TransactionBuilder {
        self.payload(aptos_stdlib::aptos_account_transfer(to, amount))
    }

    pub fn create_multisig_account(
        &self,
        additional_owners: Vec<AccountAddress>,
        signatures_required: u64,
    ) -> TransactionBuilder {
        self.payload(aptos_stdlib::multisig_account_create_with_owners(
            additional_owners,
            signatures_required,
            vec![],
            vec![],
        ))
    }

    pub fn create_multisig_account_with_existing_account(
        &self,
        owners: Vec<AccountAddress>,
        signatures_required: u64,
    ) -> TransactionBuilder {
        self.payload(
            aptos_stdlib::multisig_account_create_with_existing_account_call(
                owners,
                signatures_required,
                vec![],
                vec![],
            ),
        )
    }

    pub fn create_multisig_account_with_existing_account_and_revoke_auth_key(
        &self,
        owners: Vec<AccountAddress>,
        signatures_required: u64,
    ) -> TransactionBuilder {
        self.payload(
            aptos_stdlib::multisig_account_create_with_existing_account_and_revoke_auth_key_call(
                owners,
                signatures_required,
                vec![],
                vec![],
            ),
        )
    }

    pub fn create_multisig_transaction(
        &self,
        multisig_account: AccountAddress,
        payload: Vec<u8>,
    ) -> TransactionBuilder {
        self.payload(aptos_stdlib::multisig_account_create_transaction(
            multisig_account,
            payload,
        ))
    }

    pub fn approve_multisig_transaction(
        &self,
        multisig_account: AccountAddress,
        transaction_id: u64,
    ) -> TransactionBuilder {
        self.payload(aptos_stdlib::multisig_account_approve_transaction(
            multisig_account,
            transaction_id,
        ))
    }

    pub fn reject_multisig_transaction(
        &self,
        multisig_account: AccountAddress,
        transaction_id: u64,
    ) -> TransactionBuilder {
        self.payload(aptos_stdlib::multisig_account_reject_transaction(
            multisig_account,
            transaction_id,
        ))
    }

    pub fn create_multisig_transaction_with_payload_hash(
        &self,
        multisig_account: AccountAddress,
        payload: Vec<u8>,
    ) -> TransactionBuilder {
        self.payload(aptos_stdlib::multisig_account_create_transaction_with_hash(
            multisig_account,
            HashValue::sha3_256_of(&payload).to_vec(),
        ))
    }

    pub fn mint(&self, to: AccountAddress, amount: u64) -> TransactionBuilder {
        self.payload(aptos_stdlib::aptos_coin_mint(to, amount))
    }

    //
    // Internal Helpers
    //

    pub fn script(&self, script: Script) -> TransactionBuilder {
        // TODO[Orderless]: Change this to use TransactionPayload::Payload once it's available
        self.payload(TransactionPayload::Script(script))
    }

    fn transaction_builder(&self, payload: TransactionPayload) -> TransactionBuilder {
        TransactionBuilder {
            sender: None,
            sequence_number: None,
            payload: if self.use_txn_payload_v2_format || self.use_replay_protection_nonce {
                payload.upgrade_payload_with_fn(
                    self.use_txn_payload_v2_format,
                    self.use_replay_protection_nonce.then_some(|| thread_rng().gen()),
                )
            } else {
                payload
            },
            max_gas_amount: self.max_gas_amount,
            gas_unit_price: self.gas_unit_price,
            expiration_timestamp_secs: self.expiration_timestamp(),
            chain_id: self.chain_id,
        }
    }

    fn expiration_timestamp(&self) -> u64 {
        match self.transaction_expiration {
            TransactionExpiration::Relative {
                expiration_duration,
            } => {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    + expiration_duration
            },
            TransactionExpiration::Absolute {
                expiration_timestamp,
            } => expiration_timestamp,
        }
    }
}
