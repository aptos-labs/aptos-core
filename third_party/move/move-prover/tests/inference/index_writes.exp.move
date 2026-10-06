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
    spec set_elem(v: &mut vector<u64>, i: u64, x: u64) {
        pragma opaque = true;
        ensures [inferred] v == update(old(v), i, x);
        aborts_if [inferred] !in_range(v, i);
    }


    fun set_index(v: &mut vector<u64>, i: u64, x: u64) {
        v[i] = x;
    }
    spec set_index(v: &mut vector<u64>, i: u64, x: u64) {
        pragma opaque = true;
        ensures [inferred] v == update(old(v), i, x);
        aborts_if [inferred] !in_range(v, i);
    }


    fun incr_elem(v: &mut vector<u64>, i: u64) {
        let r = &mut v[i];
        *r = *r + 1;
    }
    spec incr_elem(v: &mut vector<u64>, i: u64) {
        pragma opaque = true;
        ensures [inferred] v == update(old(v), i, old(v)[i] + 1);
        aborts_if [inferred] !in_range(v, i);
        aborts_if [inferred] v[i] == MAX_U64;
    }


    fun set_field_elem(s: &mut S, i: u64, x: u64) {
        *vector::borrow_mut(&mut s.v, i) = x;
    }
    spec set_field_elem(s: &mut S, i: u64, x: u64) {
        pragma opaque = true;
        ensures [inferred] s == update_field(old(s), v, update(old(s).v, i, x));
        aborts_if [inferred] !in_range(s.v, i);
    }


    fun set_local(x: u64): vector<u64> {
        let v = vector[1, 2, 3];
        v[1] = x;
        v
    }
    spec set_local(x: u64): vector<u64> {
        pragma opaque = true;
        ensures [inferred] result == update(vector[1, 2, 3], 1, x);
        aborts_if [inferred] !in_range(vector[1, 2, 3], 1);
    }


    fun set_entry(m: &mut SimpleMap<u64, u64>, k: u64, x: u64) {
        *simple_map::borrow_mut(m, &k) = x;
    }
    spec set_entry(m: &mut 0x1::simple_map::SimpleMap<u64, u64>, k: u64, x: u64) {
        use 0x1::simple_map;
        pragma opaque = true;
        ensures [inferred] m == simple_map::spec_set<u64, u64>(old(m), k, x);
        aborts_if [inferred] simple_map::spec_aborts_borrow<u64, u64>(m, k);
    }


    fun set_table(t: &mut Table<u64, u64>, k: u64, x: u64) {
        *table::borrow_mut(t, k) = x;
    }
    spec set_table(t: &mut 0x1::table::Table<u64, u64>, k: u64, x: u64) {
        use 0x1::table;
        pragma opaque = true;
        ensures [inferred] t == table::spec_set<u64, u64>(old(t), k, x);
        aborts_if [inferred] !table::spec_contains<u64, u64>(t, k);
    }


    fun set_default(t: &mut Table<u64, u64>, k: u64, x: u64) {
        *table::borrow_mut_with_default(t, k, 0) = x;
    }
    spec set_default(t: &mut 0x1::table::Table<u64, u64>, k: u64, x: u64) {
        use 0x1::table;
        pragma opaque = true;
        ensures [inferred] t == table::spec_set<u64, u64>(if (table::spec_contains<u64, u64>(old(t), k)) old(t) else table::spec_set<u64, u64>(old(t), k, 0), k, x);
        aborts_if [inferred] false;
    }


    fun incr_default(t: &mut Table<u64, u64>, k: u64) {
        let r = table::borrow_mut_with_default(t, k, 0);
        *r = *r + 1;
    }
    spec incr_default(t: &mut 0x1::table::Table<u64, u64>, k: u64) {
        use 0x1::table;
        pragma opaque = true;
        ensures [inferred] t == table::spec_set<u64, u64>(if (table::spec_contains<u64, u64>(old(t), k)) old(t) else table::spec_set<u64, u64>(old(t), k, 0), k, (if (table::spec_contains<u64, u64>(old(t), k)) table::spec_get<u64, u64>(old(t), k) else 0) + 1);
        aborts_if [inferred] (if (table::spec_contains<u64, u64>(t, k)) table::spec_get<u64, u64>(t, k) else 0) == MAX_U64;
    }


    fun set_found(m: &mut BigOrderedMap<u64, u64>, k: u64, x: u64) {
        let it = big_ordered_map::internal_find(m, &k);
        if (!it.iter_is_end(m)) {
            *it.iter_borrow_mut(m) = x + 0;
        }
    }
    spec set_found(m: &mut 0x1::big_ordered_map::BigOrderedMap<u64, u64>, k: u64, x: u64) {
        use 0x1::big_ordered_map;
        pragma opaque = true;
        ensures [inferred] !big_ordered_map::iter_is_end<u64, u64>(result_of<big_ordered_map::internal_find<u64, u64>>(old(m), k), old(m)) && big_ordered_map::spec_iter_valid<u64, u64>(result_of<big_ordered_map::internal_find<u64, u64>>(old(m), k), m) ==> m == big_ordered_map::spec_set<u64, u64>(old(m), result_of<big_ordered_map::internal_find<u64, u64>>(old(m), k).key, x);
        ensures [inferred] big_ordered_map::iter_is_end<u64, u64>(result_of<big_ordered_map::internal_find<u64, u64>>(old(m), k), old(m)) ==> m == old(m);
        aborts_if [inferred] !big_ordered_map::iter_is_end<u64, u64>(result_of<big_ordered_map::internal_find<u64, u64>>(m, k), m) && big_ordered_map::spec_iter_valid<u64, u64>(result_of<big_ordered_map::internal_find<u64, u64>>(m, k), m) && big_ordered_map::spec_aborts_iter_borrow_mut<u64, u64>(result_of<big_ordered_map::internal_find<u64, u64>>(m, k), m);
    }


    fun n_mut(s: &mut S): &mut u64 {
        &mut s.n
    }
    spec n_mut(s: &mut S): &mut u64 {
        pragma opaque = true;
        ensures [inferred] result == s.n;
        ensures [inferred] s == old(s);
        aborts_if [inferred] false;
    }


    fun set_through_field(s: &mut S, x: u64) {
        *n_mut(s) = x;
    }
    spec set_through_field(s: &mut S, x: u64) {
        pragma opaque = true;
        ensures [inferred = sathard] exists y: S: ensures_of<n_mut>(old(s), result_of<n_mut>(old(s)), y) && s == update_field(y, n, x);
        aborts_if [inferred] false;
    }


    fun set_ordered(m: &mut OrderedMap<u64, u64>, k: u64, x: u64) {
        let it = ordered_map::internal_find(m, &k);
        if (!it.iter_is_end(m)) {
            *it.iter_borrow_mut(m) = x;
        }
    }
    spec set_ordered(m: &mut 0x1::ordered_map::OrderedMap<u64, u64>, k: u64, x: u64) {
        use 0x1::ordered_map;
        pragma opaque = true;
        ensures [inferred] m == (if (!ordered_map::iter_is_end<u64, u64>(result_of<ordered_map::internal_find<u64, u64>>(old(m), k), old(m))) ordered_map::spec_set<u64, u64>(old(m), ordered_map::spec_key_at<u64, u64>(old(m), result_of<ordered_map::internal_find<u64, u64>>(old(m), k).index), x) else old(m));
        aborts_if [inferred] !ordered_map::iter_is_end<u64, u64>(result_of<ordered_map::internal_find<u64, u64>>(m, k), m) && ordered_map::spec_aborts_iter_borrow_mut<u64, u64>(result_of<ordered_map::internal_find<u64, u64>>(m, k), m);
    }

}
/*
Verification: Succeeded.
Mutation: Rejected by postcondition.
Mutation: Rejected by postcondition.
Mutation: Rejected by postcondition.
Mutation: Rejected by postcondition.
*/
