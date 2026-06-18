// V2-only tests for the `table` natives, against a generic `Table<K, V>` /
// `Box<V>` module mirroring aptos_std::table (it is not in the bundled stdlib,
// so it is declared here).
//
// TODO: Enable V1 runs by installing the NativeTableContext extension.

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

    public fun add<K: copy + drop, V>(t: &mut Table<K, V>, k: K, v: V) {
        add_box<K, V, Box<V>>(t, k, Box { val: v })
    }

    public fun get<K: copy + drop, V>(t: &Table<K, V>, k: K): &V {
        &borrow_box<K, V, Box<V>>(t, k).val
    }

    public fun set<K: copy + drop, V: drop>(t: &mut Table<K, V>, k: K, v: V) {
        borrow_box_mut<K, V, Box<V>>(t, k).val = v
    }

    public fun has<K: copy + drop, V>(t: &Table<K, V>, k: K): bool {
        contains_box<K, V, Box<V>>(t, k)
    }

    public fun remove<K: copy + drop, V>(t: &mut Table<K, V>, k: K): V {
        let Box { val } = remove_box<K, V, Box<V>>(t, k);
        val
    }

    public fun destroy<K: copy + drop, V>(t: Table<K, V>) {
        destroy_empty_box<K, V, Box<V>>(&t);
        drop_unchecked_box<K, V, Box<V>>(t)
    }

    native fun new_table_handle<K, V>(): address;
    native fun add_box<K: copy + drop, V, B>(table: &mut Table<K, V>, key: K, val: Box<V>);
    native fun borrow_box<K: copy + drop, V, B>(table: &Table<K, V>, key: K): &Box<V>;
    native fun borrow_box_mut<K: copy + drop, V, B>(table: &mut Table<K, V>, key: K): &mut Box<V>;
    native fun contains_box<K: copy + drop, V, B>(table: &Table<K, V>, key: K): bool;
    native fun remove_box<K: copy + drop, V, B>(table: &mut Table<K, V>, key: K): Box<V>;
    native fun destroy_empty_box<K: copy + drop, V, B>(table: &Table<K, V>);
    native fun drop_unchecked_box<K: copy + drop, V, B>(table: Table<K, V>);
}
module 0x42::main {
    use 0x1::table;

    public fun present(): bool {
        let t = table::make<u64, vector<u8>>();
        table::add(&mut t, 7, b"hi");
        let r = table::has(&t, 7);
        table::destroy(t);
        r
    }

    public fun absent(): bool {
        let t = table::make<u64, vector<u8>>();
        let r = table::has(&t, 9);
        table::destroy(t);
        r
    }

    public fun get_after_add(): vector<u8> {
        let t = table::make<u64, vector<u8>>();
        table::add(&mut t, 7, b"hello");
        let r = *table::get(&t, 7);
        table::destroy(t);
        r
    }

    public fun set_then_get(): vector<u8> {
        let t = table::make<u64, vector<u8>>();
        table::add(&mut t, 8, b"ab");
        table::set(&mut t, 8, b"cd");
        let r = *table::get(&t, 8);
        table::destroy(t);
        r
    }

    public fun add_duplicate_aborts(): bool {
        let t = table::make<u64, vector<u8>>();
        table::add(&mut t, 7, b"a");
        table::add(&mut t, 7, b"b");
        table::destroy(t);
        true
    }

    public fun get_missing_aborts(): vector<u8> {
        let t = table::make<u64, vector<u8>>();
        let r = *table::get(&t, 5);
        table::destroy(t);
        r
    }

    public fun remove_present(): vector<u8> {
        let t = table::make<u64, vector<u8>>();
        table::add(&mut t, 7, b"hi");
        let v = table::remove(&mut t, 7);
        table::destroy(t);
        v
    }

    public fun removed_then_absent(): bool {
        let t = table::make<u64, vector<u8>>();
        table::add(&mut t, 7, b"hi");
        let _ = table::remove(&mut t, 7);
        let gone = !table::has(&t, 7);
        table::destroy(t);
        gone
    }

    public fun destroy_empty(): bool {
        let t = table::make<u64, vector<u8>>();
        table::destroy(t);
        true
    }

    public fun remove_missing_aborts(): vector<u8> {
        let t = table::make<u64, vector<u8>>();
        let r = table::remove(&mut t, 5);
        table::destroy(t);
        r
    }
}

// RUN: execute 0x42::main::present
// CHECK-V2: results: true

// RUN: execute 0x42::main::absent
// CHECK-V2: results: false

// RUN: execute 0x42::main::get_after_add
// CHECK-V2: results: 0x68656c6c6f

// RUN: execute 0x42::main::set_then_get
// CHECK-V2: results: 0x6364

// RUN: execute 0x42::main::add_duplicate_aborts
// CHECK-V2: aborted: code 25607

// RUN: execute 0x42::main::get_missing_aborts
// CHECK-V2: aborted: code 25863

// RUN: execute 0x42::main::remove_present
// CHECK-V2: results: 0x6869

// RUN: execute 0x42::main::removed_then_absent
// CHECK-V2: results: true

// RUN: execute 0x42::main::destroy_empty
// CHECK-V2: results: true

// RUN: execute 0x42::main::remove_missing_aborts
// CHECK-V2: aborted: code 25863
