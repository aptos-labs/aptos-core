// Stress test for garbage collection. It repeatedly grows and shrinks a list
// of records; each record holds a number and a small list of numbers. A simple
// random generator written in Move drives the changes, so both VMs run the same
// steps. Collection is forced over and over, so the collector keeps moving the
// nested data around. At the end the surviving data is reduced to one number.
// The old VM never collects and the new one does, so they must agree on it.

// RUN: publish
module 0x42::gc_stress {
    use std::vector;

    struct Entry has drop { key: u64, values: vector<u64> }

    fun next_rand(state: &mut u64): u64 {
        let x = *state;
        x = x ^ (x << 13);
        x = x ^ (x >> 7);
        x = x ^ (x << 17);
        *state = x;
        x
    }

    fun make_entry(val: u64): Entry {
        let values = vector::empty<u64>();
        vector::push_back(&mut values, val);
        Entry { key: val, values }
    }

    public fun churn(n: u64, max_len: u64): u64 {
        let outer = vector::empty<Entry>();
        let state: u64 = 88172645463325252;
        let i = 0;
        while (i < n) {
            if (i % 8 == 0) {
                0x0::test_utils::forge_gc();
            };
            let r = next_rand(&mut state) % 100;
            if (r < 30) {
                // Push, evicting the oldest entry when at capacity.
                let val = next_rand(&mut state);
                if (vector::length(&outer) >= max_len) {
                    vector::pop_back(&mut outer);
                };
                vector::push_back(&mut outer, make_entry(val));
            } else if (r < 45) {
                if (vector::length(&outer) > 0) {
                    vector::pop_back(&mut outer);
                };
            } else {
                // Allocate an entry and discard it, producing garbage.
                let val = next_rand(&mut state);
                let _g = make_entry(val);
            };
            i = i + 1;
        };

        // Fold the surviving structure into an order-sensitive digest.
        let mod_v: u64 = 1000000007;
        let digest: u64 = 0;
        let k = 0;
        let len = vector::length(&outer);
        while (k < len) {
            let e = vector::borrow(&outer, k);
            let acc = e.key % mod_v;
            let j = 0;
            let m = vector::length(&e.values);
            while (j < m) {
                acc = (acc + *vector::borrow(&e.values, j) % mod_v) % mod_v;
                j = j + 1;
            };
            digest = (digest * 31 + acc) % mod_v;
            k = k + 1;
        };
        digest
    }
}

// RUN: execute 0x42::gc_stress::churn --args 500, 16
// CHECK: results: 755187010
