// Iterator-validity predicates must not appear in data invariants, on any
// type: data invariants are assumed for opaque results without ever being
// proven (intrinsic producers have no pack site), so such an invariant could
// pin hidden validity slots to `spec_new()`'s fixed slot and revalidate a
// stale iterator after a structural map mutation.

// Surface 1: data invariant on the iterator enum itself.
module 0x42::validity_inv_enum {
    struct Table<phantom K: copy + drop, phantom V> has store, drop, copy {}

    enum IteratorPtr<K: copy + drop> has copy, drop {
        End,
        Some { key: K },
    }
    spec IteratorPtr {
        invariant spec_iter_valid(self, spec_new<K, u64>());
    }

    spec Table {
        pragma intrinsic = map,
            map_new = new,
            map_spec_new = spec_new,
            map_spec_iter_valid = spec_iter_valid;
    }

    public native fun new<K: copy + drop, V: store>(): Table<K, V>;
    spec native fun spec_new<K, V>(): Table<K, V>;
    spec native fun spec_iter_valid<K, V>(it: IteratorPtr<K>, t: Table<K, V>): bool;

    fun mk(): Table<u8, u64> {
        new()
    }
}

// Surface 2: data invariant on a plain-struct (non-enum) walker bound
// through `map_spec_leaf_iter_valid`.
module 0x42::validity_inv_leaf {
    struct Table<phantom K: copy + drop, phantom V> has store, drop, copy {}

    struct NodePtr has copy, drop {
        node_index: u64,
    }
    spec NodePtr {
        invariant spec_node_valid(self, spec_new<u8, u64>());
    }

    spec Table {
        pragma intrinsic = map,
            map_new = new,
            map_spec_new = spec_new,
            map_spec_leaf_iter_valid = spec_node_valid;
    }

    public native fun new<K: copy + drop, V: store>(): Table<K, V>;
    spec native fun spec_new<K, V>(): Table<K, V>;
    spec native fun spec_node_valid<K, V>(it: NodePtr, t: Table<K, V>): bool;

    fun mk(): Table<u8, u64> {
        new()
    }
}

// Surfaces 3 and 4 share a clean map module: the invariants live in CLIENT
// modules wrapping the iterator.
module 0x42::validity_inv_map {
    struct Table<phantom K: copy + drop, phantom V> has store, drop, copy {}

    enum IteratorPtr<K: copy + drop> has copy, drop {
        End,
        Some { key: K },
    }

    spec Table {
        pragma intrinsic = map,
            map_new = new,
            map_spec_new = spec_new,
            map_spec_iter_valid = spec_iter_valid,
            map_spec_iter_preserved = spec_iter_preserved;
    }

    public native fun new<K: copy + drop, V: store>(): Table<K, V>;
    spec native fun spec_new<K, V>(): Table<K, V>;
    spec native fun spec_iter_valid<K, V>(it: IteratorPtr<K>, t: Table<K, V>): bool;
    spec native fun spec_iter_preserved<K, V>(t_new: Table<K, V>, t_old: Table<K, V>): bool;

    fun mk(): Table<u8, u64> {
        new()
    }
}

// Surface 3: direct call in a client wrapper's data invariant.
module 0x42::validity_inv_wrapper {
    use 0x42::validity_inv_map::{Self, IteratorPtr};

    struct Holder has copy, drop {
        it: IteratorPtr<u8>,
    }
    spec Holder {
        invariant validity_inv_map::spec_iter_valid(self.it, validity_inv_map::spec_new<u8, u64>());
    }
}

// Surface 4: the predicate reached transitively through a helper spec fun
// (here `map_spec_iter_preserved`, the two-map frame predicate).
module 0x42::validity_inv_transitive {
    use 0x42::validity_inv_map::{Self, Table};

    struct TwoMaps has store, drop {
        a: Table<u8, u64>,
        b: Table<u8, u64>,
    }
    spec TwoMaps {
        invariant same_epoch(self);
    }
    spec fun same_epoch(s: TwoMaps): bool {
        validity_inv_map::spec_iter_preserved(s.a, s.b)
    }
}

// Surface 5: the predicate laundered through a behavioral operator — the
// invariant never calls a validity spec fun directly.
module 0x42::validity_inv_behavioral {
    use 0x42::validity_inv_map::{Self, Table, IteratorPtr};

    struct Holder has copy, drop {
        it: IteratorPtr<u8>,
    }
    spec Holder {
        invariant requires_of<validity_gate>(self.it, validity_inv_map::spec_new<u8, u64>());
    }

    public fun validity_gate(_it: IteratorPtr<u8>, _t: Table<u8, u64>) {}
    spec validity_gate {
        requires validity_inv_map::spec_iter_valid(_it, _t);
    }
}

// Surface 6: the behavioral operator hidden inside a helper spec fun.
module 0x42::validity_inv_behavioral_helper {
    use 0x42::validity_inv_map::{Self, Table, IteratorPtr};

    struct Holder has copy, drop {
        it: IteratorPtr<u8>,
    }
    spec Holder {
        invariant gated(self.it);
    }
    spec fun gated(it: IteratorPtr<u8>): bool {
        requires_of<validity_gate>(it, validity_inv_map::spec_new<u8, u64>())
    }

    public fun validity_gate(_it: IteratorPtr<u8>, _t: Table<u8, u64>) {}
    spec validity_gate {
        requires validity_inv_map::spec_iter_valid(_it, _t);
    }
}
