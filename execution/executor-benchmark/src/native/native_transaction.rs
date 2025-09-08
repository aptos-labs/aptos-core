// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::{
    account_address::AccountAddress,
    transaction::signature_verified_transaction::SignatureVerifiedTransaction,
};
use std::collections::HashMap;

#[derive(Debug)]
pub enum NativeTransaction {
    Nop {
        sender: AccountAddress,
        sequence_number: u64,
    },
    FaTransfer {
        sender: AccountAddress,
        sequence_number: u64,
        recipient: AccountAddress,
        amount: u64,
    },
    Transfer {
        sender: AccountAddress,
        sequence_number: u64,
        recipient: AccountAddress,
        amount: u64,
        fail_on_recipient_account_existing: bool,
        fail_on_recipient_account_missing: bool,
    },
    BatchTransfer {
        sender: AccountAddress,
        sequence_number: u64,
        recipients: Vec<AccountAddress>,
        amounts: Vec<u64>,
        fail_on_recipient_account_existing: bool,
        fail_on_recipient_account_missing: bool,
    },
    BlockEpilogue,
}

impl NativeTransaction {
    pub fn parse(txn: &SignatureVerifiedTransaction) -> Self {
        match &txn.expect_valid() {
            aptos_types::transaction::Transaction::UserTransaction(user_txn) => {
                // Use executable_ref() to handle both EntryFunction and Payload variants uniformly
                match user_txn.payload().executable_ref() {
                    Ok(aptos_types::transaction::TransactionExecutableRef::EntryFunction(f))
                        if !user_txn.payload().is_multisig() =>
                    {
                        match (
                            *f.module().address(),
                            f.module().name().as_str(),
                            f.function().as_str(),
                        ) {
                            (AccountAddress::ONE, "aptos_account", "fungible_transfer_only") => {
                                Self::FaTransfer {
                                    sender: user_txn.sender(),
                                    sequence_number: user_txn.sequence_number(),
                                    recipient: bcs::from_bytes(&f.args()[0]).unwrap(),
                                    amount: bcs::from_bytes(&f.args()[1]).unwrap(),
                                }
                            },
                            (AccountAddress::ONE, "coin", "transfer") => Self::Transfer {
                                sender: user_txn.sender(),
                                sequence_number: user_txn.sequence_number(),
                                recipient: bcs::from_bytes(&f.args()[0]).unwrap(),
                                amount: bcs::from_bytes(&f.args()[1]).unwrap(),
                                fail_on_recipient_account_existing: false,
                                fail_on_recipient_account_missing: true,
                            },
                            (AccountAddress::ONE, "aptos_account", "transfer") => Self::Transfer {
                                sender: user_txn.sender(),
                                sequence_number: user_txn.sequence_number(),
                                recipient: bcs::from_bytes(&f.args()[0]).unwrap(),
                                amount: bcs::from_bytes(&f.args()[1]).unwrap(),
                                fail_on_recipient_account_existing: false,
                                fail_on_recipient_account_missing: false,
                            },
                            (AccountAddress::ONE, "aptos_account", "create_account") => {
                                Self::Transfer {
                                    sender: user_txn.sender(),
                                    sequence_number: user_txn.sequence_number(),
                                    recipient: bcs::from_bytes(&f.args()[0]).unwrap(),
                                    amount: 0,
                                    fail_on_recipient_account_existing: true,
                                    fail_on_recipient_account_missing: false,
                                }
                            },
                            (AccountAddress::ONE, "aptos_account", "batch_transfer") => {
                                Self::BatchTransfer {
                                    sender: user_txn.sender(),
                                    sequence_number: user_txn.sequence_number(),
                                    recipients: bcs::from_bytes(&f.args()[0]).unwrap(),
                                    amounts: bcs::from_bytes(&f.args()[1]).unwrap(),
                                    fail_on_recipient_account_existing: false,
                                    fail_on_recipient_account_missing: true,
                                }
                            },
                            (_, "simple", "nop") => Self::Nop {
                                sender: user_txn.sender(),
                                sequence_number: user_txn.sequence_number(),
                            },
                            (AccountAddress::ONE, "code", "publish_package_txn") => {
                                // Publishing doesn't do anything, either we know how to deal
                                // with later transactions or not.
                                Self::Nop {
                                    sender: user_txn.sender(),
                                    sequence_number: user_txn.sequence_number(),
                                }
                            },
                            _ => unimplemented!(
                                "{} {}::{}",
                                *f.module().address(),
                                f.module().name().as_str(),
                                f.function().as_str()
                            ),
                        }
                    },
                    _ => unimplemented!(),
                }
            },
            aptos_types::transaction::Transaction::BlockEpilogue(_) => Self::BlockEpilogue,
            _ => unimplemented!(),
        }
    }
}

pub fn compute_deltas_for_batch(
    recipient_addresses: Vec<AccountAddress>,
    transfer_amounts: Vec<u64>,
    sender_address: AccountAddress,
) -> (HashMap<AccountAddress, i64>, u64) {
    let mut deltas = HashMap::new();
    for (recipient, amount) in recipient_addresses
        .into_iter()
        .zip(transfer_amounts.into_iter())
    {
        let amount = amount as i64;
        deltas
            .entry(recipient)
            .and_modify(|counter| *counter += amount)
            .or_insert(amount);
        deltas
            .entry(sender_address)
            .and_modify(|counter| *counter -= amount)
            .or_insert(-amount);
    }

    let amount_from_sender = -deltas.remove(&sender_address).unwrap_or(0);
    assert!(amount_from_sender >= 0);

    (deltas, amount_from_sender as u64)
}
