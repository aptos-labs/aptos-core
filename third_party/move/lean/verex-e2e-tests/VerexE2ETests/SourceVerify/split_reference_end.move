// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A loop whose body takes a `&mut` of the whole vector: `drain`'s authored
// `compute` route leaves `data_invariant` unproved while a prepared leaf's
// context holds that invariant fact; the first missing correspondence and
// cause are unlocalized. `drain_automatic` is the control.
module 0x42::split_reference_end {
    use std::vector;

    fun drain<T: drop>(v: vector<T>) {
        let n = vector::length(&v);
        while (n > 0) {
            let r = &mut v;
            vector::pop_back(r);
            n = n - 1;
        } spec {
            invariant n == len(v);
        };
        vector::destroy_empty(v);
    }
    spec drain {
        aborts_if false;
    }

    fun drain_automatic<T: drop>(v: vector<T>) {
        let n = vector::length(&v);
        while (n > 0) {
            let r = &mut v;
            vector::pop_back(r);
            n = n - 1;
        } spec {
            invariant n == len(v);
        };
        vector::destroy_empty(v);
    }
    spec drain_automatic {
        aborts_if false;
    }
}
