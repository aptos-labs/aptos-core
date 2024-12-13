// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// This trait is a container for a [`TransactionShufflerIteratorItem`](crate::transaction::TransactionShufflerIteratorItem)
/// It uses the Iterator super trait to ensure that items inside the Iterator are iterable
///
/// The naming is a bit confusing. Technically it is an "iterator" because it implements the Iterator
/// trait. Perhaps `TransactionShufflerScheduler` would be a better name since it can be applied
/// to a very complex struct with fields that just happens to implement Iterator, not something
/// simpler like a `Vec<SignedTransaction>`
///
/// ```md
/// impl TransactionShufflerIteratorItem for SignedTransaction {
///     ...
/// }
///
/// pub struct UseCaseAwareTransactionShufflerIterator(SignedTransaction)
///
/// impl TransactionShufflerIterator for UseCaseAwareTransactionShufflerIterator { ... }
/// ```
pub trait TransactionShufflerIterator: Iterator {}
