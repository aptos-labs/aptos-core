// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::event::EventSchema;
use crate::ledger_info::LedgerInfoSchema;
use crate::transaction_by_account::TransactionByAccountSchema;
use anyhow::{anyhow, ensure, Result};
use aptos_types::account_address::AccountAddress;
use aptos_types::contract_event::ContractEvent;
use aptos_types::ledger_info::LedgerInfoWithSignatures;
use aptos_types::transaction::Version;
use schemadb::iterator::SchemaIterator;
use std::iter::Peekable;
use std::marker::PhantomData;

pub struct ContinuousVersionIter<I, T> {
    inner: I,
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
                    "{} iterator: expecting version {}, got {} from underlying iterator.",
                    std::any::type_name::<T>(),
                    self.expected_next_version,
                    version,
                );
                self.expected_next_version += 1;
                Some(transaction)
            }
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
            expected_next_version: first_version,
            end_version: first_version
                .checked_add(limit as u64)
                .ok_or_else(|| anyhow!("Too many items requested"))?,
            _phantom: Default::default(),
        })
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
                    ensure!(epoch == self.next_epoch, "Epochs are not consecutive.");
                    self.next_epoch += 1;
                    Some(li)
                }
            }
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
                .map_err(|e| anyhow!("Hit error iterating events: {}", e))?;
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
            .ok_or_else(|| anyhow!("expected version overflowed."))?;
        Ok(Some(ret))
    }
}

impl<'a> Iterator for EventsByVersionIter<'a> {
    type Item = Result<Vec<ContractEvent>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_impl().transpose()
    }
}
