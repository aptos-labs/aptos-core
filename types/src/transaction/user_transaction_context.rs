// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_core_types::account_address::AccountAddress;

#[derive(Debug)]
pub struct UserTransactionContext {
    sender: AccountAddress,
    secondary_signers: Vec<AccountAddress>,
    gas_payer: AccountAddress,
    max_gas_amount: u64,
    gas_unit_price: u64,
    chain_id: u8,
}

impl UserTransactionContext {
    pub fn new(
        sender: AccountAddress,
        secondary_signers: Vec<AccountAddress>,
        gas_payer: AccountAddress,
        max_gas_amount: u64,
        gas_unit_price: u64,
        chain_id: u8,
    ) -> Self {
        Self {
            sender,
            secondary_signers,
            gas_payer,
            max_gas_amount,
            gas_unit_price,
            chain_id,
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
}
