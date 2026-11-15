spec aptos_framework::big_ordered_map {

    // The ordering bindings below (`map_borrow_front`/`back`, `map_pop_front`/`back`,
    // `map_prev_key`/`next_key`) presume `cmp::compare<K>` is a strict total order on K.
    // Built-in K types satisfy this; user-defined K types must too for this spec block
    // to be sound.
    //
    // Size presumption: BigOrderedMap validates K/V serialized sizes against node-size
    // limits (`validate_static_size_and_init_max_degrees` and per-insert checks) and
    // aborts when exceeded. These size-based aborts — including `borrow_mut`'s
    // constant-value-size requirement — are presumed not to fire and are not
    // modeled by the bindings below.
    //
    // Structural presumption: the tree's internal invariants — node shapes,
    // child kinds (leaf nodes hold only `Child::Leaf`), and index validity
    // (the `EINTERNAL_INVARIANT_BROKEN` asserts and variant field accesses) —
    // are maintained by construction by this module, presumed to hold, and
    // not modeled. Traversal specs' abort conditions are exhaustive modulo
    // this presumption.
    //
    // Iterator staleness: the iterator overlay specs apply to an iterator only
    // for the map state it was created from (the documented API contract: the
    // map must not be mutated while iterators are held). All iterator specs
    // model reads and writes at the iterator's cached key; at runtime a stale
    // iterator navigates by its retained position, so it may abort, return
    // arbitrary results, or read/mutate a DIFFERENT entry than modeled. The
    // prover enforces this contract mechanically and per map OBJECT: the
    // validity bindings below give the map and its iterator types hidden
    // version slots (fresh at creation, havocked by every structural
    // mutation, preserved by value writes, excluded from equality, not
    // nameable in specs), and validity — the bound native predicates, defined
    // by the prover as slot equality — is stated as ordinary
    // `requires`/`ensures` on the iterator API below. Any use of a stale
    // iterator, or of an iterator against a different map object (also a
    // sibling of the same type, or another element of a vector of maps),
    // fails verification. Loops that advance an iterator carry
    // `invariant spec_iter_valid(it, map)`; loops that merely hold one while
    // leaving the map unmutated need no invariant.
    spec BigOrderedMap {
        pragma intrinsic = map,
            map_new = new,
            map_new_with_config = new_with_config,
            map_len = compute_length,
            map_destroy_empty = destroy_empty,
            map_has_key = contains,
            map_add_no_override = add,
            map_upsert = upsert,
            map_del_must_exist = remove,
            map_remove_or_none = remove_or_none,
            map_get = get,
            map_borrow_front = borrow_front,
            map_borrow_back = borrow_back,
            map_front_key = front_key,
            map_back_key = back_key,
            map_pop_front = pop_front,
            map_pop_back = pop_back,
            map_prev_key = prev_key,
            map_next_key = next_key,
            map_keys = keys,
            map_to_ordered_map = to_ordered_map,
            map_new_from = new_from,
            map_add_all = add_all,
            map_borrow = borrow,
            map_borrow_mut = borrow_mut,
            map_iter_borrow_mut = iter_borrow_mut,
            map_spec_aborts_iter_borrow_mut = spec_aborts_iter_borrow_mut,
            map_spec_get = spec_get,
            map_spec_set = spec_set,
            map_spec_del = spec_remove,
            map_spec_len = spec_len,
            map_spec_has_key = spec_contains_key,
            map_spec_key_at = spec_key_at,
            map_spec_rank = spec_rank,
            map_spec_aborts_empty = spec_aborts_empty,
            map_spec_aborts_add_all = spec_aborts_add_all,
            map_spec_aborts_new_from = spec_aborts_new_from,
            map_spec_aborts_new_with_config = spec_aborts_new_with_config,
            map_spec_aborts_destroy_empty = spec_aborts_destroy_empty,
            map_spec_aborts_add = spec_aborts_add,
            map_spec_aborts_del = spec_aborts_del,
            map_spec_aborts_borrow = spec_aborts_borrow,
            map_is_empty = is_empty,
            map_spec_iter_valid = spec_iter_current,
            map_spec_leaf_iter_valid = spec_leaf_iter_valid,
            map_spec_leaf_offset = spec_leaf_offset,
            map_spec_iter_preserved = spec_iter_preserved;
    }

    // The iterator-validity predicates; their definitions (hidden-slot
    // equality: the iterator was created from this map object and no
    // structural mutation has intervened) come from the role bindings above.
    spec native fun spec_iter_current<K, V>(it: IteratorPtr<K>, map: BigOrderedMap<K, V>): bool;
    spec native fun spec_leaf_iter_valid<K, V>(it: LeafNodeIteratorPtr, map: BigOrderedMap<K, V>): bool;
    // Frame predicate: no structural mutation between the two states, so
    // iterators valid for the old state stay valid for the new one.
    spec native fun spec_iter_preserved<K, V>(m_new: BigOrderedMap<K, V>, m_old: BigOrderedMap<K, V>): bool;

    /// An iterator is valid for `map` iff it was created from this map object
    /// and the object has not been structurally mutated since. End iterators
    /// carry no position and are always valid.
    spec fun spec_iter_valid<K, V>(it: IteratorPtr<K>, map: BigOrderedMap<K, V>): bool {
        (it is IteratorPtr::End<K>) || spec_iter_current(it, map)
    }

    spec native fun spec_len<K, V>(t: BigOrderedMap<K, V>): num;
    spec native fun spec_contains_key<K, V>(t: BigOrderedMap<K, V>, k: K): bool;
    // Enumeration view: `spec_key_at(t, i)` is the i-th smallest key under
    // `cmp::compare` (0 <= i < spec_len(t)), `spec_rank(t, k)` its inverse on
    // contained keys. Lets loop invariants index a traversal by position.
    spec native fun spec_key_at<K, V>(t: BigOrderedMap<K, V>, i: num): K;
    spec native fun spec_rank<K, V>(t: BigOrderedMap<K, V>, k: K): num;
    spec native fun spec_set<K, V>(t: BigOrderedMap<K, V>, k: K, v: V): BigOrderedMap<K, V>;
    spec native fun spec_remove<K, V>(t: BigOrderedMap<K, V>, k: K): BigOrderedMap<K, V>;
    spec native fun spec_get<K, V>(t: BigOrderedMap<K, V>, k: K): V;
    spec native fun spec_aborts_destroy_empty<K, V>(t: BigOrderedMap<K, V>): bool;
    spec native fun spec_aborts_add<K, V>(t: BigOrderedMap<K, V>, k: K, v: V): bool;
    spec native fun spec_aborts_del<K, V>(t: BigOrderedMap<K, V>, k: K): bool;
    spec native fun spec_aborts_borrow<K, V>(t: BigOrderedMap<K, V>, k: K): bool;

    spec fun spec_aborts_empty<K, V>(t: BigOrderedMap<K, V>): bool {
        spec_len(t) == 0
    }

    spec fun spec_aborts_add_all<K, V>(m: BigOrderedMap<K, V>, keys: vector<K>, values: vector<V>): bool {
        len(keys) != len(values)
            || (exists i in 0..len(keys): spec_contains_key(m, keys[i]))
            || (exists i in 0..len(keys), j in 0..len(keys) where i != j: keys[i] == keys[j])
    }

    spec fun spec_aborts_new_from<K, V>(keys: vector<K>, values: vector<V>): bool {
        len(keys) != len(values)
            || (exists i in 0..len(keys), j in 0..len(keys) where i != j: keys[i] == keys[j])
    }

    spec fun spec_aborts_new_with_config<K, V>(
        inner_max_degree: u16, leaf_max_degree: u16, _reuse_slots: bool
    ): bool {
        (inner_max_degree != 0
            && (inner_max_degree < 4 || (inner_max_degree as u64) > 4096))
        || (leaf_max_degree != 0
            && (leaf_max_degree < 3 || (leaf_max_degree as u64) > 4096))
    }


    spec new_with_config {
        pragma intrinsic;
    }

    spec new {
        pragma intrinsic;
    }

    spec new_with_reusable {
        pragma verify = false;
        pragma opaque;
        aborts_if false;
        ensures spec_len(result) == 0;
        ensures forall k: K: !spec_contains_key(result, k);
    }

    spec new_with_type_size_hints {
        pragma verify = false;
        pragma opaque;
        // Exhaustive over the hint-validation aborts (parameter ordering,
        // division by zero, u64 overflow of the entry-size sums, and the
        // hint-derived degree thresholds). The internal storage-allocator
        // alignment asserts fall under the structural presumption above, and
        // the degrees passed on to `new_with_config` are within its bounds by
        // construction (clamped between the *_MIN_DEGREE thresholds asserted
        // here and `MAX_DEGREE`).
        aborts_if avg_key_bytes > max_key_bytes;
        aborts_if avg_value_bytes > max_value_bytes;
        aborts_if avg_key_bytes == 0;
        aborts_if max_key_bytes > 0
            && HINT_MAX_NODE_BYTES / max_key_bytes < INNER_MIN_DEGREE;
        aborts_if avg_key_bytes + avg_value_bytes > MAX_U64;
        aborts_if max_key_bytes + max_value_bytes > MAX_U64;
        aborts_if max_key_bytes + max_value_bytes > 0
            && HINT_MAX_NODE_BYTES / (max_key_bytes + max_value_bytes) < LEAF_MIN_DEGREE;
        ensures spec_len(result) == 0;
        ensures forall k: K: !spec_contains_key(result, k);
    }

    spec borrow {
        pragma intrinsic;
    }

    spec borrow_mut {
        pragma intrinsic;
    }

    spec contains {
        pragma intrinsic;
    }

    spec destroy_empty {
        pragma intrinsic;
    }

    spec add {
        pragma intrinsic;
    }

    spec remove {
        pragma intrinsic;
    }

    spec get {
        pragma intrinsic;
    }

    spec fun spec_unchanged_except_at<K: drop + copy + store, V: store>(
        self: &mut BigOrderedMap<K, V>, key: &K
    ): bool {
        (forall k: K where k != key:
            spec_contains_key(self, k) == spec_contains_key(old(self), k))
        && (forall k: K where k != key && spec_contains_key(old(self), k):
            spec_get(self, k) == spec_get(old(self), k))
    }

    spec remove_or_none {
        pragma intrinsic;
    }

    spec is_empty {
        pragma intrinsic;
    }

    spec iter_is_end {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
        ensures result == (self is IteratorPtr::End<K>);
    }

    spec iter_borrow {
        pragma opaque;
        pragma verify = false;
        requires spec_iter_valid(self, map);
        aborts_if iter_is_end(self, map);
        ensures result == spec_get(map, self.key);
    }

    // Intrinsic (`map_iter_borrow_mut`): the returned `&mut V` carries a table
    // index edge, so caller write-back updates the abstract map at `self.key`
    // instead of traversing intrinsic internals. The body's constant-value-size
    // assert is covered by the size presumption above.
    spec iter_borrow_mut {
        pragma intrinsic;
        requires spec_iter_valid(self, map);
    }

    spec fun spec_aborts_iter_borrow_mut<K, V>(self: IteratorPtr<K>, map: BigOrderedMap<K, V>): bool {
        (self is IteratorPtr::End<K>) || !spec_contains_key(map, self.key)
    }

    // Spec-level mirror of `iter_is_begin`. The Move body reads intrinsic map
    // internals, so the function itself cannot appear in spec expressions.
    // self is End: begin iff map is empty (End acts as both begin and end on []).
    // self is Some: begin iff self.key is the smallest key currently in map.
    spec fun spec_iter_is_begin<K, V>(self: IteratorPtr<K>, map: BigOrderedMap<K, V>): bool {
        if (self is IteratorPtr::End<K>) {
            spec_len(map) == 0
        } else {
            spec_contains_key(map, self.key)
                && (forall k: K where spec_contains_key(map, k) && k != self.key:
                    std::cmp::compare(self.key, k) == std::cmp::Ordering::Less)
        }
    }

    spec iter_is_begin {
        pragma opaque;
        pragma verify = false;
        requires spec_iter_valid(self, map);
        aborts_if false;
        ensures result <==> spec_iter_is_begin(self, map);
        // The smallest key occupies position 0, so a non-End begin iterator
        // sits at rank 0. Stated here because the characterization above is by
        // `cmp::compare` minimality, which does not by itself reach the
        // enumeration; a backward traversal needs this to conclude at begin
        // that it has walked the whole map.
        ensures result && !(self is IteratorPtr::End<K>) ==> spec_rank(map, self.key) == 0;
    }

    // Returns the iterator pointing to the smallest key K in self with K >= input
    // key (compare not Less), or End if no such key exists.
    spec internal_lower_bound {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
        ensures spec_iter_current(result, self);
        // End iff no key >= input exists (all keys are Less than input).
        ensures iter_is_end(result, self) <==>
            (forall k: K where spec_contains_key(self, k):
                std::cmp::compare(k, key) == std::cmp::Ordering::Less);
        // Otherwise, result.key is in the map, >= input, and the smallest such.
        ensures !iter_is_end(result, self) ==> spec_contains_key(self, result.key);
        ensures !iter_is_end(result, self) ==>
            std::cmp::compare(result.key, key) != std::cmp::Ordering::Less;
        ensures !iter_is_end(result, self) ==>
            (forall k: K where spec_contains_key(self, k) && std::cmp::compare(k, key) != std::cmp::Ordering::Less:
                std::cmp::compare(result.key, k) != std::cmp::Ordering::Greater);
    }

    spec iter_borrow_key {
        pragma opaque;
        pragma verify = false;
        aborts_if self is IteratorPtr::End<K>;
        ensures result == self.key;
    }

    spec allocate_spare_slots {
        pragma verify = false;
        pragma opaque;
        // Allocates vacant storage slots only: map content and iterator
        // navigation are untouched, so iterators stay valid.
        ensures self == old(self);
        ensures spec_iter_preserved(self, old(self));
    }

    spec validate_size_and_init_max_degrees {
        pragma verify = false;
        pragma opaque;
    }

    spec validate_dynamic_size_and_init_max_degrees {
        pragma verify = false;
        pragma opaque;
    }

    spec validate_static_size_and_init_max_degrees {
        pragma verify = false;
        pragma opaque;
    }

    spec keys {
        pragma intrinsic;
    }

    spec to_ordered_map {
        pragma intrinsic;
    }

    spec new_from {
        pragma intrinsic;
    }

    spec upsert {
        pragma intrinsic;
        // An existing-key upsert replaces the value in place (`add_at`
        // overwrites before ever splitting): not a structural mutation. The
        // intrinsic model preserves iterator validity on that branch, so no
        // annotation is needed here (see `test_verify_iter_across_upsert`).
    }

    spec add_all {
        pragma intrinsic;
    }

    spec borrow_front {
        pragma intrinsic;
    }

    spec front_key {
        pragma intrinsic;
    }

    spec borrow_back {
        pragma intrinsic;
    }

    spec back_key {
        pragma intrinsic;
    }

    spec pop_front {
        pragma intrinsic;
    }

    spec pop_back {
        pragma intrinsic;
    }

    spec prev_key {
        pragma intrinsic;
    }

    spec next_key {
        pragma intrinsic;
    }


    spec internal_find {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
        ensures spec_iter_current(result, self);
        ensures iter_is_end(result, self) <==> !spec_contains_key(self, key);
        ensures !iter_is_end(result, self) ==> result.key == key;
    }

    spec internal_new_begin_iter {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
        ensures spec_iter_current(result, self);
        ensures iter_is_end(result, self) <==> spec_len(self) == 0;
        ensures !iter_is_end(result, self) ==> spec_contains_key(self, result.key);
        // result.key is the smallest key in the map.
        ensures !iter_is_end(result, self) ==>
            (forall k: K where spec_contains_key(self, k) && k != result.key:
                std::cmp::compare(result.key, k) == std::cmp::Ordering::Less);
        // The first key has rank 0.
        ensures !iter_is_end(result, self) ==> spec_rank(self, result.key) == 0;
    }

    spec internal_new_end_iter {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
        ensures result is IteratorPtr::End<K>;
    }

    spec iter_next {
        pragma opaque;
        pragma verify = false;
        requires spec_iter_valid(self, map);
        aborts_if iter_is_end(self, map);
        ensures spec_iter_current(result, map);
        // End iff self.key has no strict successor in the map.
        ensures (result is IteratorPtr::End<K>) <==>
            (forall k: K where spec_contains_key(map, k):
                std::cmp::compare(k, self.key) != std::cmp::Ordering::Greater);
        // Otherwise result.key is the smallest in-map key strictly greater than self.key.
        ensures !(result is IteratorPtr::End<K>) ==> spec_contains_key(map, result.key);
        ensures !(result is IteratorPtr::End<K>) ==>
            std::cmp::compare(result.key, self.key) == std::cmp::Ordering::Greater;
        ensures !(result is IteratorPtr::End<K>) ==>
            (forall k: K where spec_contains_key(map, k)
                && std::cmp::compare(k, self.key) == std::cmp::Ordering::Greater:
                std::cmp::compare(result.key, k) != std::cmp::Ordering::Greater);
        // Rank increments per step; End means self.key had the last rank.
        ensures !(result is IteratorPtr::End<K>) && spec_contains_key(map, self.key) ==>
            spec_rank(map, result.key) == spec_rank(map, self.key) + 1;
        ensures (result is IteratorPtr::End<K>) && spec_contains_key(map, self.key) ==>
            spec_rank(map, self.key) == spec_len(map) - 1;
    }

    spec iter_prev {
        pragma opaque;
        pragma verify = false;
        requires spec_iter_valid(self, map);
        aborts_if spec_iter_is_begin(self, map);
        ensures spec_iter_current(result, map);
        // A predecessor always exists when self is not begin; from End the result
        // is the largest key. The result always points at an in-map key.
        ensures !(result is IteratorPtr::End<K>);
        ensures spec_contains_key(map, result.key);
        // From End: result.key is the largest key in the map.
        ensures (self is IteratorPtr::End<K>) ==>
            (forall k: K where spec_contains_key(map, k) && k != result.key:
                std::cmp::compare(k, result.key) == std::cmp::Ordering::Less);
        // Otherwise result.key is the largest in-map key strictly less than self.key.
        ensures !(self is IteratorPtr::End<K>) ==>
            std::cmp::compare(result.key, self.key) == std::cmp::Ordering::Less;
        ensures !(self is IteratorPtr::End<K>) ==>
            (forall k: K where spec_contains_key(map, k)
                && std::cmp::compare(k, self.key) == std::cmp::Ordering::Less:
                std::cmp::compare(k, result.key) != std::cmp::Ordering::Greater);
        // From End: result has the last rank; otherwise the rank decrements.
        ensures (self is IteratorPtr::End<K>) ==>
            spec_rank(map, result.key) == spec_len(map) - 1;
        ensures !(self is IteratorPtr::End<K>) && spec_contains_key(map, self.key) ==>
            spec_rank(map, result.key) == spec_rank(map, self.key) - 1;
    }

    spec compute_length {
        pragma intrinsic;
    }

    spec iter_modify {
        pragma opaque;
        pragma verify = false;
        // The closure's contract (`aborts_of`/`ensures_of` below) is only
        // established for inputs satisfying its precondition — the closure is
        // verified under `requires_of` — so importing it is sound only when
        // the caller establishes that precondition on the current value. The
        // end-iterator disjunct exempts calls that abort at the end-check
        // before the closure is ever invoked (`self.key` does not exist
        // there). Trivially true for closures without a `requires`. The
        // body's post-callback size validation is covered by the size
        // presumption above.
        requires iter_is_end(self, map) || requires_of<f>(spec_get(map, self.key));
        requires spec_iter_valid(self, map);
        aborts_if iter_is_end(self, map);
        aborts_if aborts_of<f>(spec_get(map, self.key));
        // A value modification is not a structural mutation: iterators stay valid.
        ensures spec_iter_preserved(map, old(map));
        // iter_modify mutates the value at self.key via the closure. Containment is
        // unchanged for every key; values for keys other than self.key are preserved;
        // the closure's contract relates the old value, the new value, and the result.
        ensures spec_contains_key(map, self.key);
        ensures spec_len(map) == spec_len(old(map));
        ensures spec_unchanged_except_at(map, self.key);
        ensures ensures_of<f>(old(spec_get(map, self.key)), result, spec_get(map, self.key));
        // A value write moves no keys, so every position survives. This has to
        // be stated positionally: equality against `spec_set` would not do it,
        // because equality on a map carrying ghosts is extensional (see
        // `$IsEqual` in the prelude), so it never produces the write term the
        // model's rank-preservation axiom triggers on. Same content as that
        // axiom, so no new trust.
        ensures forall i in 0..spec_len(map): spec_key_at(map, i) == spec_key_at(old(map), i);
        ensures forall k: K where spec_contains_key(old(map), k):
            spec_rank(map, k) == spec_rank(old(map), k);
    }

    spec internal_find_with_path {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
        ensures spec_iter_current(result.iterator, self);
        ensures iter_is_end(result.iterator, self) <==> !spec_contains_key(self, key);
        ensures !iter_is_end(result.iterator, self) ==> result.iterator.key == key;
    }

    // TRANSPARENT (the body is a plain projection): equality in an opaque
    // ensures would not carry the projected iterator's hidden validity slot;
    // inlined value flow does.
    spec iter_with_path_get_iter {
        aborts_if false;
        ensures result == self.iterator;
    }

    spec iter_remove {
        pragma opaque;
        pragma verify = false;
        requires spec_iter_valid(self.iterator, map);
        aborts_if iter_is_end(self.iterator, map);
        ensures result == spec_get(old(map), self.iterator.key);
        ensures !spec_contains_key(map, self.iterator.key);
        ensures spec_len(map) == spec_len(old(map)) - 1;
        ensures spec_unchanged_except_at(map, self.iterator.key);
        // Removal closes the position up: keys before the removed one keep
        // their place, keys after it move down by one. Positional for the same
        // reason as in `iter_modify` — extensional equality against
        // `spec_remove` cannot reach the enumeration.
        ensures forall i in 0..spec_rank(old(map), self.iterator.key):
            spec_key_at(map, i) == spec_key_at(old(map), i);
        ensures forall i in spec_rank(old(map), self.iterator.key)..spec_len(map):
            spec_key_at(map, i) == spec_key_at(old(map), i + 1);
    }

    // Position of a leaf in the walk: the number of keys held by the leaves
    // before it. Uninterpreted — its meaning comes entirely from the clauses on
    // the two leaf functions below, which say it starts at zero, advances by
    // each leaf's size, and reaches the map's length when the walk ends. That
    // is what lets a leaf walk carry a rank-indexed invariant, and what makes
    // the walk COMPLETE rather than merely sound: the entries seen are the
    // map's keys at positions `offset .. offset + leaf size`, and the offsets
    // tile `0 .. spec_len(map)`.
    spec native fun spec_leaf_offset<K, V>(
        leaf: LeafNodeIteratorPtr, map: BigOrderedMap<K, V>
    ): num;

    spec internal_leaf_new_begin_iter {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
        ensures spec_leaf_iter_valid(result, self);
        // Points at `min_leaf_index`, which is never NULL_INDEX: an empty map's
        // leaf walk visits the (empty) root leaf once.
        ensures !internal_leaf_iter_is_end(result);
        // Nothing precedes the first leaf.
        ensures spec_leaf_offset(result, self) == 0;
    }

    spec internal_leaf_iter_is_end {
        pragma opaque;
        aborts_if false;
        ensures result == (self.node_index == NULL_INDEX);
    }

    spec internal_leaf_borrow_value {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
        ensures result == self.value;
    }

    spec internal_leaf_iter_borrow_entries_and_next_leaf_index {
        pragma opaque;
        pragma verify = false;
        requires spec_leaf_iter_valid(self, map);
        aborts_if internal_leaf_iter_is_end(self);
        ensures spec_leaf_iter_valid(result_2, map);
        // Every entry in the returned leaf is a real map entry (a Leaf child
        // with contained key and matching value).
        ensures forall k: K where ordered_map::spec_contains_key(result_1, k):
            spec_contains_key(map, k);
        ensures forall k: K where ordered_map::spec_contains_key(result_1, k):
            (ordered_map::spec_get(result_1, k) is Child::Leaf<V>)
                && ordered_map::spec_get(result_1, k).value == spec_get(map, k);
        // Leaves of a nonempty map are nonempty.
        ensures spec_len(map) > 0 ==> ordered_map::spec_len(result_1) > 0;
        // The other direction, which the soundness clauses above do not give:
        // this leaf holds exactly the map's keys at positions
        // `offset .. offset + leaf size`, in that order. Leaves are visited in
        // ascending key order and each leaf's entries are ascending, so the
        // leaf's own enumeration lines up with the map's at that offset.
        ensures spec_leaf_offset(self, map) + ordered_map::spec_len(result_1)
            <= spec_len(map);
        ensures forall j in 0..ordered_map::spec_len(result_1):
            spec_key_at(map, spec_leaf_offset(self, map) + j)
                == ordered_map::spec_key_at(result_1, j);
        // The same correspondence for values, stated positionally. The
        // key-guarded clauses above cannot be used by a walk that has just read
        // position `j`: that would first require the key at `j` to be known
        // contained, which nothing about an opaque result gives. Leaf-ness is
        // part of it — `.value` is a variant selector, so without it the value
        // equality says nothing about the child a walk actually reads.
        ensures forall j in 0..ordered_map::spec_len(result_1):
            ordered_map::spec_get(result_1, ordered_map::spec_key_at(result_1, j))
                is Child::Leaf<V>;
        ensures forall j in 0..ordered_map::spec_len(result_1):
            ordered_map::spec_get(result_1, ordered_map::spec_key_at(result_1, j))
                .value
                == spec_get(map, spec_key_at(map, spec_leaf_offset(self, map) + j));
        // Advancing consumes exactly this leaf's keys, and the walk ends only
        // once every key has been consumed.
        ensures spec_leaf_offset(result_2, map)
            == spec_leaf_offset(self, map) + ordered_map::spec_len(result_1);
        // An offset counts keys, so it never goes backwards past the start.
        // Without this a walk could sit at a negative position, where a
        // position-indexed aggregate is trivially zero.
        ensures spec_leaf_offset(result_2, map) >= 0;
        ensures internal_leaf_iter_is_end(result_2) ==>
            spec_leaf_offset(result_2, map) == spec_len(map);
    }
}
