// Writes through a reference borrowed at a vector index or a map key update the
// vector or map at that index or key.
// inference-reject-mutation: *vector::borrow_mut(v, i) = x; => *vector::borrow_mut(v, i) = i;
// inference-reject-mutation: *table::borrow_mut(t, k) = x; => *table::borrow_mut(t, k) = k;
// inference-reject-mutation: *it.iter_borrow_mut(m) = x + 0; => *it.iter_borrow_mut(m) = k;
// inference-reject-mutation: *n_mut(s) = x; => *n_mut(s) = 0;
module 0x42::index_writes {
    use std::vector;
    use aptos_std::simple_map::{Self, SimpleMap};
    use aptos_std::table::{Self, Table};
    use aptos_framework::big_ordered_map::{Self, BigOrderedMap};
    use aptos_framework::ordered_map::{Self, OrderedMap};

    struct S has drop {
        v: vector<u64>,
        n: u64,
    }

    fun set_elem(v: &mut vector<u64>, i: u64, x: u64) {
        *vector::borrow_mut(v, i) = x;
    }

    fun set_index(v: &mut vector<u64>, i: u64, x: u64) {
        v[i] = x;
    }

    fun incr_elem(v: &mut vector<u64>, i: u64) {
        let r = &mut v[i];
        *r = *r + 1;
    }

    fun set_field_elem(s: &mut S, i: u64, x: u64) {
        *vector::borrow_mut(&mut s.v, i) = x;
    }

    fun set_local(x: u64): vector<u64> {
        let v = vector[1, 2, 3];
        v[1] = x;
        v
    }

    fun set_entry(m: &mut SimpleMap<u64, u64>, k: u64, x: u64) {
        *simple_map::borrow_mut(m, &k) = x;
    }

    fun set_table(t: &mut Table<u64, u64>, k: u64, x: u64) {
        *table::borrow_mut(t, k) = x;
    }

    fun set_default(t: &mut Table<u64, u64>, k: u64, x: u64) {
        *table::borrow_mut_with_default(t, k, 0) = x;
    }

    fun incr_default(t: &mut Table<u64, u64>, k: u64) {
        let r = table::borrow_mut_with_default(t, k, 0);
        *r = *r + 1;
    }

    fun set_found(m: &mut BigOrderedMap<u64, u64>, k: u64, x: u64) {
        let it = big_ordered_map::internal_find(m, &k);
        if (!it.iter_is_end(m)) {
            *it.iter_borrow_mut(m) = x + 0;
        }
    }

    fun n_mut(s: &mut S): &mut u64 {
        &mut s.n
    }

    fun set_through_field(s: &mut S, x: u64) {
        *n_mut(s) = x;
    }

    fun set_ordered(m: &mut OrderedMap<u64, u64>, k: u64, x: u64) {
        let it = ordered_map::internal_find(m, &k);
        if (!it.iter_is_end(m)) {
            *it.iter_borrow_mut(m) = x;
        }
    }
}
