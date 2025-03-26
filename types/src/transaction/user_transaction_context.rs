// Copyright (c) 2024 Supra.
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_core_types::account_address::AccountAddress;
use std::fmt::Debug;

/// Generic means to describe transaction payload type and reference based on referenced context.
/// EF describes entry-function type in the particular context of its reference.
/// MSF describes multisig payload type in the particular context of its reference.
#[derive(Debug)]
pub enum PayloadTypeReference<EFP: Debug, MSP: Debug> {
    /// Indicates a transaction other than user txn with entry-function or multisig or automation payload.
    Other,
    /// Indicates user transaction with entry-function payload variant enclosing the payload.
    UserEntryFunction(EFP),
    /// Indicates user transaction with multisig payload variant enclosing the multi-sig payload.
    Multisig(MSP),
    /// Indicates user transaction with automation payload variant.
    AutomationRegistration,
}


impl<EFP, MSP> PayloadTypeReference<EFP, MSP>
where
    EFP: Clone + Debug,
    MSP: Clone + Debug,
{
    pub fn entry_function_payload(&self) -> Option<EFP> {
        let PayloadTypeReference::UserEntryFunction(entry_function_payload) = self else {
            return None;
        };
        Some(entry_function_payload.clone())
    }

    pub fn multisig_payload(&self) -> Option<MSP> {
        let PayloadTypeReference::Multisig(multisig_payload) = self else {
            return None;
        };
        Some(multisig_payload.clone())
    }

    pub fn is_automation_registration(&self) -> bool {
        matches!(self, Self::AutomationRegistration)
    }
}

pub type PayloadTypeReferenceContext = PayloadTypeReference<EntryFunctionPayload, MultisigPayload>;

#[derive(Debug)]
pub struct UserTransactionContext {
    sender: AccountAddress,
    secondary_signers: Vec<AccountAddress>,
    gas_payer: AccountAddress,
    max_gas_amount: u64,
    gas_unit_price: u64,
    chain_id: u8,
    payload_type_reference: PayloadTypeReferenceContext,
}

impl UserTransactionContext {
    pub fn new(
        sender: AccountAddress,
        secondary_signers: Vec<AccountAddress>,
        gas_payer: AccountAddress,
        max_gas_amount: u64,
        gas_unit_price: u64,
        chain_id: u8,
        payload_type_reference: PayloadTypeReferenceContext,
    ) -> Self {
        Self {
            sender,
            secondary_signers,
            gas_payer,
            max_gas_amount,
            gas_unit_price,
            chain_id,
            payload_type_reference,
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
        self.payload_type_reference.entry_function_payload()
    }

    pub fn multisig_payload(&self) -> Option<MultisigPayload> {
        self.payload_type_reference.multisig_payload()
    }

    pub fn is_automation_registration(&self) -> bool {
        self.payload_type_reference.is_automation_registration()
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
