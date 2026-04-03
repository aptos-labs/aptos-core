// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
use std::collections::{BTreeMap, BTreeSet};

pub fn remap_set<T: Copy + Ord>(set: &mut BTreeSet<T>, id_map: &BTreeMap<T, T>) {
    let _before = set.len();
    *set = std::mem::take(set)
        .into_iter()
        .map(|x| id_map.get(&x).copied().unwrap_or(x))
        .collect();
    let _after = set.len();
    debug_assert!(_before == _after);
}
