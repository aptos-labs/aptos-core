// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::transaction::TransactionSchema;
use crate::transaction_by_account::TransactionByAccountSchema;
use anyhow::{anyhow, ensure, Result};
use aptos_types::account_address::AccountAddress;
use aptos_types::transaction::{Transaction, Version};
use schemadb::iterator::SchemaIterator;

pub struct TransactionIter<'a> {
    inner: SchemaIterator<'a, TransactionSchema>,
    expected_next_version: Version,
    end_version: Version,
}

impl<'a> TransactionIter<'a> {
    pub(crate) fn new(
        inner: SchemaIterator<'a, TransactionSchema>,
        expected_next_version: Version,
        limit: usize,
    ) -> Result<Self> {
        Ok(Self {
            inner,
            expected_next_version,
            end_version: expected_next_version
                .checked_add(limit as u64)
                .ok_or_else(|| anyhow!("Too many transactions requested."))?,
        })
    }
}

impl<'a> TransactionIter<'a> {
    fn next_impl(&mut self) -> Result<Option<Transaction>> {
        if self.expected_next_version >= self.end_version {
            return Ok(None);
        }

        let ret = match self.inner.next().transpose()? {
            Some((version, transaction)) => {
                ensure!(
                    version == self.expected_next_version,
                    "Transaction versions are not consecutive.",
                );
                self.expected_next_version += 1;
                Some(transaction)
            }
            None => None,
        };

        Ok(ret)
    }
}

impl<'a> Iterator for TransactionIter<'a> {
    type Item = Result<Transaction>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_impl().transpose()
    }
}

pub struct AccountTransactionVersionIter<'a> {
    inner: SchemaIterator<'a, TransactionByAccountSchema>,
    address: AccountAddress,
    expected_next_seq_num: Option<u64>,
    end_seq_num: u64,
    prev_version: Option<Version>,
    ledger_version: Version,
}

impl<'a> AccountTransactionVersionIter<'a> {
    pub(crate) fn new(
        inner: SchemaIterator<'a, TransactionByAccountSchema>,
        address: AccountAddress,
        end_seq_num: u64,
        ledger_version: Version,
    ) -> Self {
        Self {
            inner,
            address,
            end_seq_num,
            ledger_version,
            expected_next_seq_num: None,
            prev_version: None,
        }
    }
}

impl<'a> AccountTransactionVersionIter<'a> {
    fn next_impl(&mut self) -> Result<Option<(u64, Version)>> {
        Ok(match self.inner.next().transpose()? {
            Some(((address, seq_num), version)) => {
                // No more transactions sent by this account.
                if address != self.address {
                    return Ok(None);
                }
                if seq_num >= self.end_seq_num {
                    return Ok(None);
                }

                // Ensure seq_num_{i+1} == seq_num_{i} + 1
                if let Some(expected_seq_num) = self.expected_next_seq_num {
                    ensure!(
                        seq_num == expected_seq_num,
                        "DB corruption: account transactions sequence numbers are not contiguous: \
                     actual: {}, expected: {}",
                        seq_num,
                        expected_seq_num,
                    );
                };

                // Ensure version_{i+1} > version_{i}
                if let Some(prev_version) = self.prev_version {
                    ensure!(
                        prev_version < version,
                        "DB corruption: account transaction versions are not strictly increasing: \
                         previous version: {}, current version: {}",
                        prev_version,
                        version,
                    );
                }

                // No more transactions (in this view of the ledger).
                if version > self.ledger_version {
                    return Ok(None);
                }

                self.expected_next_seq_num = Some(seq_num + 1);
                self.prev_version = Some(version);
                Some((seq_num, version))
            }
            None => None,
        })
    }
}

impl<'a> Iterator for AccountTransactionVersionIter<'a> {
    type Item = Result<(u64, Version)>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_impl().transpose()
    }
}
