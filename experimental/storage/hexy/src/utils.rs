// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

/// Sorta vector of key-value pairs by the keep and then deduplicate items, keeping the last value
/// for each key.
pub fn sort_dedup<K: Ord, V>(mut data: Vec<(K, V)>) -> Vec<(K, V)> {
    // Stable sort by key.
    data.sort_by(|a, b| key(a).cmp(key(b)));

    // Dedup keeping the last value.
    let mut out_cur = 0;
    for in_cur in 1..=data.len() {
        if in_cur == data.len() || key(&data[in_cur]) != key(&data[in_cur - 1]) {
            if in_cur != out_cur + 1 {
                data.swap(in_cur - 1, out_cur);
            }
            out_cur += 1;
        }
    }
    data.truncate(out_cur);
    data
}

fn key<'k, 'kv: 'k, K: 'k, V>(kv: &'kv (K, V)) -> &'k K {
    &kv.0
}
