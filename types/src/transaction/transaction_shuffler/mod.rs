// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Defines the traits related to the shuffled transaction iterator because the structs
//! they are implemented for (e.g.,
//! [`SignedTransaction`](crate::transaction::SignedTransaction), and
//! [`SignatureVerifiedTransaction`](crate::transaction::signature_verified_transaction::SignatureVerifiedTransaction))
//! are also defined in this crate. This ensures that the traits and their implementations are
//! co-located, adhering to Rust's orphan rule, and making the codebase easier to navigate and maintain.

pub mod iterator;
pub mod iterator_item;
