// Two tables holding the same key stay separate, and keys of several types (address, byte
// vector, struct, String) round-trip through add, get, contains and remove. MonoMove only:
// the V1 side of the test harness has no table support.

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

    public fun add<K: copy + drop, V>(table: &mut Table<K, V>, key: K, value: V) {
        add_box<K, V, Box<V>>(table, key, Box { val: value })
    }

    public fun get<K: copy + drop, V>(table: &Table<K, V>, key: K): &V {
        &borrow_box<K, V, Box<V>>(table, key).val
    }

    public fun has<K: copy + drop, V>(table: &Table<K, V>, key: K): bool {
        contains_box<K, V, Box<V>>(table, key)
    }

    public fun remove<K: copy + drop, V>(table: &mut Table<K, V>, key: K): V {
        let Box { val } = remove_box<K, V, Box<V>>(table, key);
        val
    }

    public fun destroy<K: copy + drop, V>(table: Table<K, V>) {
        destroy_empty_box<K, V, Box<V>>(&table);
        drop_unchecked_box<K, V, Box<V>>(table)
    }

    native fun new_table_handle<K, V>(): address;
    native fun add_box<K: copy + drop, V, B>(table: &mut Table<K, V>, key: K, val: Box<V>);
    native fun borrow_box<K: copy + drop, V, B>(table: &Table<K, V>, key: K): &Box<V>;
    native fun contains_box<K: copy + drop, V, B>(table: &Table<K, V>, key: K): bool;
    native fun remove_box<K: copy + drop, V, B>(table: &mut Table<K, V>, key: K): Box<V>;
    native fun destroy_empty_box<K: copy + drop, V, B>(table: &Table<K, V>);
    native fun drop_unchecked_box<K: copy + drop, V, B>(table: Table<K, V>);
}
module 0x42::main {
    use std::string::{String, utf8};
    use 0x1::table;

    struct Key has copy, drop {
        a: u8,
        b: u64,
    }

    public fun isolated(): (vector<u8>, vector<u8>) {
        let t1 = table::make<u64, vector<u8>>();
        let t2 = table::make<u64, vector<u8>>();
        table::add(&mut t1, 7, b"a");
        table::add(&mut t2, 7, b"b");
        let first = *table::get(&t1, 7);
        let second = *table::get(&t2, 7);
        table::remove(&mut t1, 7);
        table::remove(&mut t2, 7);
        table::destroy(t1);
        table::destroy(t2);
        (first, second)
    }

    public fun address_keys(): (u64, bool, u64) {
        let map = table::make<address, u64>();
        table::add(&mut map, @0x1, 10);
        table::add(&mut map, @0x100, 20);
        let got = *table::get(&map, @0x100);
        let missing = table::has(&map, @0x2);
        let removed = table::remove(&mut map, @0x1) + table::remove(&mut map, @0x100);
        table::destroy(map);
        (got, missing, removed)
    }

    public fun bytes_keys(): (u64, bool, u64) {
        let map = table::make<vector<u8>, u64>();
        table::add(&mut map, b"", 1);
        table::add(&mut map, b"ab", 2);
        let got = *table::get(&map, b"ab");
        let missing = table::has(&map, b"a");
        let removed = table::remove(&mut map, b"") + table::remove(&mut map, b"ab");
        table::destroy(map);
        (got, missing, removed)
    }

    // `Key` is a u8 then a u64 (padded in memory); equal keys must find the same entry.
    public fun struct_keys(): (u64, bool, u64) {
        let map = table::make<Key, u64>();
        table::add(&mut map, Key { a: 1, b: 2 }, 5);
        table::add(&mut map, Key { a: 2, b: 1 }, 6);
        let got = *table::get(&map, Key { a: 1, b: 2 });
        let missing = table::has(&map, Key { a: 1, b: 1 });
        let removed = table::remove(&mut map, Key { a: 1, b: 2 }) + table::remove(&mut map, Key { a: 2, b: 1 });
        table::destroy(map);
        (got, missing, removed)
    }

    public fun string_keys(): (u64, bool, u64) {
        let map = table::make<String, u64>();
        table::add(&mut map, utf8(b"x"), 3);
        table::add(&mut map, utf8(b"xy"), 4);
        let got = *table::get(&map, utf8(b"xy"));
        let missing = table::has(&map, utf8(b"y"));
        let removed = table::remove(&mut map, utf8(b"x")) + table::remove(&mut map, utf8(b"xy"));
        table::destroy(map);
        (got, missing, removed)
    }
}

// RUN: execute 0x42::main::isolated
// CHECK-V2: results: 0x61, 0x62

// RUN: execute 0x42::main::address_keys
// CHECK-V2: results: 20, false, 30

// RUN: execute 0x42::main::bytes_keys
// CHECK-V2: results: 2, false, 3

// RUN: execute 0x42::main::struct_keys
// CHECK-V2: results: 5, false, 11

// RUN: execute 0x42::main::string_keys
// CHECK-V2: results: 4, false, 7
