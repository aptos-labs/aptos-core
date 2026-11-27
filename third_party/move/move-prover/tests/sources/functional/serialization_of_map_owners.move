// Serialization of intrinsic map owners: BCS serializes the physical owner,
// which holds a handle or nothing, never the entries. Equal serializations
// give equal handles of a handle owner, and nothing about the entries of any
// owner. A struct holding an owner gives its other fields.
module 0x42::placeholder_table {
    struct Table<phantom K: copy + drop, phantom V> has store {}
    spec Table {
        pragma intrinsic = map,
        map_new = new,
        map_destroy_empty = destroy_empty,
        map_len = length,
        map_is_empty = empty,
        map_has_key = contains,
        map_add_no_override = add,
        map_add_override_if_exists = upsert,
        map_del_must_exist = remove,
        map_del_return_key = remove_return_key,
        map_borrow = borrow,
        map_borrow_mut = borrow_mut,
        map_borrow_mut_with_default = borrow_mut_with_default,
        map_spec_new = spec_new,
        map_spec_get = spec_get,
        map_spec_set = spec_set,
        map_spec_del = spec_remove,
        map_spec_len = spec_len,
        map_spec_has_key = spec_contains;
    }

    public native fun new<K: copy + drop, V: store>(): Table<K, V>;
    public native fun destroy_empty<K: copy + drop, V>(table: Table<K, V>);
    public native fun add<K: copy + drop, V>(table: &mut Table<K, V>, key: K, val: V);
    public native fun upsert<K: copy + drop, V>(table: &mut Table<K, V>, key: K, val: V);
    public native fun borrow<K: copy + drop, V>(table: &Table<K, V>, key: K): &V;
    public native fun borrow_mut<K: copy + drop, V>(table: &mut Table<K, V>, key: K): &mut V;
    public native fun borrow_mut_with_default<K: copy + drop, V>(table: &mut Table<K, V>, key: K, default: V): &mut V;
    public native fun length<K: copy + drop, V>(table: &Table<K, V>): u64;
    public native fun empty<K: copy + drop, V>(table: &Table<K, V>): bool;
    public native fun remove<K: copy + drop, V>(table: &mut Table<K, V>, key: K): V;
    public native fun remove_return_key<K: copy + drop, V>(table: &mut Table<K, V>, key: K): (K, V);
    public native fun contains<K: copy + drop, V>(table: &Table<K, V>, key: K): bool;

    spec native fun spec_new<K, V>(): Table<K, V>;
    spec native fun spec_len<K, V>(t: Table<K, V>): num;
    spec native fun spec_contains<K, V>(t: Table<K, V>, k: K): bool;
    spec native fun spec_set<K, V>(t: Table<K, V>, k: K, v: V): Table<K, V>;
    spec native fun spec_remove<K, V>(t: Table<K, V>, k: K): Table<K, V>;
    spec native fun spec_get<K, V>(t: Table<K, V>, k: K): V;
}

module 0x42::handle_table {
    use std::bcs;

    struct Table<phantom K: copy + drop, phantom V> has store { handle: address }
    spec Table {
        pragma intrinsic = map,
        map_new = new,
        map_destroy_empty = destroy_empty,
        map_len = length,
        map_is_empty = empty,
        map_has_key = contains,
        map_add_no_override = add,
        map_add_override_if_exists = upsert,
        map_del_must_exist = remove,
        map_del_return_key = remove_return_key,
        map_borrow = borrow,
        map_borrow_mut = borrow_mut,
        map_borrow_mut_with_default = borrow_mut_with_default,
        map_spec_new = spec_new,
        map_spec_get = spec_get,
        map_spec_set = spec_set,
        map_spec_del = spec_remove,
        map_spec_len = spec_len,
        map_spec_has_key = spec_contains;
    }

    public native fun new<K: copy + drop, V: store>(): Table<K, V>;
    public native fun destroy_empty<K: copy + drop, V>(table: Table<K, V>);
    public native fun add<K: copy + drop, V>(table: &mut Table<K, V>, key: K, val: V);
    public native fun upsert<K: copy + drop, V>(table: &mut Table<K, V>, key: K, val: V);
    public native fun borrow<K: copy + drop, V>(table: &Table<K, V>, key: K): &V;
    public native fun borrow_mut<K: copy + drop, V>(table: &mut Table<K, V>, key: K): &mut V;
    public native fun borrow_mut_with_default<K: copy + drop, V>(table: &mut Table<K, V>, key: K, default: V): &mut V;
    public native fun length<K: copy + drop, V>(table: &Table<K, V>): u64;
    public native fun empty<K: copy + drop, V>(table: &Table<K, V>): bool;
    public native fun remove<K: copy + drop, V>(table: &mut Table<K, V>, key: K): V;
    public native fun remove_return_key<K: copy + drop, V>(table: &mut Table<K, V>, key: K): (K, V);
    public native fun contains<K: copy + drop, V>(table: &Table<K, V>, key: K): bool;

    spec native fun spec_new<K, V>(): Table<K, V>;
    spec native fun spec_len<K, V>(t: Table<K, V>): num;
    spec native fun spec_contains<K, V>(t: Table<K, V>, k: K): bool;
    spec native fun spec_set<K, V>(t: Table<K, V>, k: K, v: V): Table<K, V>;
    spec native fun spec_remove<K, V>(t: Table<K, V>, k: K): Table<K, V>;
    spec native fun spec_get<K, V>(t: Table<K, V>, k: K): V;

    // The handle conclusion is usable where the handle can be read.
    fun same_handle(a: &Table<u64, u64>, b: &Table<u64, u64>): bool { bcs::to_bytes(a) == bcs::to_bytes(b) }
    spec same_handle {
        ensures result ==> a.handle == b.handle;
    }
}

module 0x42::serialization_of_map_owners {
    use std::bcs;
    use 0x42::placeholder_table;
    use 0x42::handle_table;

    struct PlaceholderHolder has store { id: u64, table: placeholder_table::Table<u64, u64> }
    struct HandleHolder has store { id: u64, table: handle_table::Table<u64, u64> }

    // A placeholder owner serializes alike whatever its entries.
    fun placeholder_len(a: &placeholder_table::Table<u64, u64>, b: &placeholder_table::Table<u64, u64>): bool {
        bcs::to_bytes(a) == bcs::to_bytes(b)
    }
    spec placeholder_len {
        ensures result ==> placeholder_table::spec_len(a) == placeholder_table::spec_len(b); // error: the entries are not serialized
    }

    fun placeholder_contains(a: &placeholder_table::Table<u64, u64>, b: &placeholder_table::Table<u64, u64>): bool {
        bcs::to_bytes(a) == bcs::to_bytes(b)
    }
    spec placeholder_contains {
        ensures result ==> placeholder_table::spec_contains(a, 1) == placeholder_table::spec_contains(b, 1); // error: the entries are not serialized
    }

    fun placeholder_spec_only(a: &placeholder_table::Table<u64, u64>, b: &placeholder_table::Table<u64, u64>): bool { true }
    spec placeholder_spec_only {
        ensures bcs::serialize(a) == bcs::serialize(b) ==> placeholder_table::spec_len(a) == placeholder_table::spec_len(b); // error: the entries are not serialized
    }

    fun placeholder_lengths(a: &placeholder_table::Table<u64, u64>, b: &placeholder_table::Table<u64, u64>): bool {
        bcs::to_bytes(a) == bcs::to_bytes(b)
    }
    spec placeholder_lengths {
        ensures result ==> len(bcs::serialize(a)) == len(bcs::serialize(b));
    }

    // The fields of a struct holding an owner, other than the owner's entries.
    fun placeholder_holder_id(a: &PlaceholderHolder, b: &PlaceholderHolder): bool { bcs::to_bytes(a) == bcs::to_bytes(b) }
    spec placeholder_holder_id {
        ensures result ==> a.id == b.id;
    }

    fun placeholder_holder_len(a: &PlaceholderHolder, b: &PlaceholderHolder): bool { bcs::to_bytes(a) == bcs::to_bytes(b) }
    spec placeholder_holder_len {
        ensures result ==> placeholder_table::spec_len(a.table) == placeholder_table::spec_len(b.table); // error: the entries are not serialized
    }

    fun handle_holder_id(a: &HandleHolder, b: &HandleHolder): bool { bcs::to_bytes(a) == bcs::to_bytes(b) }
    spec handle_holder_id {
        ensures result ==> a.id == b.id;
    }

    // Equal handles at one state hold one table, but no law relates two
    // snapshots of a handle's entries, so this does not follow.
    fun handle_len(a: &handle_table::Table<u64, u64>, b: &handle_table::Table<u64, u64>): bool {
        bcs::to_bytes(a) == bcs::to_bytes(b)
    }
    spec handle_len {
        ensures result ==> handle_table::spec_len(a) == handle_table::spec_len(b); // error: no law relates a handle's entries
    }
}
