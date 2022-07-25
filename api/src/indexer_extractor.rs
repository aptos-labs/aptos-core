// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::context::Context;
use aptos_api_types::TransactionOnChainData;
use std::{time::Duration, thread};

const RETRY_TIME_MILLIS: u64 = 5000;
const TRANSACTION_FETCH_BATCH_SIZE: u16 = 500;

pub struct Extractor {
    context: Context,
    current_version: u64,
    ledger_version: u64,
    transactions_buffer: Vec<TransactionOnChainData>,
}

impl Extractor {
    pub fn new(context: Context, start_version: u64, ledger_version: u64) -> Self {
        Self {
            context,
            current_version: start_version,
            ledger_version,
            transactions_buffer: Vec::new(),
        }
    }

    /// kicks off the extractor process
    pub fn bootstrap(&mut self) -> () {
        loop {
            if !self.transactions_buffer.is_empty() {
                // parse and pipe out
                build_and_push_proto(self.transactions_buffer.last().unwrap());
                self.transactions_buffer.pop();
                self.current_version += 1;
                continue;
            }
            // fill it up!
            let res = self.context.get_transactions(
                self.current_version,
                TRANSACTION_FETCH_BATCH_SIZE,
                self.current_version,
            );

            match res {
                Ok(mut transactions) => {
                    transactions.reverse();
                    self.transactions_buffer = transactions;
                }
                Err(_) => {
                    aptos_logger::debug!(
                        "Could not fetch {} transactions starting at {}. Will check again in {}ms.",
                        TRANSACTION_FETCH_BATCH_SIZE,
                        self.current_version,
                        RETRY_TIME_MILLIS,
                    );
                    thread::sleep(Duration::from_millis(RETRY_TIME_MILLIS));
                }
            }
        }
    }
}

pub fn build_and_push_proto(transaction: &TransactionOnChainData) -> () {
    println!("building proto for transaction {}", { transaction.version });
}
