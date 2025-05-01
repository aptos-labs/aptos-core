// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    schema::{
        event::EventSchema, ledger_info::LedgerInfoSchema, state_value::StateValueSchema,
        transaction_summaries_by_account::TransactionSummariesByAccountSchema,
    },
    state_kv_db::StateKvDb,
};
use aptos_schemadb::{
    iterator::{ScanDirection, SchemaIterator},
    ReadOptions,
};
use aptos_storage_interface::{db_ensure as ensure, AptosDbError, Result};
use aptos_types::{
    account_address::AccountAddress,
    contract_event::ContractEvent,
    ledger_info::LedgerInfoWithSignatures,
    state_store::{
        state_key::{prefix::StateKeyPrefix, StateKey},
        state_value::StateValue,
    },
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

pub struct PrefixedStateValueIterator<'a> {
    kv_iter: Option<SchemaIterator<'a, StateValueSchema>>,
    key_prefix: StateKeyPrefix,
    prev_key: Option<StateKey>,
    desired_version: Version,
    is_finished: bool,
}

impl<'a> PrefixedStateValueIterator<'a> {
    pub fn new(
        db: &'a StateKvDb,
        key_prefix: StateKeyPrefix,
        first_key: Option<StateKey>,
        desired_version: Version,
    ) -> Result<Self> {
        let mut read_opts = ReadOptions::default();
        // Without this, iterators are not guaranteed a total order of all keys, but only keys for the same prefix.
        // For example,
        // aptos/abc|2
        // aptos/abc|1
        // aptos/abc|0
        // aptos/abd|1
        // if we seek('aptos/'), and call next, we may not reach `aptos/abd/1` because the prefix extractor we adopted
        // here will stick with prefix `aptos/abc` and return `None` or any arbitrary result after visited all the
        // keys starting with `aptos/abc`.
        read_opts.set_total_order_seek(true);
        let mut kv_iter = db
            .metadata_db()
            .iter_with_opts::<StateValueSchema>(read_opts)?;
        if let Some(first_key) = &first_key {
            kv_iter.seek(&(first_key.clone(), u64::MAX))?;
        } else {
            kv_iter.seek(&&key_prefix)?;
        };
        Ok(Self {
            kv_iter: Some(kv_iter),
            key_prefix,
            prev_key: None,
            desired_version,
            is_finished: false,
        })
    }

    fn next_by_kv(&mut self) -> Result<Option<(StateKey, StateValue)>> {
        let iter = self.kv_iter.as_mut().unwrap();
        if !self.is_finished {
            while let Some(((state_key, version), state_value_opt)) = iter.next().transpose()? {
                // In case the previous seek() ends on the same key with version 0.
                if Some(&state_key) == self.prev_key.as_ref() {
                    continue;
                }
                // Cursor is currently at the first available version of the state key.
                // Check if the key_prefix is a valid prefix of the state_key we got from DB.
                if !self.key_prefix.is_prefix(&state_key)? {
                    // No more keys matching the key_prefix, we can return the result.
                    self.is_finished = true;
                    break;
                }

                if version > self.desired_version {
                    iter.seek(&(state_key.clone(), self.desired_version))?;
                    continue;
                }

                self.prev_key = Some(state_key.clone());
                // Seek to the next key - this can be done by seeking to the current key with version 0
                iter.seek(&(state_key.clone(), 0))?;

                if let Some(state_value) = state_value_opt {
                    return Ok(Some((state_key, state_value)));
                }
            }
        }
        Ok(None)
    }
}

impl<'a> Iterator for PrefixedStateValueIterator<'a> {
    type Item = Result<(StateKey, StateValue)>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_by_kv().transpose()
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

impl<'a> Iterator for EpochEndingLedgerInfoIter<'a> {
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

impl<'a> Iterator for EventsByVersionIter<'a> {
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

impl<'a> AccountTransactionSummariesIter<'a> {
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

impl<'a> Iterator for AccountTransactionSummariesIter<'a> {
    type Item = Result<(Version, IndexedTransactionSummary)>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_impl().transpose()
    }
}
