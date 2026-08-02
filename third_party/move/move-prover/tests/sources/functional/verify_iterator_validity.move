// Tests iterator-validity tracking for intrinsic maps: the `iterator_create`,
// `iterator_use`, and `iterator_invalidate` pragmas plus the intrinsic
// `map_iter_borrow_mut` role. Using an iterator after a structural map
// mutation must fail verification; uses without intervening mutation must
// verify.
module 0x42::iter_table {
    struct Table<phantom K: copy + drop, phantom V> has store, drop {}

    enum IteratorPtr<K: copy + drop> has copy, drop {
        End,
        Some { key: K },
    }

    // Iterator-validity ghost state: the value carries its creation epoch.
    spec IteratorPtr {
        ghost stamp: num;
    }

    spec Table {
        pragma intrinsic = map,
            map_new = new,
            map_has_key = contains,
            map_add_no_override = add,
            map_del_must_exist = remove,
            map_iter_borrow_mut = iter_borrow_mut,
            map_spec_aborts_iter_borrow_mut = spec_aborts_iter_borrow_mut,
            map_spec_new = spec_new,
            map_spec_get = spec_get,
            map_spec_set = spec_set,
            map_spec_del = spec_remove,
            map_spec_len = spec_len,
            map_spec_has_key = spec_contains;
    }

    public native fun new<K: copy + drop, V: store>(): Table<K, V>;
    public native fun contains<K: copy + drop, V>(t: &Table<K, V>, key: K): bool;
    public native fun add<K: copy + drop, V>(t: &mut Table<K, V>, key: K, val: V);
    public native fun remove<K: copy + drop, V>(t: &mut Table<K, V>, key: K): V;
    public native fun iter_borrow_mut<K: copy + drop, V>(self: IteratorPtr<K>, t: &mut Table<K, V>): &mut V;

    spec native fun spec_new<K, V>(): Table<K, V>;
    spec native fun spec_len<K, V>(t: Table<K, V>): num;
    spec native fun spec_contains<K, V>(t: Table<K, V>, k: K): bool;
    spec native fun spec_set<K, V>(t: Table<K, V>, k: K, v: V): Table<K, V>;
    spec native fun spec_remove<K, V>(t: Table<K, V>, k: K): Table<K, V>;
    spec native fun spec_get<K, V>(t: Table<K, V>, k: K): V;

    spec fun spec_aborts_iter_borrow_mut<K, V>(self: IteratorPtr<K>, t: Table<K, V>): bool {
        (self is IteratorPtr::End<K>) || !spec_contains(t, self.key)
    }

    spec iter_borrow_mut {
        pragma iterator_use;
    }

    public fun find<K: copy + drop, V>(_t: &Table<K, V>, _key: K): IteratorPtr<K> {
        abort 0
    }
    spec find {
        pragma opaque;
        pragma verify = false;
        pragma iterator_create;
        aborts_if false;
        ensures (result is IteratorPtr::Some<K>) <==> spec_contains(_t, _key);
        ensures (result is IteratorPtr::Some<K>) ==> result.key == _key;
    }

    public fun iter_borrow<K: copy + drop, V>(self: IteratorPtr<K>, _t: &Table<K, V>): &V {
        abort 0
    }
    spec iter_borrow {
        pragma opaque;
        pragma verify = false;
        pragma iterator_use;
        aborts_if (self is IteratorPtr::End<K>) || !spec_contains(_t, self.key);
        ensures result == spec_get(_t, self.key);
    }

    // Structural mutation through an iterator: consumes it and invalidates all
    // outstanding iterators of this map type.
    public fun erase<K: copy + drop, V>(self: IteratorPtr<K>, _t: &mut Table<K, V>) {
        abort 0
    }
    spec erase {
        pragma opaque;
        pragma verify = false;
        pragma iterator_use;
        pragma iterator_invalidate;
        aborts_if (self is IteratorPtr::End<K>) || !spec_contains(_t, self.key);
        ensures _t == spec_remove(old(_t), self.key);
    }

    // A wrapper carrying an iterator in a field (like `IteratorPtrWithPath`).
    // Shadows follow the iterator through field writes and write-backs, so
    // implanting a stale iterator into a fresh wrapper is caught even though
    // the wrapper itself was created at the current epoch. Field access
    // requires this module, so the scenario tests live here.
    struct IterBox<K: copy + drop> has copy, drop {
        it: IteratorPtr<K>,
        tag: u64,
    }

    public fun box_find<K: copy + drop, V>(_t: &Table<K, V>, _key: K): IterBox<K> {
        abort 0
    }
    spec box_find {
        pragma opaque;
        pragma verify = false;
        pragma iterator_create = it;
        aborts_if false;
    }

    public fun box_use<K: copy + drop, V>(_self: IterBox<K>, _t: &Table<K, V>): u64 {
        abort 0
    }
    spec box_use {
        pragma opaque;
        pragma verify = false;
        pragma iterator_use = it;
        aborts_if false;
    }

    // Forwarding without a pragma: the returned iterator's lineage is unknown,
    // so the destination's shadow is havocked (fail-closed) — even when the
    // result overwrites a local that previously held a fresh iterator.
    public fun box_project<K: copy + drop>(_self: &IterBox<K>): IteratorPtr<K> {
        abort 0
    }
    spec box_project {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
        ensures result == _self.it;
    }

    // Annotated projection: consumes the wrapper's validity and creates an
    // equally-valid iterator.
    public fun box_get<K: copy + drop>(_self: &IterBox<K>): IteratorPtr<K> {
        abort 0
    }
    spec box_get {
        pragma opaque;
        pragma verify = false;
        pragma iterator_use = it;
        pragma iterator_create;
        aborts_if false;
        ensures result == _self.it;
    }

    fun forward_stale_fails(): u64 {
        let t = new<u8, u64>();
        add(&mut t, 1, 2);
        let b = box_find(&t, 1);
        add(&mut t, 2, 3);
        let it = find(&t, 1);
        it = box_project(&b);
        *iter_borrow(it, &t)
    }

    fun project_fresh(): u64 {
        let t = new<u8, u64>();
        add(&mut t, 1, 2);
        let b = box_find(&t, 1);
        let it = box_get(&b);
        *iter_borrow(it, &t)
    }

    fun unpack_fresh(): u64 {
        let t = new<u8, u64>();
        add(&mut t, 1, 2);
        let b = box_find(&t, 1);
        let IterBox { it, tag: _ } = b;
        *iter_borrow(it, &t)
    }

    fun implant_stale_fails(): u64 {
        let t = new<u8, u64>();
        add(&mut t, 1, 2);
        let it_old = find(&t, 1);
        add(&mut t, 2, 3);
        let b = box_find(&t, 1);
        b.it = it_old;
        box_use(b, &t)
    }

    fun implant_fresh(): u64 {
        let t = new<u8, u64>();
        add(&mut t, 1, 2);
        add(&mut t, 2, 3);
        let it_new = find(&t, 1);
        let b = box_find(&t, 1);
        b.it = it_new;
        box_use(b, &t)
    }

    // A key-agnostic (unkeyed) iterator, mirroring a leaf/node walk: validity
    // is tracked against the per-map-type epoch.
    enum NodePtr has copy, drop {
        Node { node_index: u64 },
    }

    spec NodePtr {
        ghost stamp: num;
    }

    public fun node_begin<K: copy + drop, V>(_t: &Table<K, V>): NodePtr {
        abort 0
    }
    spec node_begin {
        pragma opaque;
        pragma verify = false;
        pragma iterator_create;
        aborts_if false;
    }

    public fun node_next<K: copy + drop, V>(self: NodePtr, _t: &Table<K, V>): NodePtr {
        abort 0
    }
    spec node_next {
        pragma opaque;
        pragma verify = false;
        pragma iterator_use;
        pragma iterator_create;
        aborts_if false;
    }
}

module 0x42::VerifyIteratorValidity {
    use 0x42::iter_table::{Self, Table};
    use 0x42::iter_table::spec_get;

    // Use without intervening mutation verifies.
    fun fresh_use(): u64 {
        let t = iter_table::new<u8, u64>();
        iter_table::add(&mut t, 1, 2);
        let it = iter_table::find(&t, 1);
        *iter_table::iter_borrow(it, &t)
    }
    spec fresh_use {
        ensures result == 2;
    }

    // Write-back through the intrinsic iter_borrow_mut role.
    fun fresh_borrow_mut(): Table<u8, u64> {
        let t = iter_table::new<u8, u64>();
        iter_table::add(&mut t, 1, 2);
        let it = iter_table::find(&t, 1);
        *iter_table::iter_borrow_mut(it, &mut t) = 5;
        t
    }
    spec fresh_borrow_mut {
        ensures spec_get(result, 1) == 5;
    }

    // Structural mutation between create and use: `iterator_use` assert fails.
    fun stale_use_fails(): u64 {
        let t = iter_table::new<u8, u64>();
        iter_table::add(&mut t, 1, 2);
        let it = iter_table::find(&t, 1);
        iter_table::add(&mut t, 2, 3);
        *iter_table::iter_borrow(it, &t)
    }

    // Same staleness through the intrinsic iter_borrow_mut role.
    fun stale_borrow_mut_fails(): Table<u8, u64> {
        let t = iter_table::new<u8, u64>();
        iter_table::add(&mut t, 1, 2);
        let it = iter_table::find(&t, 1);
        iter_table::remove(&mut t, 1);
        iter_table::add(&mut t, 1, 4);
        *iter_table::iter_borrow_mut(it, &mut t) = 5;
        t
    }

    // `iterator_invalidate` bumps the epoch: an iterator created before the
    // erase cannot be used after it.
    fun use_after_invalidate_fails(): u64 {
        let t = iter_table::new<u8, u64>();
        iter_table::add(&mut t, 1, 2);
        iter_table::add(&mut t, 2, 3);
        let it1 = iter_table::find(&t, 1);
        let it2 = iter_table::find(&t, 2);
        iter_table::erase(it1, &mut t);
        *iter_table::iter_borrow(it2, &t)
    }

    // Re-created iterators after an invalidation are valid again.
    fun refind_after_invalidate(): u64 {
        let t = iter_table::new<u8, u64>();
        iter_table::add(&mut t, 1, 2);
        iter_table::add(&mut t, 2, 3);
        let it1 = iter_table::find(&t, 1);
        iter_table::erase(it1, &mut t);
        let it2 = iter_table::find(&t, 2);
        *iter_table::iter_borrow(it2, &t)
    }
    spec refind_after_invalidate {
        ensures result == 3;
    }

    // Tracking applies in generic functions too (against the skolemized
    // instantiation's ghost state): stale use fails, fresh use verifies.
    fun generic_stale_fails<K: copy + drop>(t: &mut Table<K, u64>, k1: K, k2: K): u64 {
        iter_table::add(t, k1, 2);
        let it = iter_table::find(t, k1);
        iter_table::add(t, k2, 3);
        *iter_table::iter_borrow(it, t)
    }

    fun generic_fresh<K: copy + drop>(t: &mut Table<K, u64>, k1: K): u64 {
        iter_table::add(t, k1, 2);
        let it = iter_table::find(t, k1);
        *iter_table::iter_borrow(it, t)
    }

    // Unkeyed (node-walk) iterators chain through use+create without mutation.
    fun node_walk_fresh(): u64 {
        let t = iter_table::new<u8, u64>();
        iter_table::add(&mut t, 1, 2);
        let n = iter_table::node_begin(&t);
        let n2 = iter_table::node_next(n, &t);
        let _n3 = iter_table::node_next(n2, &t);
        7
    }

    // A structural mutation invalidates unkeyed iterators too.
    fun node_walk_stale_fails(): u64 {
        let t = iter_table::new<u8, u64>();
        iter_table::add(&mut t, 1, 2);
        let n = iter_table::node_begin(&t);
        iter_table::add(&mut t, 2, 3);
        let _n2 = iter_table::node_next(n, &t);
        7
    }

    // Creating another iterator with the same key does not re-validate a stale
    // one: validity follows the lineage of each iterator-holding local, not
    // the iterator's value (here both iterators are equal as values, since
    // this iterator type is fully determined by the key).
    fun revalidation_by_alias_fails(): u64 {
        let t = iter_table::new<u8, u64>();
        iter_table::add(&mut t, 1, 2);
        let it_old = iter_table::find(&t, 1);
        iter_table::add(&mut t, 2, 3);
        let _it_new = iter_table::find(&t, 1);
        *iter_table::iter_borrow(it_old, &t)
    }

    // An iterator held across loop iterations stays valid when the loop does
    // not mutate the map (validity is maintained as an implicit loop invariant).
    fun loop_no_mutation(n: u8): u64 {
        let t = iter_table::new<u8, u64>();
        iter_table::add(&mut t, 1, 2);
        let it = iter_table::find(&t, 1);
        let sum = 0;
        let i = 0;
        while (i < n) {
            sum += *iter_table::iter_borrow(it, &t);
            i += 1;
        };
        sum
    }

    // A loop body that structurally mutates the map invalidates an iterator
    // held across iterations: the implicit invariant's induction case fails.
    fun loop_mutation_fails(n: u8): u64 {
        let t = iter_table::new<u8, u64>();
        iter_table::add(&mut t, 1, 2);
        let it = iter_table::find(&t, 1);
        let sum = 0;
        let i = 0;
        while (i < n) {
            sum += *iter_table::iter_borrow(it, &t);
            iter_table::add(&mut t, 100, 1);
            i += 1;
        };
        sum
    }
}
