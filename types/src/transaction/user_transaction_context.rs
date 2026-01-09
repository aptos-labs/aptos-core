// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use move_core_types::account_address::AccountAddress;

/// Represents the transaction index context for the monotonically increasing counter.
#[derive(Debug, Clone, Copy)]
pub enum TransactionIndexKind {
    /// Actual block/chunk execution (PersistedAuxiliaryInfo::V1).
    /// The reserved byte in the counter will be 0.
    BlockExecution { transaction_index: u32 },
    /// Validation or simulation (PersistedAuxiliaryInfo::TimestampNotYetAssignedV1).
    /// The reserved byte in the counter will be 1.
    ValidationOrSimulation { transaction_index: u32 },
    /// Not available (PersistedAuxiliaryInfo::None).
    /// Will abort with ETRANSACTION_INDEX_NOT_AVAILABLE.
    NotAvailable,
}

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
    /// The transaction index context for the monotonically increasing counter.
    transaction_index_kind: TransactionIndexKind,
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
        transaction_index_kind: TransactionIndexKind,
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
            transaction_index_kind,
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

    pub fn transaction_index_kind(&self) -> TransactionIndexKind {
        self.transaction_index_kind
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
