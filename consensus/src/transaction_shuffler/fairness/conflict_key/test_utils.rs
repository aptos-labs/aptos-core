// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction_shuffler::fairness::conflict_key::{
    ConflictKey, ConflictKeyId, ConflictKeyRegistry,
};
use proptest::prelude::*;
use std::hash::Hash;

impl ConflictKeyId {
    pub fn new_for_test(idx: usize) -> Self {
        Self(idx)
    }
}

impl ConflictKeyRegistry {
    pub fn all_exempt(num_txns: usize) -> Self {
        ConflictKeyRegistry {
            id_by_txn: vec![ConflictKeyId::new_for_test(0); num_txns],
            is_exempt_by_id: vec![true],
        }
    }

    pub fn non_conflict(num_txns: usize) -> Self {
        ConflictKeyRegistry {
            id_by_txn: (0..num_txns).map(ConflictKeyId::new_for_test).collect(),
            is_exempt_by_id: vec![false; num_txns],
        }
    }

    pub fn full_conflict(num_txns: usize) -> Self {
        ConflictKeyRegistry {
            id_by_txn: vec![ConflictKeyId::new_for_test(0); num_txns],
            is_exempt_by_id: vec![false],
        }
    }

    pub fn nums_per_key<const NUM_KEYS: usize>(nums_per_key: [usize; NUM_KEYS]) -> Self {
        Self::nums_per_round_per_key([nums_per_key])
    }

    pub fn nums_per_round_per_key<const NUM_KEYS: usize, const NUM_ROUNDS: usize>(
        nums_per_round_per_key: [[usize; NUM_KEYS]; NUM_ROUNDS],
    ) -> Self {
        let mut seq = (0..NUM_ROUNDS).flat_map(|_| 0..NUM_KEYS);
        let nums_per_key = nums_per_round_per_key.into_iter().flatten();

        ConflictKeyRegistry {
            id_by_txn: nums_per_key
                .flat_map(|num| {
                    let s = seq.next().unwrap();
                    vec![ConflictKeyId::new_for_test(s); num]
                })
                .collect(),
            is_exempt_by_id: vec![false; NUM_KEYS],
        }
    }
}

#[derive(Debug)]
struct FakeAccount {
    id: usize,
}

impl Arbitrary for FakeAccount {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        (0..10usize).prop_map(|id| FakeAccount { id }).boxed()
    }
}

#[derive(Debug)]
struct FakeModule {
    id: usize,
}

impl FakeModule {
    pub fn exempt(&self) -> bool {
        self.id % 3 == 0
    }
}

impl Arbitrary for FakeModule {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        (0..10usize).prop_map(|id| FakeModule { id }).boxed()
    }
}

#[derive(Debug)]
struct FakeEntryFun {
    module: FakeModule,
    id: usize,
}

impl Arbitrary for FakeEntryFun {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        (any::<FakeModule>(), 0..3usize)
            .prop_map(|(module, id)| FakeEntryFun { module, id })
            .boxed()
    }
}

#[derive(Debug)]
pub struct FakeTxn {
    sender: FakeAccount,
    entry_fun: FakeEntryFun,
}

impl Arbitrary for FakeTxn {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        (any::<FakeAccount>(), any::<FakeEntryFun>())
            .prop_map(|(sender, entry_fun)| FakeTxn { sender, entry_fun })
            .boxed()
    }
}

#[derive(Eq, Hash, PartialEq)]
pub(crate) struct FakeSenderKey {
    id: usize,
}

impl ConflictKey<FakeTxn> for FakeSenderKey {
    fn extract_from(txn: &FakeTxn) -> Self {
        Self { id: txn.sender.id }
    }

    fn conflict_exempt(&self) -> bool {
        false
    }
}

#[derive(Eq, Hash, PartialEq)]
pub(crate) enum FakeEntryFunModuleKey {
    Module(usize),
    Exempt,
}

impl ConflictKey<FakeTxn> for FakeEntryFunModuleKey {
    fn extract_from(txn: &FakeTxn) -> Self {
        if txn.entry_fun.module.exempt() {
            Self::Exempt
        } else {
            Self::Module(txn.entry_fun.module.id)
        }
    }

    fn conflict_exempt(&self) -> bool {
        match self {
            Self::Exempt => true,
            Self::Module(..) => false,
        }
    }
}

#[derive(Eq, Hash, PartialEq)]
pub(crate) enum FakeEntryFunKey {
    EntryFun { module: usize, function: usize },
    Exempt,
}

impl ConflictKey<FakeTxn> for FakeEntryFunKey {
    fn extract_from(txn: &FakeTxn) -> Self {
        if txn.entry_fun.module.exempt() {
            Self::Exempt
        } else {
            Self::EntryFun {
                module: txn.entry_fun.module.id,
                function: txn.entry_fun.id,
            }
        }
    }

    fn conflict_exempt(&self) -> bool {
        match self {
            Self::Exempt => true,
            Self::EntryFun { .. } => false,
        }
    }
}
