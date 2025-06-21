// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction_filter::TransactionMatcher;
use aptos_crypto::HashValue;
use aptos_types::{quorum_store::BatchId, transaction::SignedTransaction, PeerId};
#[cfg(any(test, feature = "fuzzing"))]
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};

/// A batch transaction filter that applies a set of rules to determine
/// if a transaction in a batch should be allowed or denied.
///
/// Rules are applied in the order they are defined, and the first
/// matching rule determines the outcome for the transaction.
/// If no rules match, the transaction is allowed by default.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BatchTransactionFilter {
    batch_transaction_rules: Vec<BatchTransactionRule>,
}

impl BatchTransactionFilter {
    pub fn new(batch_transaction_rules: Vec<BatchTransactionRule>) -> Self {
        Self {
            batch_transaction_rules,
        }
    }

    /// Returns true iff the filter allows the transaction in the batch
    pub fn allows_transaction(
        &self,
        batch_id: BatchId,
        batch_author: PeerId,
        batch_digest: &HashValue,
        signed_transaction: &SignedTransaction,
    ) -> bool {
        // If the filter is empty, allow the transaction by default
        if self.is_empty() {
            return true;
        }

        // Check if any rule matches the batch transaction
        for batch_transaction_rule in &self.batch_transaction_rules {
            if batch_transaction_rule.matches(
                batch_id,
                batch_author,
                batch_digest,
                signed_transaction,
            ) {
                return match batch_transaction_rule {
                    BatchTransactionRule::Allow(_) => true,
                    BatchTransactionRule::Deny(_) => false,
                };
            }
        }

        true // No rules match (allow the batch transaction by default)
    }

    /// Returns an empty batch transaction filter with no rules
    pub fn empty() -> Self {
        Self {
            batch_transaction_rules: Vec::new(),
        }
    }

    /// Filters the transactions in the given batch and returns only those that are allowed
    pub fn filter_batch_transactions(
        &self,
        batch_id: BatchId,
        batch_author: PeerId,
        batch_digest: HashValue,
        transactions: Vec<SignedTransaction>,
    ) -> Vec<SignedTransaction> {
        transactions
            .into_iter()
            .filter(|txn| self.allows_transaction(batch_id, batch_author, &batch_digest, txn))
            .collect()
    }

    /// Returns true iff the filter is empty (i.e., has no rules)
    pub fn is_empty(&self) -> bool {
        self.batch_transaction_rules.is_empty()
    }
}

// These are useful test-only methods for creating and testing filters
#[cfg(any(test, feature = "fuzzing"))]
impl BatchTransactionFilter {
    /// Adds a filter that matches all batch transactions
    pub fn add_all_filter(self, allow: bool) -> Self {
        let batch_matcher = BatchTransactionMatcher::Batch(BatchMatcher::All);
        self.add_multiple_matchers_filter(allow, vec![batch_matcher])
    }

    /// Adds a filter rule that matches a specific batch ID
    pub fn add_batch_id_filter(self, allow: bool, batch_id: BatchId) -> Self {
        let batch_matcher = BatchTransactionMatcher::Batch(BatchMatcher::BatchId(batch_id));
        self.add_multiple_matchers_filter(allow, vec![batch_matcher])
    }

    /// Adds a filter rule that matches a specific batch author
    pub fn add_batch_author_filter(self, allow: bool, batch_author: PeerId) -> Self {
        let batch_matcher = BatchTransactionMatcher::Batch(BatchMatcher::BatchAuthor(batch_author));
        self.add_multiple_matchers_filter(allow, vec![batch_matcher])
    }

    /// Adds a filter rule that matches a specific batch digest
    pub fn add_batch_digest_filter(self, allow: bool, batch_digest: HashValue) -> Self {
        let batch_matcher = BatchTransactionMatcher::Batch(BatchMatcher::BatchDigest(batch_digest));
        self.add_multiple_matchers_filter(allow, vec![batch_matcher])
    }

    /// Adds a filter rule that matches a specific transaction sender
    pub fn add_sender_filter(self, allow: bool, sender: AccountAddress) -> Self {
        let transaction_matcher = TransactionMatcher::Sender(sender);
        self.add_multiple_matchers_filter(allow, vec![BatchTransactionMatcher::Transaction(
            transaction_matcher,
        )])
    }

    /// Adds a filter rule containing multiple matchers
    pub fn add_multiple_matchers_filter(
        mut self,
        allow: bool,
        batch_transaction_matchers: Vec<BatchTransactionMatcher>,
    ) -> Self {
        let transaction_rule = if allow {
            BatchTransactionRule::Allow(batch_transaction_matchers)
        } else {
            BatchTransactionRule::Deny(batch_transaction_matchers)
        };
        self.batch_transaction_rules.push(transaction_rule);

        self
    }
}

/// A batch transaction rule that defines whether to allow or deny
/// transactions in a batch based on a set of matchers. All matchers
/// must match for the rule to apply.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BatchTransactionRule {
    Allow(Vec<BatchTransactionMatcher>),
    Deny(Vec<BatchTransactionMatcher>),
}

impl BatchTransactionRule {
    /// Returns true iff the rule matches the given batch transaction. This
    /// requires that all matchers in the rule match the batch transaction.
    pub fn matches(
        &self,
        batch_id: BatchId,
        batch_author: PeerId,
        batch_digest: &HashValue,
        signed_transaction: &SignedTransaction,
    ) -> bool {
        let batch_transaction_matchers = match self {
            BatchTransactionRule::Allow(matchers) => matchers,
            BatchTransactionRule::Deny(matchers) => matchers,
        };
        batch_transaction_matchers.iter().all(|matcher| {
            matcher.matches(batch_id, batch_author, batch_digest, signed_transaction)
        })
    }
}

/// A matcher that defines the criteria for matching batches or transactions
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BatchTransactionMatcher {
    Batch(BatchMatcher),
    Transaction(TransactionMatcher),
}

impl BatchTransactionMatcher {
    /// Returns true iff the matcher matches the given batch transaction
    pub fn matches(
        &self,
        batch_id: BatchId,
        batch_author: PeerId,
        batch_digest: &HashValue,
        signed_transaction: &SignedTransaction,
    ) -> bool {
        match self {
            BatchTransactionMatcher::Batch(batch_matcher) => {
                batch_matcher.matches(batch_id, batch_author, batch_digest)
            },
            BatchTransactionMatcher::Transaction(transaction_matcher) => {
                transaction_matcher.matches(signed_transaction)
            },
        }
    }
}

/// A matcher that defines the criteria for matching batches
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BatchMatcher {
    All,                    // Matches any batch
    BatchId(BatchId),       // Matches batches with the specified ID
    BatchAuthor(PeerId),    // Matches batches authored by the specified peer
    BatchDigest(HashValue), // Matches batches with the specified digest
}

impl BatchMatcher {
    /// Returns true iff the matcher matches the given batch information
    fn matches(&self, batch_id: BatchId, batch_author: PeerId, batch_digest: &HashValue) -> bool {
        match self {
            BatchMatcher::All => true,
            BatchMatcher::BatchId(target_batch_id) => matches_batch_id(batch_id, target_batch_id),
            BatchMatcher::BatchAuthor(target_author) => {
                matches_batch_author(batch_author, target_author)
            },
            BatchMatcher::BatchDigest(target_digest) => {
                matches_batch_digest(batch_digest, target_digest)
            },
        }
    }
}

/// Returns true iff the batch ID matches the target batch ID
fn matches_batch_id(batch_id: BatchId, target_batch_id: &BatchId) -> bool {
    batch_id == *target_batch_id
}

/// Returns true iff the batch author matches the target author
fn matches_batch_author(batch_author: PeerId, target_author: &PeerId) -> bool {
    batch_author == *target_author
}

/// Returns true iff the batch digest matches the target digest
fn matches_batch_digest(batch_digest: &HashValue, target_digest: &HashValue) -> bool {
    batch_digest == target_digest
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_matches_batch_id() {
        // Create a batch ID
        let batch_id = BatchId::new_for_test(1000);

        // Verify that the batch ID matches itself
        verify_matches_batch_id(batch_id, &batch_id, true);

        // Verify that a different batch ID does not match
        let different_batch_id = BatchId::new_for_test(122);
        verify_matches_batch_id(batch_id, &different_batch_id, false);
    }

    #[test]
    fn test_matches_batch_author() {
        // Create a batch author
        let batch_author = PeerId::random();

        // Verify that the batch author matches itself
        verify_matches_batch_author(batch_author, &batch_author, true);

        // Verify that a different batch author does not match
        let different_batch_author = PeerId::random();
        verify_matches_batch_author(batch_author, &different_batch_author, false);
    }

    #[test]
    fn test_matches_batch_digest() {
        // Create a batch digest
        let batch_digest = HashValue::random();

        // Verify that the batch digest matches itself
        verify_matches_batch_digest(&batch_digest, &batch_digest, true);

        // Verify that a different batch digest does not match
        let different_batch_digest = HashValue::random();
        verify_matches_batch_digest(&batch_digest, &different_batch_digest, false);
    }

    /// Verifies that the batch ID matches the target batch ID
    fn verify_matches_batch_id(batch_id: BatchId, target_batch_id: &BatchId, matches: bool) {
        let result = matches_batch_id(batch_id, target_batch_id);
        assert_eq!(matches, result);
    }

    /// Verifies that the batch author matches the target author
    fn verify_matches_batch_author(batch_author: PeerId, target_author: &PeerId, matches: bool) {
        let result = matches_batch_author(batch_author, target_author);
        assert_eq!(matches, result);
    }

    /// Verifies that the batch digest matches the target digest
    fn verify_matches_batch_digest(
        batch_digest: &HashValue,
        target_digest: &HashValue,
        matches: bool,
    ) {
        let result = matches_batch_digest(batch_digest, target_digest);
        assert_eq!(matches, result);
    }
}
