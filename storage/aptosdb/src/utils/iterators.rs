// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::schema::{
    event::EventSchema, ledger_info::LedgerInfoSchema,
    transaction_summaries_by_account::TransactionSummariesByAccountSchema,
};
use aptos_schemadb::iterator::{ScanDirection, SchemaIterator};
use aptos_storage_interface::{db_ensure as ensure, AptosDbError, Result};
use aptos_types::{
    account_address::AccountAddress,
    contract_event::ContractEvent,
    ledger_info::LedgerInfoWithSignatures,
    transaction::{IndexedTransactionSummary, Version},
};
use std::{iter::Peekable, marker::PhantomData};

pub struct ContinuousVersionIter<I, T> {
    inner: I,
    first_version: Version,
    expected_next_version: Version,
    end_version: Version,
    _phantom: PhantomData<T>,
}

impl<I, T> ContinuousVersionIter<I, T>
where
    I: Iterator<Item = Result<(Version, T)>>,
{
    fn next_impl(&mut self) -> Result<Option<T>> {
        if self.expected_next_version >= self.end_version {
            return Ok(None);
        }

        let ret = match self.inner.next().transpose()? {
            Some((version, transaction)) => {
                ensure!(
                    version == self.expected_next_version,
                    "{} iterator: first version {}, expecting version {}, got {} from underlying iterator.",
                    std::any::type_name::<T>(),
                    self.first_version,
                    self.expected_next_version,
                    version,
                );
                self.expected_next_version += 1;
                Some(transaction)
            },
            None => None,
        };

        Ok(ret)
    }
}

impl<I, T> Iterator for ContinuousVersionIter<I, T>
where
    I: Iterator<Item = Result<(Version, T)>>,
{
    type Item = Result<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_impl().transpose()
    }
}

pub trait ExpectContinuousVersions<T>: Iterator<Item = Result<(Version, T)>> + Sized {
    fn expect_continuous_versions(
        self,
        first_version: Version,
        limit: usize,
    ) -> Result<ContinuousVersionIter<Self, T>>;
}

impl<I, T> ExpectContinuousVersions<T> for I
where
    I: Iterator<Item = Result<(Version, T)>>,
{
    fn expect_continuous_versions(
        self,
        first_version: Version,
        limit: usize,
    ) -> Result<ContinuousVersionIter<Self, T>> {
        Ok(ContinuousVersionIter {
            inner: self,
            first_version,
            expected_next_version: first_version,
            end_version: first_version
                .checked_add(limit as u64)
                .ok_or(AptosDbError::TooManyRequested(first_version, limit as u64))?,
            _phantom: Default::default(),
        })
    }
}

pub struct EpochEndingLedgerInfoIter<'a> {
    inner: SchemaIterator<'a, LedgerInfoSchema>,
    next_epoch: u64,
    end_epoch: u64,
}

impl<'a> EpochEndingLedgerInfoIter<'a> {
    pub(crate) fn new(
        inner: SchemaIterator<'a, LedgerInfoSchema>,
        next_epoch: u64,
        end_epoch: u64,
    ) -> Self {
        Self {
            inner,
            next_epoch,
            end_epoch,
        }
    }

    fn next_impl(&mut self) -> Result<Option<LedgerInfoWithSignatures>> {
        if self.next_epoch >= self.end_epoch {
            return Ok(None);
        }

        let ret = match self.inner.next().transpose()? {
            Some((epoch, li)) => {
                if !li.ledger_info().ends_epoch() {
                    None
                } else {
                    ensure!(
                        epoch == self.next_epoch,
                        "Epochs are not consecutive. expecting: {}, got: {}",
                        self.next_epoch,
                        epoch,
                    );
                    self.next_epoch += 1;
                    Some(li)
                }
            },
            _ => None,
        };

        Ok(ret)
    }
}

impl Iterator for EpochEndingLedgerInfoIter<'_> {
    type Item = Result<LedgerInfoWithSignatures>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_impl().transpose()
    }
}

pub struct EventsByVersionIter<'a> {
    inner: Peekable<SchemaIterator<'a, EventSchema>>,
    expected_next_version: Version,
    end_version: Version,
}

impl<'a> EventsByVersionIter<'a> {
    pub(crate) fn new(
        inner: SchemaIterator<'a, EventSchema>,
        expected_next_version: Version,
        end_version: Version,
    ) -> Self {
        Self {
            inner: inner.peekable(),
            expected_next_version,
            end_version,
        }
    }

    fn next_impl(&mut self) -> Result<Option<Vec<ContractEvent>>> {
        if self.expected_next_version >= self.end_version {
            return Ok(None);
        }

        let mut ret = Vec::new();
        while let Some(res) = self.inner.peek() {
            let ((version, _index), _event) = res
                .as_ref()
                .map_err(|e| AptosDbError::Other(format!("Hit error iterating events: {}", e)))?;
            if *version != self.expected_next_version {
                break;
            }
            let ((_version, _index), event) =
                self.inner.next().transpose()?.expect("Known to exist.");
            ret.push(event);
        }
        self.expected_next_version = self
            .expected_next_version
            .checked_add(1)
            .ok_or_else(|| AptosDbError::Other("expected version overflowed.".to_string()))?;
        Ok(Some(ret))
    }
}

impl Iterator for EventsByVersionIter<'_> {
    type Item = Result<Vec<ContractEvent>>;

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

impl AccountTransactionSummariesIter<'_> {
    fn next_impl(&mut self) -> Result<Option<(Version, IndexedTransactionSummary)>> {
        // If already iterated over `limit` transactions, return None.
        if self.count >= self.limit {
            return Ok(None);
        }

        Ok(match self.inner.next().transpose()? {
            Some(((address, version), txn_summary)) => {
                // No more transactions sent by this account.
                if address != self.address {
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
                    version == txn_summary.version(),
                    "DB corruption: version mismatch: version in key: {}, version in txn summary: {}",
                    version,
                    txn_summary.version(),
                );

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

impl Iterator for AccountTransactionSummariesIter<'_> {
    type Item = Result<(Version, IndexedTransactionSummary)>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_impl().transpose()
    }
}
