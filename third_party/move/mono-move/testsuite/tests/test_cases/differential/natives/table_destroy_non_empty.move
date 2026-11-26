// The table native `destroy_empty_box` on a table that still holds entries: it does not
// abort, and the entries stay readable after it. The test's `destroy_check` calls it
// without consuming the table, which is then dropped unchecked. MonoMove only: the V1 side
// of the test harness has no table support.

// RUN: publish
module 0x1::table {
    struct Table<phantom K: copy + drop, phantom V> has store {
        handle: address,
    }

    struct Box<V> has key, drop, store {
        val: V,
    }

    public fun make<K: copy + drop, V: store>(): Table<K, V> {
        Table { handle: new_table_handle<K, V>() }
    }

    public fun add<K: copy + drop, V>(tbl: &mut Table<K, V>, key: K, val: V) {
        add_box<K, V, Box<V>>(tbl, key, Box { val })
    }

    public fun get<K: copy + drop, V>(tbl: &Table<K, V>, key: K): &V {
        &borrow_box<K, V, Box<V>>(tbl, key).val
    }

    public fun has<K: copy + drop, V>(tbl: &Table<K, V>, key: K): bool {
        contains_box<K, V, Box<V>>(tbl, key)
    }

    // The emptiness native alone, leaving the table usable.
    public fun destroy_check<K: copy + drop, V>(tbl: &Table<K, V>) {
        destroy_empty_box<K, V, Box<V>>(tbl)
    }

    public fun drop_unchecked<K: copy + drop, V>(tbl: Table<K, V>) {
        drop_unchecked_box<K, V, Box<V>>(tbl)
    }

    native fun new_table_handle<K, V>(): address;
    native fun add_box<K: copy + drop, V, B>(table: &mut Table<K, V>, key: K, val: Box<V>);
    native fun borrow_box<K: copy + drop, V, B>(table: &Table<K, V>, key: K): &Box<V>;
    native fun contains_box<K: copy + drop, V, B>(table: &Table<K, V>, key: K): bool;
    native fun destroy_empty_box<K: copy + drop, V, B>(table: &Table<K, V>);
    native fun drop_unchecked_box<K: copy + drop, V, B>(table: Table<K, V>);
}
module 0x42::main {
    use 0x1::table;

    // Entries read before and after the emptiness native.
    public fun read_around_destroy(): (bool, vector<u8>, bool, vector<u8>) {
        let entries = table::make<u64, vector<u8>>();
        table::add(&mut entries, 7, b"hi");
        table::add(&mut entries, 8, b"yo");
        let had_before = table::has(&entries, 7);
        let before = *table::get(&entries, 8);
        table::destroy_check(&entries);
        let had_after = table::has(&entries, 7);
        let after = *table::get(&entries, 8);
        table::drop_unchecked(entries);
        (had_before, before, had_after, after)
    }
}

// RUN: execute 0x42::main::read_around_destroy
// CHECK-V2: results: true, 0x796f, true, 0x796f
