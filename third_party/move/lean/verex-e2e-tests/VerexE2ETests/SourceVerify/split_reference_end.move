// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A loop whose body takes a `&mut` of the whole vector: the reference's end
// splits on the loop's state, and the split leaves a pattern equation that is
// not in normal form, so `drain` is a parked gap. `drain_automatic` is the
// control: the same loop, taken by the automatic route, verifies.
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
