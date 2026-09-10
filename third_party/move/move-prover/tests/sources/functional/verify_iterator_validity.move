// Tests per-object iterator-validity for intrinsic maps. Binding a validity
// predicate role gives the map — and the predicate's iterator enum — a
// hidden validity slot: fresh at creation, havoced by every structural
// mutation of the map, preserved by value writes, excluded from equality,
// and not nameable from any spec. Validity itself is the bound native
// predicate (slot equality), stated as ordinary `requires`/`ensures` on the
// iterator API. Using an iterator after a structural mutation of its map —
// or against a different map object — must fail verification; uses without
// intervening mutation must verify.
module 0x42::iter_table {
    struct Table<phantom K: copy + drop, phantom V> has store, drop, copy {}

    enum IteratorPtr<K: copy + drop> has copy, drop {
        End,
        Some { key: K },
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
            map_spec_has_key = spec_contains,
            map_spec_iter_valid = spec_iter_valid,
            map_spec_leaf_iter_valid = spec_node_valid,
            map_spec_iter_preserved = spec_iter_preserved;
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
    // The validity predicates and the preservation frame predicate: their
    // definitions (hidden-slot equality) come from the role templates.
    spec native fun spec_iter_valid<K, V>(it: IteratorPtr<K>, t: Table<K, V>): bool;
    spec native fun spec_node_valid<K, V>(it: NodePtr, t: Table<K, V>): bool;
    spec native fun spec_iter_preserved<K, V>(t_new: Table<K, V>, t_old: Table<K, V>): bool;

    spec fun spec_aborts_iter_borrow_mut<K, V>(self: IteratorPtr<K>, t: Table<K, V>): bool {
        (self is IteratorPtr::End<K>) || !spec_contains(t, self.key)
    }

    spec iter_borrow_mut {
        requires spec_iter_valid(self, t);
    }

    public fun find<K: copy + drop, V>(_t: &Table<K, V>, _key: K): IteratorPtr<K> {
        abort 0
    }
    spec find {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
        ensures spec_iter_valid(result, _t);
        ensures (result is IteratorPtr::Some<K>) <==> spec_contains(_t, _key);
        ensures (result is IteratorPtr::Some<K>) ==> result.key == _key;
    }

    public fun iter_borrow<K: copy + drop, V>(self: IteratorPtr<K>, _t: &Table<K, V>): &V {
        abort 0
    }
    spec iter_borrow {
        pragma opaque;
        pragma verify = false;
        requires spec_iter_valid(self, _t);
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
        requires spec_iter_valid(self, _t);
        aborts_if (self is IteratorPtr::End<K>) || !spec_contains(_t, self.key);
        // The whole-map equality excludes the hidden slot: the opaque `&mut`
        // map's slot stays havoced, which IS the invalidation — no
        // annotation needed.
        ensures _t == spec_remove(old(_t), self.key);
    }

    // A wrapper carrying an iterator in a field (like `IteratorPtrWithPath`).
    // The hidden slot travels inside the iterator value through field writes
    // and write-backs, so implanting a stale iterator into a fresh wrapper
    // is caught even though the wrapper itself was freshly created.
    // Field access requires this module, so the scenario tests live here.
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
        aborts_if false;
        ensures spec_iter_valid(result.it, _t);
    }

    public fun box_use<K: copy + drop, V>(_self: IterBox<K>, _t: &Table<K, V>): u64 {
        abort 0
    }
    spec box_use {
        pragma opaque;
        pragma verify = false;
        requires spec_iter_valid(_self.it, _t);
        aborts_if false;
    }

    // OPAQUE forwarding: the result's hidden slot is unconstrained
    // (fail-closed) — even when the result overwrites a local that
    // previously held a fresh iterator, and although the ensures equates
    // the values (equality excludes the slot). An opaque function cannot
    // state slot lineage; forwarders must be transparent (or take the map
    // and re-establish validity).
    public fun box_project<K: copy + drop>(_self: &IterBox<K>): IteratorPtr<K> {
        abort 0
    }
    spec box_project {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
        ensures result == _self.it;
    }

    // TRANSPARENT projection: value flow carries the hidden slot, so the
    // result inherits the wrapper's iterator validity with no contract.
    public fun box_get<K: copy + drop>(_self: &IterBox<K>): IteratorPtr<K> {
        _self.it
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

    // A key-agnostic (unkeyed) iterator, mirroring a leaf/node walk: bound
    // through `map_spec_leaf_iter_valid`, it gets its own hidden slot and
    // the same per-object validity model.
    enum NodePtr has copy, drop {
        Node { node_index: u64 },
    }

    public fun node_begin<K: copy + drop, V>(_t: &Table<K, V>): NodePtr {
        abort 0
    }
    spec node_begin {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
        ensures spec_node_valid(result, _t);
    }

    public fun node_next<K: copy + drop, V>(self: NodePtr, _t: &Table<K, V>): NodePtr {
        abort 0
    }
    spec node_next {
        pragma opaque;
        pragma verify = false;
        requires spec_node_valid(self, _t);
        aborts_if false;
        ensures spec_node_valid(result, _t);
    }

    // A value-only write through the intrinsic borrow preserves validity;
    // `spec_iter_preserved` states the same frame promise for opaque
    // helpers whose implementations do not structurally mutate.
    public fun value_touch<K: copy + drop, V>(_t: &mut Table<K, V>) {
        abort 0
    }
    spec value_touch {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
        ensures spec_iter_preserved(_t, old(_t));
        ensures _t == old(_t);
    }
}

module 0x42::VerifyIteratorValidity {
    use std::vector;
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

    // Structural mutation between create and use: the validity requires fails.
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

    // An invalidating operation havocs the brand: an iterator created before
    // the erase cannot be used after it.
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

    // Tracking applies in generic functions too: stale use fails, fresh use
    // verifies.
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
    // one: the hidden slot belongs to each iterator value and is excluded from
    // value equality (here both iterators are equal as runtime values, since
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
    // not mutate the map: neither the iterator nor the map is a loop target,
    // so no havoc touches the stamp or the brand — no loop invariant needed.
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

    // An iterator ADVANCED in the loop is a loop target and gets havocked;
    // a user-written validity invariant re-establishes it (expressible since
    // validity is a plain spec function).
    fun node_loop_with_invariant(rounds: u8): u64 {
        let t = iter_table::new<u8, u64>();
        iter_table::add(&mut t, 1, 2);
        let np = iter_table::node_begin(&t);
        let i = 0;
        while ({
            spec {
                invariant iter_table::spec_node_valid(np, t);
            };
            i < rounds
        }) {
            np = iter_table::node_next(np, &t);
            i += 1;
        };
        7
    }

    // ---- Per-object identity: the cells the brand model exists for ----

    // An iterator from map A used against same-type map B fails, even with no
    // intervening mutation: B's brand is an independent unconstrained value.
    fun cross_map_fails(): u64 {
        let a = iter_table::new<u8, u64>();
        let b = iter_table::new<u8, u64>();
        iter_table::add(&mut a, 1, 2);
        iter_table::add(&mut b, 1, 9);
        let it = iter_table::find(&a, 1);
        *iter_table::iter_borrow(it, &b)
    }

    // Mutating a same-type sibling does NOT invalidate this map's iterators
    // (per-object state; a type-level scheme would reject this).
    fun sibling_independent(): u64 {
        let a = iter_table::new<u8, u64>();
        let b = iter_table::new<u8, u64>();
        iter_table::add(&mut a, 1, 2);
        let it = iter_table::find(&a, 1);
        iter_table::add(&mut b, 7, 7);
        *iter_table::iter_borrow(it, &a)
    }
    spec sibling_independent {
        ensures result == 2;
    }

    // Vector of same-type maps: an iterator from element 0 fails against
    // element 1 (no type-level scheme can distinguish the elements) ...
    fun vector_cross_element_fails(): u64 {
        let v = vector[iter_table::new<u8, u64>(), iter_table::new<u8, u64>()];
        iter_table::add(vector::borrow_mut(&mut v, 0), 1, 2);
        iter_table::add(vector::borrow_mut(&mut v, 1), 1, 9);
        let it = iter_table::find(vector::borrow(&v, 0), 1);
        let r = *iter_table::iter_borrow(it, vector::borrow(&v, 1));
        let (a, b) = (vector::pop_back(&mut v), vector::pop_back(&mut v));
        let (_, _) = (a, b);
        r
    }

    // ... and verifies against its own element.
    fun vector_same_element(): u64 {
        let v = vector[iter_table::new<u8, u64>(), iter_table::new<u8, u64>()];
        iter_table::add(vector::borrow_mut(&mut v, 0), 1, 2);
        let it = iter_table::find(vector::borrow(&v, 0), 1);
        let r = *iter_table::iter_borrow(it, vector::borrow(&v, 0));
        let (a, b) = (vector::pop_back(&mut v), vector::pop_back(&mut v));
        let (_, _) = (a, b);
        r
    }
    spec vector_same_element {
        ensures result == 2;
    }

    // A COPY shares the brand — correct: an iterator navigates a bit-identical
    // copy exactly as the original ...
    fun copy_shares_brand(): u64 {
        let t = iter_table::new<u8, u64>();
        iter_table::add(&mut t, 1, 2);
        let it = iter_table::find(&t, 1);
        let t2 = t;
        *iter_table::iter_borrow(it, &t2)
    }
    spec copy_shares_brand {
        ensures result == 2;
    }

    // ... until the copy diverges: mutating it havocs ITS brand only; the
    // iterator dies on the copy but stays valid on the original.
    fun copy_diverges_fails(): u64 {
        let t = iter_table::new<u8, u64>();
        iter_table::add(&mut t, 1, 2);
        let it = iter_table::find(&t, 1);
        let t2 = t;
        iter_table::add(&mut t2, 2, 3);
        *iter_table::iter_borrow(it, &t2)
    }

    fun copy_diverges_original_ok(): u64 {
        let t = iter_table::new<u8, u64>();
        iter_table::add(&mut t, 1, 2);
        let it = iter_table::find(&t, 1);
        let t2 = t;
        iter_table::add(&mut t2, 2, 3);
        *iter_table::iter_borrow(it, &t)
    }
    spec copy_diverges_original_ok {
        ensures result == 2;
    }

    // An opaque helper promising `spec_iter_preserved` (a value-only write)
    // keeps iterators alive across the call.
    fun preserved_wrapper_ok(): u64 {
        let t = iter_table::new<u8, u64>();
        iter_table::add(&mut t, 1, 2);
        let it = iter_table::find(&t, 1);
        iter_table::value_touch(&mut t);
        *iter_table::iter_borrow(it, &t)
    }
    spec preserved_wrapper_ok {
        ensures result == 2;
    }

    // An opaque wrapper taking `&mut` map with NO frame promise:
    // fail-closed, iterators die.
    fun opaque_mut_wrapper_fails(): u64 {
        let t = iter_table::new<u8, u64>();
        iter_table::add(&mut t, 1, 2);
        let it = iter_table::find(&t, 1);
        opaque_touch(&mut t);
        *iter_table::iter_borrow(it, &t)
    }

    fun opaque_touch(_t: &mut Table<u8, u64>) {
    }
    spec opaque_touch {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
    }

    // A loop body that structurally mutates the map invalidates an iterator
    // held across iterations: the map is a loop target, its havoc loses the
    // brand, and the validity requires in the next iteration fails.
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
