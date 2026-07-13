// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_aggregator::types::DelayedFieldsSpeculativeError;
use aptos_types::error::PanicOr;
use move_core_types::value::MoveTypeLayout;
use std::sync::atomic::AtomicU32;

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
    /// A dependency on other transaction has been found during the read.
    Dependency(TxnIndex),
}

/// Returned as Ok(..) when read successfully from the multi-version data-structure.
#[derive(Debug, PartialEq, Eq)]
pub enum MVDataOutput<V> {
    /// Information from the last versioned-write. Note that the version is returned
    /// and not the data to avoid copying big values around.
    Versioned(Version, V),
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

#[derive(Clone, Debug)]
pub enum UnknownOrLayout<'a> {
    Unknown,
    // TODO: Make this Arc<MoveTypeLayout> to avoid deep cloning.
    Known(Option<&'a MoveTypeLayout>),
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;
    use aptos_types::{
        block_executor::value::SpeculativeValue,
        state_store::state_value::StateValue,
        write_set::{TransactionWrite, WriteOpKind},
    };
    use bytes::Bytes;
    use claims::{assert_err, assert_ok_eq};
    use std::{fmt::Debug, hash::Hash};

    #[derive(Clone, Eq, Hash, PartialEq, Debug)]
    pub(crate) struct KeyType<K: Hash + Clone + Debug + Eq>(pub K);

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
                bytes: value.to_be_bytes().to_vec().into(),
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

    impl SpeculativeValue for TestValue {
        fn eq_value(&self, other: &Self) -> bool {
            self == other
        }

        fn eq_metadata(&self, _other: &Self) -> bool {
            unimplemented!("Irrelevant for the test")
        }

        fn bytes_len(&self) -> Option<usize> {
            (!self.bytes.is_empty()).then_some(self.bytes.len())
        }

        fn write_op_kind(&self) -> WriteOpKind {
            self.kind.clone()
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
    pub(crate) fn value_for(txn_idx: TxnIndex, incarnation: Incarnation) -> TestValue {
        TestValue::new(vec![txn_idx * 5, txn_idx + incarnation, incarnation * 5])
    }
}
