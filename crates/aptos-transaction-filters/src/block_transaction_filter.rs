// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction_filter::TransactionMatcher;
use aptos_crypto::HashValue;
use aptos_types::transaction::SignedTransaction;
use serde::{Deserialize, Serialize};

/// A block transaction filter that applies a set of rules to determine
/// if a transaction in a block should be allowed or denied.
///
/// Rules are applied in the order they are defined, and the first
/// matching rule determines the outcome for the transaction.
/// If no rules match, the transaction is allowed by default.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BlockTransactionFilter {
    block_transaction_rules: Vec<BlockTransactionRule>,
}

impl BlockTransactionFilter {
    pub fn new(block_transaction_rules: Vec<BlockTransactionRule>) -> Self {
        Self {
            block_transaction_rules,
        }
    }

    /// Returns true iff the filter allows the transaction in the block
    pub fn allows_transaction(
        &self,
        block_id: HashValue,
        block_epoch: u64,
        block_timestamp: u64,
        signed_transaction: &SignedTransaction,
    ) -> bool {
        // If the filter is empty, allow the transaction by default
        if self.is_empty() {
            return true;
        }

        // Check if any rule matches the block transaction
        for block_transaction_rule in &self.block_transaction_rules {
            if block_transaction_rule.matches(
                block_id,
                block_epoch,
                block_timestamp,
                signed_transaction,
            ) {
                return match block_transaction_rule {
                    BlockTransactionRule::Allow(_) => true,
                    BlockTransactionRule::Deny(_) => false,
                };
            }
        }

        true // No rules match (allow the block transaction by default)
    }

    /// Returns an empty block transaction filter with no rules
    pub fn empty() -> Self {
        Self {
            block_transaction_rules: Vec::new(),
        }
    }

    /// Filters the transactions in the given block and returns only those that are allowed
    pub fn filter_block_transactions(
        &self,
        block_id: HashValue,
        block_epoch: u64,
        block_timestamp_usecs: u64,
        transactions: Vec<SignedTransaction>,
    ) -> Vec<SignedTransaction> {
        transactions
            .into_iter()
            .filter(|txn| {
                self.allows_transaction(block_id, block_epoch, block_timestamp_usecs, txn)
            })
            .collect()
    }

    /// Returns true iff the filter is empty (i.e., has no rules)
    pub fn is_empty(&self) -> bool {
        self.block_transaction_rules.is_empty()
    }
}

// These are useful test-only methods for creating and testing filters
#[cfg(any(test, feature = "fuzzing"))]
impl BlockTransactionFilter {
    /// Adds a filter that matches all block transactions
    pub fn add_all_filter(self, allow: bool) -> Self {
        let block_matcher = BlockTransactionMatcher::Block(BlockMatcher::All);
        self.add_multiple_matchers_filter(allow, vec![block_matcher])
    }

    /// Adds a block ID filter to the filter
    pub fn add_block_id_filter(self, allow: bool, block_id: HashValue) -> Self {
        let block_matcher = BlockTransactionMatcher::Block(BlockMatcher::BlockId(block_id));
        self.add_multiple_matchers_filter(allow, vec![block_matcher])
    }

    /// Adds a block epoch greater than matcher to the filter
    pub fn add_block_epoch_greater_than_filter(self, allow: bool, epoch: u64) -> Self {
        let block_matcher =
            BlockTransactionMatcher::Block(BlockMatcher::BlockEpochGreaterThan(epoch));
        self.add_multiple_matchers_filter(allow, vec![block_matcher])
    }

    /// Adds a block epoch less than matcher to the filter
    pub fn add_block_epoch_less_than_filter(self, allow: bool, epoch: u64) -> Self {
        let block_matcher = BlockTransactionMatcher::Block(BlockMatcher::BlockEpochLessThan(epoch));
        self.add_multiple_matchers_filter(allow, vec![block_matcher])
    }

    /// Adds a block timestamp greater than matcher to the filter
    pub fn add_block_timestamp_greater_than_filter(self, allow: bool, timestamp: u64) -> Self {
        let block_matcher =
            BlockTransactionMatcher::Block(BlockMatcher::BlockTimeStampGreaterThan(timestamp));
        self.add_multiple_matchers_filter(allow, vec![block_matcher])
    }

    /// Adds a block timestamp less than matcher to the filter
    pub fn add_block_timestamp_less_than_filter(self, allow: bool, timestamp: u64) -> Self {
        let block_matcher =
            BlockTransactionMatcher::Block(BlockMatcher::BlockTimeStampLessThan(timestamp));
        self.add_multiple_matchers_filter(allow, vec![block_matcher])
    }

    /// Adds a filter rule containing multiple matchers
    pub fn add_multiple_matchers_filter(
        mut self,
        allow: bool,
        block_transaction_matchers: Vec<BlockTransactionMatcher>,
    ) -> Self {
        let transaction_rule = if allow {
            BlockTransactionRule::Allow(block_transaction_matchers)
        } else {
            BlockTransactionRule::Deny(block_transaction_matchers)
        };
        self.block_transaction_rules.push(transaction_rule);

        self
    }
}

/// A block transaction rule that defines whether to allow or deny
/// transactions in a block based on a set of matchers. All matchers
/// must match for the rule to apply.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BlockTransactionRule {
    Allow(Vec<BlockTransactionMatcher>),
    Deny(Vec<BlockTransactionMatcher>),
}

impl BlockTransactionRule {
    /// Returns true iff the rule matches the given block transaction. This
    /// requires that all matchers in the rule match the block transaction.
    pub fn matches(
        &self,
        block_id: HashValue,
        block_epoch: u64,
        block_timestamp: u64,
        signed_transaction: &SignedTransaction,
    ) -> bool {
        let block_transaction_matchers = match self {
            BlockTransactionRule::Allow(matchers) => matchers,
            BlockTransactionRule::Deny(matchers) => matchers,
        };
        block_transaction_matchers.iter().all(|matcher| {
            matcher.matches(block_id, block_epoch, block_timestamp, signed_transaction)
        })
    }
}

/// A matcher that defines the criteria for matching blocks or transactions
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BlockTransactionMatcher {
    Block(BlockMatcher),
    Transaction(TransactionMatcher),
}

impl BlockTransactionMatcher {
    /// Returns true iff the matcher matches the given block transaction
    pub fn matches(
        &self,
        block_id: HashValue,
        block_epoch: u64,
        block_timestamp: u64,
        signed_transaction: &SignedTransaction,
    ) -> bool {
        match self {
            BlockTransactionMatcher::Block(block_matcher) => {
                block_matcher.matches(block_id, block_epoch, block_timestamp)
            },
            BlockTransactionMatcher::Transaction(transaction_matcher) => {
                transaction_matcher.matches(signed_transaction)
            },
        }
    }
}

/// A matcher that defines the criteria for matching blocks
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BlockMatcher {
    All,                            // Matches any block
    BlockId(HashValue),             // Matches blocks with the specified ID
    BlockEpochGreaterThan(u64),     // Matches blocks with epochs greater than the specified value
    BlockEpochLessThan(u64),        // Matches blocks with epochs less than the specified value
    BlockTimeStampGreaterThan(u64), // Matches blocks with timestamps greater than the specified value
    BlockTimeStampLessThan(u64),    // Matches blocks with timestamps less than the specified value
}

impl BlockMatcher {
    /// Returns true iff the matcher matches the given block information
    fn matches(&self, block_id: HashValue, block_epoch: u64, block_timestamp: u64) -> bool {
        match self {
            BlockMatcher::All => true,
            BlockMatcher::BlockId(target_block_id) => matches_block_id(block_id, target_block_id),
            BlockMatcher::BlockEpochGreaterThan(target_epoch) => {
                matches_epoch_greater_than(block_epoch, target_epoch)
            },
            BlockMatcher::BlockEpochLessThan(target_epoch) => {
                matches_epoch_less_than(block_epoch, target_epoch)
            },
            BlockMatcher::BlockTimeStampGreaterThan(target_timestamp) => {
                matches_timestamp_greater_than(block_timestamp, target_timestamp)
            },
            BlockMatcher::BlockTimeStampLessThan(target_timestamp) => {
                matches_timestamp_less_than(block_timestamp, target_timestamp)
            },
        }
    }
}

/// Returns true iff the block ID matches the target block ID
fn matches_block_id(block_id: HashValue, target_block_id: &HashValue) -> bool {
    block_id == *target_block_id
}

/// Returns true iff the block epoch is greater than the target epoch
fn matches_epoch_greater_than(block_epoch: u64, target_epoch: &u64) -> bool {
    block_epoch > *target_epoch
}

/// Returns true iff the block epoch is less than the target epoch
fn matches_epoch_less_than(block_epoch: u64, target_epoch: &u64) -> bool {
    block_epoch < *target_epoch
}

/// Returns true iff the block timestamp is greater than the target timestamp
fn matches_timestamp_greater_than(block_timestamp: u64, target_timestamp: &u64) -> bool {
    block_timestamp > *target_timestamp
}

/// Returns true iff the block timestamp is less than the target timestamp
fn matches_timestamp_less_than(block_timestamp: u64, target_timestamp: &u64) -> bool {
    block_timestamp < *target_timestamp
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_matches_block_id() {
        // Create a block ID
        let block_id = HashValue::random();

        // Verify that the block ID matches itself
        verify_matches_block_id(block_id, &block_id, true);

        // Verify that a different block ID does not match
        let different_block_id = HashValue::random();
        verify_matches_block_id(block_id, &different_block_id, false);
    }

    #[test]
    fn test_matches_epoch_greater_than() {
        // Create an epoch
        let epoch = 10;

        // Verify that a greater epoch matches
        verify_matches_epoch_greater_than(epoch + 1, &epoch, true);

        // Verify that an equal epoch does not match
        verify_matches_epoch_greater_than(epoch, &epoch, false);

        // Verify that a lesser epoch does not match
        verify_matches_epoch_greater_than(epoch - 1, &epoch, false);
    }

    #[test]
    fn test_matches_epoch_less_than() {
        // Create an epoch
        let epoch = 10;

        // Verify that a lesser epoch matches
        verify_matches_epoch_less_than(epoch - 1, &epoch, true);

        // Verify that an equal epoch does not match
        verify_matches_epoch_less_than(epoch, &epoch, false);

        // Verify that a greater epoch does not match
        verify_matches_epoch_less_than(epoch + 1, &epoch, false);
    }

    #[test]
    fn test_matches_timestamp_greater_than() {
        // Create a timestamp
        let timestamp = 100;

        // Verify that a greater timestamp matches
        verify_matches_timestamp_greater_than(timestamp + 1, &timestamp, true);

        // Verify that an equal timestamp does not match
        verify_matches_timestamp_greater_than(timestamp, &timestamp, false);

        // Verify that a lesser timestamp does not match
        verify_matches_timestamp_greater_than(timestamp - 1, &timestamp, false);
    }

    #[test]
    fn test_matches_timestamp_less_than() {
        // Create a timestamp
        let timestamp = 100;

        // Verify that a lesser timestamp matches
        verify_matches_timestamp_less_than(timestamp - 1, &timestamp, true);

        // Verify that an equal timestamp does not match
        verify_matches_timestamp_less_than(timestamp, &timestamp, false);

        // Verify that a greater timestamp does not match
        verify_matches_timestamp_less_than(timestamp + 1, &timestamp, false);
    }

    /// Verifies that the block ID matches the target block ID
    fn verify_matches_block_id(block_id: HashValue, target_block_id: &HashValue, matches: bool) {
        let result = matches_block_id(block_id, target_block_id);
        assert_eq!(matches, result);
    }

    /// Verifies that the block epoch is greater than the target epoch
    fn verify_matches_epoch_greater_than(block_epoch: u64, target_epoch: &u64, matches: bool) {
        let result = matches_epoch_greater_than(block_epoch, target_epoch);
        assert_eq!(matches, result);
    }

    /// Verifies that the block epoch is less than the target epoch
    fn verify_matches_epoch_less_than(block_epoch: u64, target_epoch: &u64, matches: bool) {
        let result = matches_epoch_less_than(block_epoch, target_epoch);
        assert_eq!(matches, result);
    }

    /// Verifies that the block timestamp is greater than the target timestamp
    fn verify_matches_timestamp_greater_than(
        block_timestamp: u64,
        target_timestamp: &u64,
        matches: bool,
    ) {
        let result = matches_timestamp_greater_than(block_timestamp, target_timestamp);
        assert_eq!(matches, result);
    }

    /// Verifies that the block timestamp is less than the target timestamp
    fn verify_matches_timestamp_less_than(
        block_timestamp: u64,
        target_timestamp: &u64,
        matches: bool,
    ) {
        let result = matches_timestamp_less_than(block_timestamp, target_timestamp);
        assert_eq!(matches, result);
    }
}
