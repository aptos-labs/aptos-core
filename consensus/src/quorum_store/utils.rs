// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use consensus_types::common::Round;
use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashSet},
    hash::Hash,
};

#[allow(dead_code)]
pub(crate) struct RoundExpirations<I: Ord> {
    expiries: BinaryHeap<(Reverse<Round>, I)>,
}

impl<I: Ord + Hash> RoundExpirations<I> {
    #[allow(dead_code)]
    pub(crate) fn new() -> Self {
        Self {
            expiries: BinaryHeap::new(),
        }
    }
    #[allow(dead_code)]
    pub(crate) fn add_item(&mut self, item: I, expiry_round: Round) {
        self.expiries.push((Reverse(expiry_round), item));
    }

    /// Expire and return items corresponding to round <= given (expired) round.
    #[allow(dead_code)]
    pub(crate) fn expire(&mut self, round: Round) -> HashSet<I> {
        let mut ret = HashSet::new();
        while let Some((Reverse(r), _)) = self.expiries.peek() {
            if *r <= round {
                let (_, item) = self.expiries.pop().unwrap();
                ret.insert(item);
            } else {
                break;
            }
        }
        ret
    }
}
