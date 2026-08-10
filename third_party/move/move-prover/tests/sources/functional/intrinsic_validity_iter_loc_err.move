// A validity predicate's iterator (first) parameter must be a type declared
// in the map's own module — its hidden slot is synthesized while that module
// is built. The diagnostic points at the offending binding.
module 0x42::validity_iter_loc_other {
    struct ForeignPtr has copy, drop {
        pos: u64,
    }
}

// First parameter is not a struct type at all.
module 0x42::validity_iter_loc_prim {
    struct Table<phantom K: copy + drop, phantom V> has store, drop {}
    spec Table {
        pragma intrinsic = map,
            map_new = new,
            map_spec_iter_valid = spec_iter_valid;
    }
    native fun new<K: copy + drop, V: store>(): Table<K, V>;
    spec native fun spec_iter_valid<K, V>(it: u64, t: Table<K, V>): bool;

    fun mk(): Table<u8, u64> {
        new()
    }
}

// First parameter is a struct declared in a different module.
module 0x42::validity_iter_loc_foreign {
    use 0x42::validity_iter_loc_other::ForeignPtr;

    struct Table<phantom K: copy + drop, phantom V> has store, drop {}
    spec Table {
        pragma intrinsic = map,
            map_new = new,
            map_spec_leaf_iter_valid = spec_node_valid;
    }
    native fun new<K: copy + drop, V: store>(): Table<K, V>;
    spec native fun spec_node_valid<K, V>(it: ForeignPtr, t: Table<K, V>): bool;

    fun mk(): Table<u8, u64> {
        new()
    }
}
