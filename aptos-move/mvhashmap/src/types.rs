// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::{
    delta_change_set::DeltaOp,
    types::{DelayedFieldsSpeculativeError, PanicOr},
};
use aptos_crypto::hash::HashValue;
use aptos_types::executable::ExecutableDescriptor;
use move_core_types::value::MoveTypeLayout;
use std::sync::{atomic::AtomicU32, Arc};

pub type AtomicTxnIndex = AtomicU32;
pub type TxnIndex = u32;
pub type Incarnation = u32;

/// Custom error type representing storage version. Result<Index, StorageVersion>
/// then represents either index of some type (i.e. TxnIndex, Version), or a
/// version corresponding to the storage (pre-block) state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StorageVersion;

// TODO: Find better representations for this, a similar one for TxnIndex.
pub type Version = Result<(TxnIndex, Incarnation), StorageVersion>;

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Flag {
    Done,
    Estimate,
}

#[derive(Debug, PartialEq, Eq)]
pub enum MVGroupError {
    /// The base group contents are not initialized.
    Uninitialized,
    /// Entry corresponding to the tag was not found.
    TagNotFound,
    /// A dependency on other transaction has been found during the read.
    Dependency(TxnIndex),
    /// Tag serialization is needed for group size computation
    TagSerializationError,
}

/// Returned as Err(..) when failed to read from the multi-version data-structure.
#[derive(Debug, PartialEq, Eq)]
pub enum MVDataError {
    /// No prior entry is found.
    Uninitialized,
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
    Versioned(Version, Arc<V>, Option<Arc<MoveTypeLayout>>),
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

// TODO: once VersionedAggregators is separated from the MVHashMap, seems that
// MVDataError and MVModulesError can be unified and simplified.
#[derive(Debug, PartialEq, Eq)]
pub enum MVDelayedFieldsError {
    /// No prior entry is found. This can happen if the aggregator was created
    /// by an earlier transaction which aborted, re-executed, and did not re-create
    /// the aggregator (o.w. the ID of the aggregator provided to the reading API
    /// could not have been obtained). NOTE: We could record & return some additional
    /// information and save validations in the caller.
    NotFound,
    /// A dependency on another transaction (index returned) was found during the read.
    Dependency(TxnIndex),
    /// While reading, delta application failed at the returned transaction index
    /// (either it violated the limits when not supposed to, or vice versa).
    /// Note: we can return affected indices to optimize invalidations by the caller.
    DeltaApplicationFailure,
}

impl MVDelayedFieldsError {
    pub fn from_panic_or(
        err: PanicOr<DelayedFieldsSpeculativeError>,
    ) -> PanicOr<MVDelayedFieldsError> {
        match err {
            PanicOr::CodeInvariantError(e) => PanicOr::CodeInvariantError(e),
            PanicOr::Or(DelayedFieldsSpeculativeError::NotFound(_)) => {
                PanicOr::Or(MVDelayedFieldsError::NotFound)
            },
            PanicOr::Or(_) => PanicOr::Or(MVDelayedFieldsError::DeltaApplicationFailure),
        }
    }
}

// In order to store base vales at the lowest index, i.e. at index 0, without conflicting
// with actual transaction index 0, the following struct wraps the index and internally
// increments it by 1.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub(crate) struct ShiftedTxnIndex {
    idx: TxnIndex,
}

impl ShiftedTxnIndex {
    pub fn new(real_idx: TxnIndex) -> Self {
        Self { idx: real_idx + 1 }
    }

    pub(crate) fn idx(&self) -> Result<TxnIndex, StorageVersion> {
        if self.idx > 0 {
            Ok(self.idx - 1)
        } else {
            Err(StorageVersion)
        }
    }

    pub(crate) fn zero() -> Self {
        Self { idx: 0 }
    }
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;
    use aptos_aggregator::delta_change_set::serialize;
    use aptos_types::{
        access_path::AccessPath, executable::ModulePath, state_store::state_value::StateValue,
        write_set::TransactionWrite,
    };
    use bytes::Bytes;
    use claims::{assert_err, assert_ok_eq};
    use std::{fmt::Debug, hash::Hash, sync::Arc};

    #[derive(Clone, Eq, Hash, PartialEq, Debug)]
    pub(crate) struct KeyType<K: Hash + Clone + Debug + Eq>(
        /// Wrapping the types used for testing to add ModulePath trait implementation.
        pub K,
    );

    impl<K: Hash + Clone + Eq + Debug> ModulePath for KeyType<K> {
        fn module_path(&self) -> Option<AccessPath> {
            None
        }
    }

    #[test]
    fn test_shifted_idx() {
        let zero = ShiftedTxnIndex::zero();
        let shifted_indices: Vec<_> = (0..20).map(ShiftedTxnIndex::new).collect();
        for (i, shifted_idx) in shifted_indices.iter().enumerate() {
            assert_ne!(zero, *shifted_idx);
            for j in 0..i {
                assert_ne!(ShiftedTxnIndex::new(j as TxnIndex), *shifted_idx);
            }
            assert_eq!(ShiftedTxnIndex::new(i as TxnIndex), *shifted_idx);
        }
        assert_eq!(ShiftedTxnIndex::zero(), zero);
        assert_err!(zero.idx());

        for (i, shifted_idx) in shifted_indices.into_iter().enumerate() {
            assert_ok_eq!(shifted_idx.idx(), i as TxnIndex);
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    pub(crate) struct TestValue {
        bytes: Bytes,
    }

    impl TestValue {
        pub(crate) fn deletion() -> Self {
            Self {
                bytes: vec![].into(),
            }
        }

        pub fn new(mut seed: Vec<u32>) -> Self {
            seed.resize(4, 0);
            Self {
                bytes: seed.into_iter().flat_map(|v| v.to_be_bytes()).collect(),
            }
        }

        pub(crate) fn from_u128(value: u128) -> Self {
            Self {
                bytes: serialize(&value).into(),
            }
        }

        pub(crate) fn with_len(len: usize) -> Self {
            assert!(len > 0, "0 is deletion");
            Self {
                bytes: vec![100_u8; len].into(),
            }
        }
    }

    impl TransactionWrite for TestValue {
        fn bytes(&self) -> Option<&Bytes> {
            (!self.bytes.is_empty()).then_some(&self.bytes)
        }

        fn from_state_value(_maybe_state_value: Option<StateValue>) -> Self {
            unimplemented!("Irrelevant for the test")
        }

        fn as_state_value(&self) -> Option<StateValue> {
            unimplemented!("Irrelevant for the test")
        }

        fn set_bytes(&mut self, bytes: Bytes) {
            self.bytes = bytes;
        }
    }

    // Generate a Vec deterministically based on txn_idx and incarnation.
    pub(crate) fn value_for(txn_idx: TxnIndex, incarnation: Incarnation) -> TestValue {
        TestValue::new(vec![txn_idx * 5, txn_idx + incarnation, incarnation * 5])
    }

    // Generate the value_for txn_idx and incarnation in arc.
    pub(crate) fn arc_value_for(txn_idx: TxnIndex, incarnation: Incarnation) -> Arc<TestValue> {
        // Generate a Vec deterministically based on txn_idx and incarnation.
        Arc::new(value_for(txn_idx, incarnation))
    }

    // Convert value for txn_idx and incarnation into u128.
    pub(crate) fn u128_for(txn_idx: TxnIndex, incarnation: Incarnation) -> u128 {
        value_for(txn_idx, incarnation).as_u128().unwrap().unwrap()
    }
}
