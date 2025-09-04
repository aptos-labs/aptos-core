// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_types::transaction::use_case::{UseCaseAwareTransaction, UseCaseKey};
use move_core_types::account_address::AccountAddress;
use proptest_derive::Arbitrary;
use std::fmt::Debug;

mod manual;
mod proptests;

#[derive(Arbitrary)]
enum Contract {
    Platform,
    Others,
    User(u8),
}

impl Debug for Contract {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Contract::*;

        write!(f, "c{}", match self {
            Platform => "PP".to_string(),
            Others => "OO".to_string(),
            User(addr) => hex::encode_upper(addr.to_be_bytes()),
        })
    }
}

#[derive(Arbitrary)]
struct Account(u8);

impl Debug for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "a{}", hex::encode_upper(self.0.to_be_bytes()))
    }
}

impl Account {
    fn as_account_address(&self) -> AccountAddress {
        let mut addr = [0u8; 32];
        addr[31..].copy_from_slice(&self.0.to_be_bytes());
        AccountAddress::new(addr)
    }
}

struct Transaction {
    contract: Contract,
    sender: Account,
    original_idx: usize,
}

impl Debug for Transaction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "t{}:{:?}{:?}",
            self.original_idx, self.contract, self.sender
        )
    }
}

impl UseCaseAwareTransaction for Transaction {
    fn parse_sender(&self) -> AccountAddress {
        self.sender.as_account_address()
    }

    fn parse_use_case(&self) -> UseCaseKey {
        use UseCaseKey::*;

        match self.contract {
            Contract::Platform => Platform,
            Contract::Others => Others,
            Contract::User(c) => ContractAddress(Account(c).as_account_address()),
        }
    }
}

fn into_txns(txns: impl IntoIterator<Item = (Contract, Account)>) -> Vec<Transaction> {
    let mut original_idx = 0;
    txns.into_iter()
        .map(|(contract, sender)| {
            let txn = Transaction {
                contract,
                sender,
                original_idx,
            };

            original_idx += 1;
            txn
        })
        .collect()
}
