// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;
use aptos_crypto::hash::CryptoHasher;
use aptos_crypto::HashValue;
use aptos_types::proof::AccumulatorProof;
use aptos_types::proof::position::Position;
use crate::HashValueArrayRead;

struct MerklarrUpdater<'a, ArrayRead, Hasher> {
    array: &'a ArrayRead,
    num_items: u64,
    hasher: PhantomData<Hasher>,
}

impl<'a, ArrayRead, Hasher> MerklarrUpdater<'a, ArrayRead, Hasher>
    where
        ArrayRead: HashValueArrayRead,
        Hasher: CryptoHasher,
{
    fn update(&self, _updates: &[(ItemIndex, HashValue)]) -> Vec<(ArrayIndex, HashValue)> {
        todo!()
    }
}
