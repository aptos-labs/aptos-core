// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    in_mem::{base::HexyBase, overlay::HexyOverlay},
    utils::sort_dedup,
    LeafIdx, NodePosition, ARITY,
};
use velor_crypto::{
    hash::{CryptoHasher, HexyHasher, HOT_STATE_PLACE_HOLDER_HASH},
    HashValue,
};
use velor_infallible::Mutex;
use itertools::Itertools;
use proptest::{collection::vec, prelude::*};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

fn arb_test_case() -> impl Strategy<Value = (LeafIdx, Vec<Vec<(LeafIdx, HashValue)>>)> {
    (1u32..1000, 1usize..100).prop_flat_map(|(num_leaves, num_batches)| {
        (
            Just(num_leaves),
            vec(
                vec((0..(num_leaves as LeafIdx), any::<HashValue>()), 0..100),
                num_batches,
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

fn sleep_random() {
    let micros = rand::random::<u64>() % 100;
    std::thread::sleep(std::time::Duration::from_micros(micros));
}

fn update_fn(
    base: Arc<HexyBase>,
    base_overlay: Arc<Mutex<HexyOverlay>>,
    latest: Arc<Mutex<HexyOverlay>>,
    updates: Vec<Vec<(LeafIdx, HashValue)>>,
) -> impl FnOnce() {
    move || {
        for batch in updates.into_iter() {
            let view = latest.lock().view(&base, &base_overlay.lock());
            sleep_random();
            *latest.lock() = view.new_overlay(batch.clone()).unwrap();
        }
    }
}

fn merge_fn(
    base: Arc<HexyBase>,
    base_overlay: Arc<Mutex<HexyOverlay>>,
    latest: Arc<Mutex<HexyOverlay>>,
    quit_signal: Arc<AtomicBool>,
) -> impl FnOnce() {
    move || {
        let mut quit = false;
        while !quit {
            quit = quit_signal.load(Ordering::Acquire);
            sleep_random();

            let base_overlay_ = base_overlay.lock().clone();
            let latest_ = latest.lock().clone();
            if latest_.is_the_same(&base_overlay_) {
                continue;
            }
            base.merge(
                latest_
                    .overlay
                    .into_layers_view_after(base_overlay_.overlay.clone()),
            )
            .unwrap();
            *base_overlay.lock() = base_overlay_;
        }
    }
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
        let root_overlay = HexyOverlay::new_empty(&base);
        let base_overlay = Arc::new(Mutex::new(root_overlay.clone()));
        let latest = Arc::new(Mutex::new(root_overlay));
        let quit_signal = Arc::new(AtomicBool::new(false));

        let root_hash = naive_root_hash(num_leaves, &updates);

        let update_thread = std::thread::spawn(
            update_fn(base.clone(), base_overlay.clone(), latest.clone(), updates)
        );
        let merge_thread = std::thread::spawn(
            merge_fn(base.clone(), base_overlay.clone(), latest.clone(), quit_signal.clone())
        );

        update_thread.join().unwrap();
        prop_assert_eq!(latest.lock().root_hash, root_hash);

        quit_signal.store(true, Ordering::Release);
        merge_thread.join().unwrap();
        prop_assert_eq!(base.root_hash(), root_hash);
    }
}

#[test]
fn test_get_hash() {
    let base = Arc::new(HexyBase::allocate(17));
    // level 0: 0-16
    // level 1: 0-1
    // level 2 (root): 0

    unsafe {
        base.unsafe_get_hash(NodePosition::height_and_index(0, 0))
            .unwrap();
        base.unsafe_get_hash(NodePosition::height_and_index(0, 16))
            .unwrap();
        // 31 should work, since it's one of 16's siblings.
        base.unsafe_get_hash(NodePosition::height_and_index(0, 31))
            .unwrap();
        assert!(base
            .unsafe_get_hash(NodePosition::height_and_index(0, 32))
            .is_err());

        base.unsafe_get_hash(NodePosition::height_and_index(1, 0))
            .unwrap();
        base.unsafe_get_hash(NodePosition::height_and_index(1, 1))
            .unwrap();
        base.unsafe_get_hash(NodePosition::height_and_index(1, 7))
            .unwrap();
        assert!(base
            .unsafe_get_hash(NodePosition::height_and_index(1, 16))
            .is_err());

        base.unsafe_get_hash(NodePosition::height_and_index(2, 0))
            .unwrap();
        assert!(base
            .unsafe_get_hash(NodePosition::height_and_index(2, 1))
            .is_err());

        assert!(base
            .unsafe_get_hash(NodePosition::height_and_index(3, 0))
            .is_err());
    }
}
