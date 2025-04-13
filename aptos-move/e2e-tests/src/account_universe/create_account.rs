// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::arc_with_non_send_sync)]

use crate::{
    account::{Account, AccountData},
    account_universe::{
        txn_one_account_result, AUTransactionGen, AccountPair, AccountPairGen, AccountUniverse,
    },
    common_transactions::create_account_txn,
    gas_costs,
};
use aptos_proptest_helpers::Index;
use aptos_types::{
    transaction::{ExecutionStatus, SignedTransaction, TransactionStatus},
    vm_status::{AbortLocation, StatusCode},
};
use proptest::prelude::*;
use proptest_derive::Arbitrary;
use std::sync::Arc;

/// Represents a create-account transaction performed in the account universe.
///
/// The parameters are the minimum and maximum balances to transfer.
#[derive(Arbitrary, Clone, Debug)]
#[proptest(params = "(u64, u64)")]
pub struct CreateAccountGen {
    sender: Index,
    new_account: Account,
    #[proptest(strategy = "params.0 ..= params.1")]
    amount: u64,
}

impl AUTransactionGen for CreateAccountGen {
    fn apply(
        &self,
        universe: &mut AccountUniverse,
    ) -> (SignedTransaction, (TransactionStatus, u64)) {
        let sender = universe.pick(self.sender).1;

        let txn = create_account_txn(sender.account(), &self.new_account, sender.sequence_number);

        let mut gas_used = sender.create_account_gas_cost();
        let low_balance_gas_used = sender.create_account_low_balance_gas_cost();
        let gas_price = txn.gas_unit_price();

        let (status, is_success) = txn_one_account_result(
            sender,
            self.amount,
            gas_price,
            gas_used,
            low_balance_gas_used,
        );
        if is_success {
            sender.event_counter_created = true;
            universe.add_account(AccountData::with_account(
                self.new_account.clone(),
                self.amount,
                0,
                true,
                false,
            ));
        } else {
            gas_used = 0;
        }

        (txn, (status, gas_used))
    }
}

/// Represents a create-account transaction in the account universe where the destination already
/// exists.
///
/// The parameters are the minimum and maximum balances to transfer.
#[derive(Arbitrary, Clone, Debug)]
#[proptest(params = "(u64, u64)")]
pub struct CreateExistingAccountGen {
    sender_receiver: AccountPairGen,
}

impl AUTransactionGen for CreateExistingAccountGen {
    fn apply(
        &self,
        universe: &mut AccountUniverse,
    ) -> (SignedTransaction, (TransactionStatus, u64)) {
        let AccountPair {
            account_1: sender,
            account_2: receiver,
            ..
        } = self.sender_receiver.pick(universe);

        let txn = create_account_txn(sender.account(), receiver.account(), sender.sequence_number);

        // This transaction should never work, but it will fail differently if there's not enough
        // gas to reserve.
        let mut gas_used = 0;
        let gas_price = txn.gas_unit_price();
        let enough_max_gas = sender.balance >= gas_costs::TXN_RESERVED * gas_price;
        let status = if enough_max_gas {
            sender.sequence_number += 1;
            gas_used = sender.create_existing_account_gas_cost();
            sender.balance -= gas_used * gas_price;
            // TODO(tmn) provide a real abort location
            TransactionStatus::Keep(ExecutionStatus::MoveAbort {
                location: AbortLocation::Script,
                code: 777_777,
                info: None,
            })
        } else {
            // Not enough gas to get past the prologue.
            TransactionStatus::Discard(StatusCode::INSUFFICIENT_BALANCE_FOR_TRANSACTION_FEE)
        };

        (txn, (status, gas_used))
    }
}

pub fn create_account_strategy(
    min: u64,
    max: u64,
) -> impl Strategy<Value = Arc<dyn AUTransactionGen + 'static>> {
    prop_oneof![
        3 => any_with::<CreateAccountGen>((min, max)).prop_map(CreateAccountGen::arced),
        1 => any_with::<CreateExistingAccountGen>((min, max)).prop_map(
            CreateExistingAccountGen::arced,
        ),
    ]
}
