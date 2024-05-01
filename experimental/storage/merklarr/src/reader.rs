// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::hash::CryptoHasher;
use aptos_crypto::HashValue;
use aptos_types::proof::AccumulatorProof;
use aptos_types::proof::position::Position;
use std::marker::PhantomData;
use crate::HashValueArrayRead;

struct MerklarrReader<'a, ArrayRead, Hasher> {
    array: &'a ArrayRead,
    num_items: u64,
    hasher: PhantomData<Hasher>,
}

impl<'a, ArrayRead, Hasher> MerklarrReader<'a, ArrayRead, Hasher>
where
    ArrayRead: HashValueArrayRead,
    Hasher: CryptoHasher,
{
    fn get_proof(&self, index: u64) -> crate::Result<AccumulatorProof<Hasher>> {
        let root_pos = Position::root_from_leaf_count(self.num_items);
        Position::from_leaf_index(index)
            .iter_ancestor_sibling()
            .take(root_pos.level() as usize)
            .map(|pos| self.array.at(pos.to_postorder_index()))
            .collect::<crate::Result<Vec<_>>>()
            .map(AccumulatorProof::new)
    }

    fn get_root_hash(&self) -> crate::Result<HashValue> {
        self.array.at(Position::root_from_leaf_count(self.num_items).to_postorder_index())
    }
}
