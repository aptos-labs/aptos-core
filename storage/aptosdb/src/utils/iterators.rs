// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    schema::{
        event::EventSchema, ledger_info::LedgerInfoSchema, state_value::StateValueSchema,
        state_value_index::StateValueIndexSchema,
        transaction_by_account::TransactionByAccountSchema,
    },
    state_kv_db::StateKvDb,
};
use aptos_schemadb::{iterator::SchemaIterator, ReadOptions};
use aptos_storage_interface::{db_ensure as ensure, AptosDbError, Result};
use aptos_types::{
    account_address::AccountAddress,
    contract_event::ContractEvent,
    ledger_info::LedgerInfoWithSignatures,
    state_store::{state_key::StateKey, state_key_prefix::StateKeyPrefix, state_value::StateValue},
    transaction::Version,
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
    db: &'a StateKvDb,
    kv_iter: Option<SchemaIterator<'a, StateValueSchema>>,
    index_iter: Option<SchemaIterator<'a, StateValueIndexSchema>>,
    key_prefix: StateKeyPrefix,
    prev_key: Option<StateKey>,
    desired_version: Version,
    is_finished: bool,
    use_index: bool,
}

impl<'a> PrefixedStateValueIterator<'a> {
    pub fn new(
        db: &'a StateKvDb,
        key_prefix: StateKeyPrefix,
        first_key: Option<StateKey>,
        desired_version: Version,
        use_index: bool,
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
        let (kv_iter, index_iter) = if use_index {
            let mut index_iter = db.metadata_db().iter::<StateValueIndexSchema>(read_opts)?;
            if let Some(first_key) = &first_key {
                index_iter.seek(&(first_key.clone(), u64::MAX))?;
            } else {
                index_iter.seek(&&key_prefix)?;
            };
            (None, Some(index_iter))
        } else {
            let mut kv_iter = db.metadata_db().iter::<StateValueSchema>(read_opts)?;
            if let Some(first_key) = &first_key {
                kv_iter.seek(&(first_key.clone(), u64::MAX))?;
            } else {
                kv_iter.seek(&&key_prefix)?;
            };
            (Some(kv_iter), None)
        };
        Ok(Self {
            db,
            kv_iter,
            index_iter,
            key_prefix,
            prev_key: None,
            desired_version,
            is_finished: false,
            use_index,
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

    fn next_by_index(&mut self) -> Result<Option<(StateKey, StateValue)>> {
        let iter = self.index_iter.as_mut().unwrap();
        if !self.is_finished {
            while let Some(((state_key, version), _)) = iter.next().transpose()? {
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

                if let Some(state_value) = self
                    .db
                    .db_shard(state_key.get_shard_id())
                    .get::<StateValueSchema>(&(state_key.clone(), version))?
                    .ok_or_else(|| {
                        AptosDbError::NotFound(format!(
                            "Key {state_key:?} is not found at version {version}.",
                            state_key = state_key,
                            version = version
                        ))
                    })?
                {
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
        if self.use_index {
            self.next_by_index().transpose()
        } else {
            self.next_by_kv().transpose()
        }
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
            },
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
