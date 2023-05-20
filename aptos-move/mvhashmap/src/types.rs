// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::delta_change_set::DeltaOp;
use aptos_crypto::hash::HashValue;
use aptos_types::executable::ExecutableDescriptor;
use std::sync::Arc;

pub type TxnIndex = u32;
pub type Incarnation = u32;
pub type Version = (TxnIndex, Incarnation);

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Flag {
    Done,
    Estimate,
}

/// Returned as Err(..) when failed to read from the multi-version data-structure.
#[derive(Debug, PartialEq, Eq)]
pub enum MVDataError {
    /// No prior entry is found.
    NotFound,
    /// Read resulted in an unresolved delta value.
    Unresolved(DeltaOp),
    /// A dependency on other transaction has been found during the read.
    Dependency(TxnIndex),
    /// Delta application failed, txn execution should fail.
    DeltaApplicationFailure,
}

#[derive(Debug, PartialEq, Eq)]
pub enum MVModulesError {
    /// No prior entry is found.
    NotFound,
    /// A dependency on other transaction has been found during the read.
    Dependency(TxnIndex),
}

/// Returned as Ok(..) when read successfully from the multi-version data-structure.
#[derive(Debug, PartialEq, Eq)]
pub enum MVDataOutput<V> {
    /// Result of resolved delta op, always u128. Unlike with `Version`, we return
    /// actual data because u128 is cheap to copy and validation can be done correctly
    /// on values as well (ABA is not a problem).
    Resolved(u128),
    /// Information from the last versioned-write. Note that the version is returned
    /// and not the data to avoid copying big values around.
    Versioned(Version, Arc<V>),
}

/// Returned as Ok(..) when read successfully from the multi-version data-structure.
#[derive(Debug, PartialEq, Eq)]
pub enum MVModulesOutput<M, X> {
    /// Arc to the executable corresponding to the latest module, and a descriptor
    /// with either the module hash or indicator that the module is from storage.
    Executable((Arc<X>, ExecutableDescriptor)),
    /// Arc to the latest module, together with its (cryptographic) hash. Note that
    /// this can't be a storage-level module, as it's from multi-versioned modules map.
    /// The Option can be None if HashValue can't be computed, currently may happen
    /// if the latest entry corresponded to the module deletion.
    Module((Arc<M>, HashValue)),
}
