// exclude_for: path
// Copyright © Aptos Foundation
// Observations of a container while one of its elements is mutably borrowed
// through a native `borrow_mut` see the element's *current* value: the borrow
// summary's `Index` edge yields a sync site whose saved selector localizes the
// sync, for vectors and tables alike, and composing with a deeper field borrow.
// Excluded under `--path-refs`: the legacy write-back instrumentation crashes
// on these observations ("inconsistent IsParent instruction").
module 0x42::prophecy_observe_index {
    use extensions::table::{Self, Table};
    use std::vector;

    fun observe_vec_elem() {
        let v = vector[1u64, 2];
        let e = vector::borrow_mut(&mut v, 0);
        *e = 5;
        spec {
            assert v[0] == 5;
            assert v[1] == 2;
            assume v[0] == 5;
        };
        *e = 6;
        let x = *vector::borrow(&v, 0);
        spec {
            assert x == 6;
            assert x == 100; // error: intended failure guards against vacuity
        };
    }

    struct P has copy, drop, store { f: u64 }

    fun observe_elem_field() {
        let v = vector[P { f: 1 }];
        let e = vector::borrow_mut(&mut v, 0);
        let g = &mut e.f;
        *g = 5;
        spec {
            assert v[0].f == 5;
        };
        *g = 6;
        spec {
            assert v[0].f == 100; // error: intended failure guards against vacuity
        };
    }

    fun observe_table_elem(): Table<u8, u64> {
        let t = table::new<u8, u64>();
        table::add(&mut t, 1, 10);
        let e = table::borrow_mut(&mut t, 1);
        *e = 5;
        spec {
            assert table::spec_get(t, 1) == 5;
        };
        *e = 6;
        spec {
            assert table::spec_get(t, 1) == 6;
            assert table::spec_get(t, 1) == 100; // error: intended failure guards against vacuity
        };
        t
    }
}
