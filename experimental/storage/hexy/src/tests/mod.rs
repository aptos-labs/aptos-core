// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    in_mem::{base::HexyBase, overlay::HexyOverlay},
    utils::sort_dedup,
    LeafIdx, ARITY,
};
use aptos_crypto::{
    hash::{CryptoHasher, HexyHasher, HOT_STATE_PLACE_HOLDER_HASH},
    HashValue,
};
use itertools::Itertools;
use proptest::{collection::vec, prelude::*};
use std::{collections::BTreeMap, sync::Arc};

fn arb_test_case() -> impl Strategy<Value = (LeafIdx, Vec<Vec<(LeafIdx, HashValue)>>)> {
    (1u32..1000).prop_flat_map(|num_leaves| {
        (
            Just(num_leaves),
            vec(
                vec((0..(num_leaves as LeafIdx), any::<HashValue>()), 0..100),
                0..100,
            ),
        )
    })
}

fn naive_root_hash(num_leaves: LeafIdx, updates: &[Vec<(LeafIdx, HashValue)>]) -> HashValue {
    let mut hashes = vec![*HOT_STATE_PLACE_HOLDER_HASH; num_leaves as usize];
    let all_updates = updates
        .iter()
        .flatten()
        .cloned()
        .collect::<BTreeMap<_, _>>();
    all_updates
        .into_iter()
        .for_each(|(idx, hash)| hashes[idx as usize] = hash);

    while hashes.len() > 1 {
        hashes = hashes
            .into_iter()
            .chunks(ARITY)
            .into_iter()
            .map(|chunk| {
                let mut children = chunk.into_iter().collect_vec();
                children.resize_with(ARITY, || *HOT_STATE_PLACE_HOLDER_HASH);

                if children
                    .iter()
                    .all(|hash| hash == &*HOT_STATE_PLACE_HOLDER_HASH)
                {
                    *HOT_STATE_PLACE_HOLDER_HASH
                } else {
                    let mut hasher = HexyHasher::default();
                    for child in children {
                        hasher.update(child.as_slice())
                    }
                    hasher.finish()
                }
            })
            .collect_vec()
    }

    hashes
        .first()
        .cloned()
        .unwrap_or(*HOT_STATE_PLACE_HOLDER_HASH)
}

proptest! {
    #[test]
    fn test_sort_dedup(data in vec(any::<(u16, u16)>(), 0..100)) {
        let sort_debuped = sort_dedup(data.clone());
        let expected = data.into_iter().collect::<BTreeMap<_, _>>().into_iter().collect_vec();

        assert_eq!(sort_debuped, expected);
    }

    #[test]
    fn test_update((num_leaves, updates) in arb_test_case()) {
        let base = Arc::new(HexyBase::allocate(num_leaves));
        let bottom_overlay = HexyOverlay::new_empty(&base);
        let mut top_overlay = bottom_overlay.clone();

        for batch in updates.iter() {
            let view = top_overlay.view(&base, &bottom_overlay);
            top_overlay = view.new_overlay(batch.clone()).unwrap();
        }

        prop_assert_eq!(
            top_overlay.root_hash,
            naive_root_hash(num_leaves, &updates)
        )
    }
}
