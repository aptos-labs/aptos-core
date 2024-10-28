// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use aptos_types::{transaction::signature_verified_transaction::SignatureVerifiedTransaction, account_address::AccountAddress};
use once_cell::sync::{Lazy, OnceCell};
use rayon::{ThreadPool, ThreadPoolBuilder};


pub enum NativeTransaction {
    Nop { sender: AccountAddress, sequence_number: u64, },
    FaTransfer { sender: AccountAddress, sequence_number: u64, recipient: AccountAddress, amount: u64},
    Transfer { sender: AccountAddress, sequence_number: u64, recipient: AccountAddress, amount: u64, fail_on_account_existing: bool, fail_on_account_missing: bool, },
    BatchTransfer { sender: AccountAddress, sequence_number: u64, recipients: Vec<AccountAddress>, amounts: Vec<u64>, fail_on_account_existing: bool, fail_on_account_missing: bool, },
}

impl NativeTransaction {
    pub fn parse(txn: &SignatureVerifiedTransaction) -> Self {
        match &txn.expect_valid() {
            aptos_types::transaction::Transaction::UserTransaction(user_txn) => match user_txn.payload() {
                aptos_types::transaction::TransactionPayload::EntryFunction(f) => {
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
                                amount: bcs::from_bytes(&f.args()[1]).unwrap()
                            }
                        },
                        (AccountAddress::ONE, "coin", "transfer") => {
                            Self::Transfer {
                                sender: user_txn.sender(),
                                sequence_number: user_txn.sequence_number(),
                                recipient: bcs::from_bytes(&f.args()[0]).unwrap(),
                                amount: bcs::from_bytes(&f.args()[1]).unwrap(),
                                fail_on_account_existing: false,
                                fail_on_account_missing: true,
                            }
                        },
                        (AccountAddress::ONE, "aptos_account", "transfer") => {
                            Self::Transfer {
                                sender: user_txn.sender(),
                                sequence_number: user_txn.sequence_number(),
                                recipient: bcs::from_bytes(&f.args()[0]).unwrap(),
                                amount: bcs::from_bytes(&f.args()[1]).unwrap(),
                                fail_on_account_existing: false,
                                fail_on_account_missing: false,
                            }
                        },
                        (AccountAddress::ONE, "aptos_account", "create_account") => {
                            Self::Transfer {
                                sender: user_txn.sender(),
                                sequence_number: user_txn.sequence_number(),
                                recipient: bcs::from_bytes(&f.args()[0]).unwrap(),
                                amount: 0,
                                fail_on_account_existing: true,
                                fail_on_account_missing: false,
                            }
                        },
                        (AccountAddress::ONE, "aptos_account", "batch_transfer") => {
                            Self::BatchTransfer {
                                sender: user_txn.sender(),
                                sequence_number: user_txn.sequence_number(),
                                recipients: bcs::from_bytes(&f.args()[0]).unwrap(),
                                amounts: bcs::from_bytes(&f.args()[1]).unwrap(),
                                fail_on_account_existing: false,
                                fail_on_account_missing: true,
                            }
                        },
                        (_, "simple", "nop") => {
                            Self::Nop {
                                sender: user_txn.sender(),
                                sequence_number: user_txn.sequence_number()
                            }
                        },
                        (AccountAddress::ONE, "code", "publish_package_txn") => {
                            // Publishing doesn't do anything, either we know how to deal
                            // with later transactions or not.
                            Self::Nop {
                                sender: user_txn.sender(),
                                sequence_number: user_txn.sequence_number()
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
            },
            _ => unimplemented!(),
        }
    }
}


pub static NATIVE_EXECUTOR_CONCURRENCY_LEVEL: OnceCell<usize> = OnceCell::new();
pub static NATIVE_EXECUTOR_POOL: Lazy<Arc<ThreadPool>> = Lazy::new(|| {
    Arc::new(ThreadPoolBuilder::new()
        .num_threads(NativeConfig::get_concurrency_level())
        .thread_name(|index| format!("native_exe_{}", index))
        .build()
        .unwrap())
});

pub struct NativeConfig;

impl NativeConfig {
    pub fn set_concurrency_level_once(concurrency_level: usize) {
        NATIVE_EXECUTOR_CONCURRENCY_LEVEL
            .set(concurrency_level)
            .ok();
    }

    pub fn get_concurrency_level() -> usize {
        match NATIVE_EXECUTOR_CONCURRENCY_LEVEL.get() {
            Some(concurrency_level) => *concurrency_level,
            None => 32,
        }
    }
}
