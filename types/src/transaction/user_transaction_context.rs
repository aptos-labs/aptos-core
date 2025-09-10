// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::ScheduledTxnConfig;
use move_core_types::account_address::AccountAddress;

#[derive(Debug)]
pub struct UserTransactionContext {
    sender: AccountAddress,
    secondary_signers: Vec<AccountAddress>,
    gas_payer: AccountAddress,
    max_gas_amount: u64,
    gas_unit_price: u64,
    chain_id: u8,
    entry_function_payload: Option<EntryFunctionPayload>,
    multisig_payload: Option<MultisigPayload>,
    disallow_module_publishing: bool,
    payload_config: Option<PayloadConfig>,
}

#[derive(Debug, Clone)]
pub struct PayloadConfig {
    pub multisig_address: Option<AccountAddress>,
    pub replay_protection_nonce: Option<u64>,
    pub scheduled_txn_auth_token: Option<ScheduledTxnConfig>,
}

impl UserTransactionContext {
    pub fn new(
        sender: AccountAddress,
        secondary_signers: Vec<AccountAddress>,
        gas_payer: AccountAddress,
        max_gas_amount: u64,
        gas_unit_price: u64,
        chain_id: u8,
        entry_function_payload: Option<EntryFunctionPayload>,
        multisig_payload: Option<MultisigPayload>,
        disallow_module_publishing: bool,
    ) -> Self {
        Self {
            sender,
            secondary_signers,
            gas_payer,
            max_gas_amount,
            gas_unit_price,
            chain_id,
            entry_function_payload,
            multisig_payload,
            disallow_module_publishing,
            payload_config: None,
        }
    }

    pub fn new_with_payload_config(
        sender: AccountAddress,
        secondary_signers: Vec<AccountAddress>,
        gas_payer: AccountAddress,
        max_gas_amount: u64,
        gas_unit_price: u64,
        chain_id: u8,
        entry_function_payload: Option<EntryFunctionPayload>,
        multisig_payload: Option<MultisigPayload>,
        disallow_module_publishing: bool,
        payload_config: Option<PayloadConfig>,
    ) -> Self {
        Self {
            sender,
            secondary_signers,
            gas_payer,
            max_gas_amount,
            gas_unit_price,
            chain_id,
            entry_function_payload,
            multisig_payload,
            disallow_module_publishing,
            payload_config,
        }
    }

    pub fn sender(&self) -> AccountAddress {
        self.sender
    }

    pub fn secondary_signers(&self) -> Vec<AccountAddress> {
        self.secondary_signers.clone()
    }

    pub fn gas_payer(&self) -> AccountAddress {
        self.gas_payer
    }

    pub fn max_gas_amount(&self) -> u64 {
        self.max_gas_amount
    }

    pub fn gas_unit_price(&self) -> u64 {
        self.gas_unit_price
    }

    pub fn chain_id(&self) -> u8 {
        self.chain_id
    }

    pub fn entry_function_payload(&self) -> Option<EntryFunctionPayload> {
        self.entry_function_payload.clone()
    }

    pub fn multisig_payload(&self) -> Option<MultisigPayload> {
        self.multisig_payload.clone()
    }

    pub fn disallow_module_publishing(&self) -> bool {
        self.disallow_module_publishing
    }

    pub fn payload_config(&self) -> Option<PayloadConfig> {
        self.payload_config.clone()
    }
}

#[derive(Debug, Clone)]
pub struct EntryFunctionPayload {
    pub account_address: AccountAddress,
    pub module_name: String,
    pub function_name: String,
    pub ty_arg_names: Vec<String>,
    pub args: Vec<Vec<u8>>,
}
impl EntryFunctionPayload {
    pub fn new(
        account_address: AccountAddress,
        module_name: String,
        function_name: String,
        ty_arg_names: Vec<String>,
        args: Vec<Vec<u8>>,
    ) -> Self {
        Self {
            account_address,
            module_name,
            function_name,
            ty_arg_names,
            args,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MultisigPayload {
    pub multisig_address: AccountAddress,
    pub entry_function_payload: Option<EntryFunctionPayload>,
}
impl MultisigPayload {
    pub fn new(
        multisig_address: AccountAddress,
        entry_function_payload: Option<EntryFunctionPayload>,
    ) -> Self {
        Self {
            multisig_address,
            entry_function_payload,
        }
    }
}
