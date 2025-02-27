// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::schema::{
    ordered_transaction_by_account::OrderedTransactionByAccountSchema,
    transaction_summaries_by_account::TransactionSummariesByAccountSchema,
};
use aptos_schemadb::iterator::{ScanDirection, SchemaIterator};
use aptos_storage_interface::{db_ensure as ensure, AptosDbError, Result};
use aptos_types::{
    account_address::AccountAddress,
    indexer::indexer_db_reader::{IndexedTransactionSummary, Order},
    transaction::Version,
};

pub fn ensure_slice_len_eq(data: &[u8], len: usize) -> Result<()> {
    ensure!(
        data.len() == len,
        "Unexpected data len {}, expected {}.",
        data.len(),
        len,
    );
    Ok(())
}

pub const MAX_REQUEST_LIMIT: u64 = 10_000;

pub fn error_if_too_many_requested(num_requested: u64, max_allowed: u64) -> Result<()> {
    if num_requested > max_allowed {
        Err(AptosDbError::TooManyRequested(num_requested, max_allowed))
    } else {
        Ok(())
    }
}

// Convert requested range and order to a range in ascending order.
pub fn get_first_seq_num_and_limit(order: Order, cursor: u64, limit: u64) -> Result<(u64, u64)> {
    ensure!(limit > 0, "limit should > 0, got {}", limit);

    Ok(if order == Order::Ascending {
        (cursor, limit)
    } else if limit <= cursor {
        (cursor - limit + 1, limit)
    } else {
        (0, cursor + 1)
    })
}

// This is a replicate of the AccountOrderedTransactionsIter from storage/aptosdb crate.
pub struct AccountOrderedTransactionsIter<'a> {
    inner: SchemaIterator<'a, OrderedTransactionByAccountSchema>,
    address: AccountAddress,
    expected_next_seq_num: Option<u64>,
    end_seq_num: u64,
    prev_version: Option<Version>,
    ledger_version: Version,
}

impl<'a> AccountOrderedTransactionsIter<'a> {
    pub fn new(
        inner: SchemaIterator<'a, OrderedTransactionByAccountSchema>,
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

impl<'a> AccountOrderedTransactionsIter<'a> {
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
            },
            None => None,
        })
    }
}

impl<'a> Iterator for AccountOrderedTransactionsIter<'a> {
    type Item = Result<(u64, Version)>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_impl().transpose()
    }
}

pub struct AccountTransactionSummariesIter<'a> {
    inner: SchemaIterator<'a, TransactionSummariesByAccountSchema>,
    address: AccountAddress,
    start_version: Option<Version>,
    end_version: Option<Version>,
    limit: u64,
    direction: ScanDirection,
    prev_version: Option<Version>,
    ledger_version: Version,
    count: u64,
}

impl<'a> AccountTransactionSummariesIter<'a> {
    pub fn new(
        inner: SchemaIterator<'a, TransactionSummariesByAccountSchema>,
        address: AccountAddress,
        start_version: Option<Version>,
        end_version: Option<Version>,
        limit: u64,
        direction: ScanDirection,
        ledger_version: Version,
    ) -> Self {
        Self {
            inner,
            address,
            start_version,
            end_version,
            limit,
            direction,
            ledger_version,
            prev_version: None,
            count: 0,
        }
    }
}

impl<'a> AccountTransactionSummariesIter<'a> {
    fn next_impl(&mut self) -> Result<Option<(Version, IndexedTransactionSummary)>> {
        Ok(match self.inner.next().transpose()? {
            Some(((address, version), txn_summary)) => {
                // No more transactions sent by this account.
                if address != self.address {
                    return Ok(None);
                }

                // If already iterated over `limit` transactions, return None.
                if self.count > self.limit {
                    return Ok(None);
                }

                // This case ideally shouldn't occur if the iterator is initiated properly.
                if (self.direction == ScanDirection::Backward
                    && version > self.end_version.unwrap())
                    || (self.direction == ScanDirection::Forward
                        && version < self.start_version.unwrap())
                {
                    return Ok(None);
                }

                ensure!(
                    version == txn_summary.version,
                    "DB corruption: version mismatch: version in key: {}, version in txn summary: {}",
                    version,
                    txn_summary.version,
                );

                // Ensure version_{i+1} > version_{i}
                if let Some(prev_version) = self.prev_version {
                    if self.direction == ScanDirection::Forward {
                        ensure!(
                            prev_version < version,
                            "DB corruption: account transaction versions are not strictly increasing when scanning forward: \
                             previous version: {}, current version: {}",
                            prev_version,
                            version,
                        );
                    } else {
                        ensure!(
                            prev_version > version,
                            "DB corruption: account transaction versions are not strictly decreasing when scanning backward: \
                             previous version: {}, current version: {}",
                            prev_version,
                            version,
                        );
                    }
                }

                // No more transactions (in this view of the ledger).
                if version > self.ledger_version {
                    return Ok(None);
                }

                self.prev_version = Some(version);
                self.count += 1;
                Some((version, txn_summary))
            },
            None => None,
        })
    }
}

impl<'a> Iterator for AccountTransactionSummariesIter<'a> {
    type Item = Result<(Version, IndexedTransactionSummary)>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_impl().transpose()
    }
}
