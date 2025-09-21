// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::{delta_change_set::DeltaOp, types::DelayedFieldsSpeculativeError};
use aptos_types::{
    error::PanicOr,
    write_set::{TransactionWrite, WriteOpKind},
};
use fail::fail_point;
use move_core_types::value::MoveTypeLayout;
use std::sync::atomic::AtomicU32;
use triomphe::Arc;

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

#[derive(Debug, PartialEq, Eq)]
pub enum MVGroupError {
    /// The base group contents are not initialized.
    Uninitialized,
    /// Entry corresponding to the tag was not found.
    TagNotFound,
    /// A dependency on other transaction has been found during the read.
    Dependency(TxnIndex),
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

/// Returned as Ok(..) when read successfully from the multi-version data-structure.
#[derive(Debug, PartialEq, Eq)]
pub enum MVDataOutput<V> {
    /// Result of resolved delta op, always u128. Unlike with `Version`, we return
    /// actual data because u128 is cheap to copy and validation can be done correctly
    /// on values as well (ABA is not a problem).
    Resolved(u128),
    /// Information from the last versioned-write. Note that the version is returned
    /// and not the data to avoid copying big values around.
    Versioned(Version, ValueWithLayout<V>),
}

// TODO[agg_v2](cleanup): once VersionedAggregators is separated from the MVHashMap,
// seems that MVDataError and MVModulesError can be unified and simplified.
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

#[derive(Debug, PartialEq, Eq)]
pub enum UnsyncGroupError {
    /// The base group contents are not initialized.
    Uninitialized,
    /// Entry corresponding to the tag was not found.
    TagNotFound,
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

    pub(crate) fn zero_idx() -> Self {
        Self { idx: 0 }
    }
}

// TODO[agg_v2](cleanup): consider adding `DoesntExist` variant.
// Currently, "not existing value" is represented as Deletion.
#[derive(Debug, PartialEq, Eq)]
pub enum ValueWithLayout<V> {
    // When we read from storage, but don't have access to layout, we can only store the raw value.
    // This should never be returned to the user, before exchange is performed.
    RawFromStorage(Arc<V>),
    // We've used the optional layout, and applied exchange to the storage value.
    // The type layout is Some if there is a delayed field in the resource.
    // The type layout is None if there is no delayed field in the resource.
    Exchanged(Arc<V>, Option<Arc<MoveTypeLayout>>),
}

impl<T> Clone for ValueWithLayout<T> {
    fn clone(&self) -> Self {
        match self {
            ValueWithLayout::RawFromStorage(value) => {
                ValueWithLayout::RawFromStorage(value.clone())
            },
            ValueWithLayout::Exchanged(value, layout) => {
                ValueWithLayout::Exchanged(value.clone(), layout.clone())
            },
        }
    }
}

impl<V: TransactionWrite> ValueWithLayout<V> {
    pub fn write_op_kind(&self) -> WriteOpKind {
        match self {
            ValueWithLayout::RawFromStorage(value) => value.write_op_kind(),
            ValueWithLayout::Exchanged(value, _) => value.write_op_kind(),
        }
    }

    pub fn bytes_len(&self) -> Option<usize> {
        fail_point!("value_with_layout_bytes_len", |_| { Some(10) });
        match self {
            ValueWithLayout::RawFromStorage(value) | ValueWithLayout::Exchanged(value, _) => {
                value.bytes().map(|b| b.len())
            },
        }
    }

    pub fn extract_value_no_layout(&self) -> &V {
        match self {
            ValueWithLayout::RawFromStorage(value) => value.as_ref(),
            ValueWithLayout::Exchanged(value, None) => value.as_ref(),
            ValueWithLayout::Exchanged(_, Some(_)) => panic!("Unexpected layout"),
        }
    }
}

#[derive(Clone, Debug)]
pub enum UnknownOrLayout<'a> {
    Unknown,
    // TODO: Make this Arc<MoveTypeLayout> to avoid deep cloning.
    Known(Option<&'a MoveTypeLayout>),
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;
    use aptos_aggregator::delta_change_set::serialize;
    use aptos_types::{
        executable::ModulePath,
        state_store::state_value::StateValue,
        write_set::{TransactionWrite, WriteOpKind},
    };
    use bytes::Bytes;
    use claims::{assert_err, assert_ok_eq};
    use move_core_types::{account_address::AccountAddress, identifier::IdentStr};
    use std::{fmt::Debug, hash::Hash};

    #[derive(Clone, Eq, Hash, PartialEq, Debug)]
    pub(crate) struct KeyType<K: Hash + Clone + Debug + Eq>(
        /// Wrapping the types used for testing to add ModulePath trait implementation.
        pub K,
    );

    impl<K: Hash + Clone + Eq + Debug> ModulePath for KeyType<K> {
        fn is_module_path(&self) -> bool {
            false
        }

        fn from_address_and_module_name(
            _address: &AccountAddress,
            _module_name: &IdentStr,
        ) -> Self {
            unreachable!("Irrelevant for test")
        }
    }

    #[test]
    fn test_shifted_idx() {
        let zero = ShiftedTxnIndex::zero_idx();
        let shifted_indices: Vec<_> = (0..20).map(ShiftedTxnIndex::new).collect();
        for (i, shifted_idx) in shifted_indices.iter().enumerate() {
            assert_ne!(zero, *shifted_idx);
            for j in 0..i {
                assert_ne!(ShiftedTxnIndex::new(j as TxnIndex), *shifted_idx);
            }
            assert_eq!(ShiftedTxnIndex::new(i as TxnIndex), *shifted_idx);
        }
        assert_eq!(ShiftedTxnIndex::zero_idx(), zero);
        assert_err!(zero.idx());

        for (i, shifted_idx) in shifted_indices.into_iter().enumerate() {
            assert_ok_eq!(shifted_idx.idx(), i as TxnIndex);
        }
    }

    // Kind is set to Creation by default as that makes sense for providing
    // group base values (used in some tests), and most tests do not care about
    // the kind. Otherwise, there are specific constructors that initialize kind
    // for the tests that care (testing group commit logic in parallel).
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) struct TestValue {
        bytes: Bytes,
        kind: WriteOpKind,
    }

    impl TestValue {
        pub(crate) fn deletion() -> Self {
            Self {
                bytes: Bytes::new(),
                kind: WriteOpKind::Deletion,
            }
        }

        pub(crate) fn with_kind(value: usize, is_creation: bool) -> Self {
            let mut s = Self::from_u128(value as u128);
            s.kind = if is_creation {
                WriteOpKind::Creation
            } else {
                WriteOpKind::Modification
            };
            s
        }

        pub(crate) fn new(mut seed: Vec<u32>) -> Self {
            seed.resize(4, 0);
            Self {
                bytes: seed.into_iter().flat_map(|v| v.to_be_bytes()).collect(),
                kind: WriteOpKind::Creation,
            }
        }

        pub(crate) fn from_u128(value: u128) -> Self {
            Self {
                bytes: serialize(&value).into(),
                kind: WriteOpKind::Creation,
            }
        }

        pub(crate) fn creation_with_len(len: usize) -> Self {
            Self {
                bytes: vec![100_u8; len].into(),
                kind: WriteOpKind::Creation,
            }
        }

        pub(crate) fn modification_with_len(len: usize) -> Self {
            Self {
                bytes: vec![100_u8; len].into(),
                kind: WriteOpKind::Modification,
            }
        }
    }

    impl TransactionWrite for TestValue {
        fn bytes(&self) -> Option<&Bytes> {
            (!self.bytes.is_empty()).then_some(&self.bytes)
        }

        fn write_op_kind(&self) -> WriteOpKind {
            self.kind.clone()
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
    fn value_for(txn_idx: TxnIndex, incarnation: Incarnation) -> TestValue {
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
