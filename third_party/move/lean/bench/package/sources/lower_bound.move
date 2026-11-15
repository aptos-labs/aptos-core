// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// The binary search of `aptos_framework::ordered_map`, verified over its keys.
/// The Move counterpart of `bench/rust/lower_bound.rs`.
module bench::lower_bound {
    struct Entry<V> has drop {
        key: u64,
        value: V,
    }

    public fun binary_search<V>(key: u64, entries: &vector<Entry<V>>, start: u64, end: u64): u64 {
        let low = start;
        let high = end;
        while ({
            spec {
                invariant start <= low && low <= high && high <= end;
                invariant forall i in start..low: entries[i].key < key;
                invariant forall i in high..end: entries[i].key >= key;
            };
            low < high
        }) {
            let mid = low + (high - low) / 2;
            if (entries[mid].key < key) {
                low = mid + 1;
            } else {
                high = mid;
            }
        };
        low
    }
    spec binary_search {
        requires start <= end && end <= len(entries);
        requires forall i in start..end, j in start..end: i <= j ==> entries[i].key <= entries[j].key;
        aborts_if false;
        ensures start <= result && result <= end;
        ensures forall i in start..result: entries[i].key < key;
        ensures forall i in result..end: entries[i].key >= key;
    }

    public fun lower_bound<V>(entries: &vector<Entry<V>>, key: u64): u64 {
        binary_search(key, entries, 0, entries.length())
    }
    spec lower_bound {
        requires forall i in 0..len(entries), j in 0..len(entries): i <= j ==> entries[i].key <= entries[j].key;
        aborts_if false;
        ensures result <= len(entries);
        ensures forall i in 0..result: entries[i].key < key;
        ensures forall i in result..len(entries): entries[i].key >= key;
    }
}
