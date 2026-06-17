// V2-only tests for the `table` natives.
//
// TODO: This currently uses a MOCK table module with generics partially stripped.
// This is required because generic structs/enums are not yet supported by the
// specializer. Fix this once support lands.
//
// This works due to a lucky coincidence -- the table native impl is compatible
// with both the mock `Table/Box` and the real `Table<K, V>/Box<V>` at the binary
// level.
//
// TODO: Enable V1 runs by installing the NativeTableContext extension.

// RUN: publish
module 0x1::table {
    struct Table has store, drop {
        handle: address,
    }

    struct Box has store, drop {
        val: vector<u8>,
    }

    public fun make(): Table {
        Table { handle: new_table_handle() }
    }

    public fun add(t: &mut Table, k: u64, v: vector<u8>) {
        add_box<u64>(t, k, Box { val: v })
    }

    public fun get(t: &Table, k: u64): vector<u8> {
        *&borrow_box<u64>(t, k).val
    }

    public fun set(t: &mut Table, k: u64, v: vector<u8>) {
        borrow_box_mut<u64>(t, k).val = v
    }

    public fun has(t: &Table, k: u64): bool {
        contains_box<u64>(t, k)
    }

    native fun new_table_handle(): address;
    native fun add_box<K: copy + drop>(table: &mut Table, key: K, val: Box);
    native fun borrow_box<K: copy + drop>(table: &Table, key: K): &Box;
    native fun borrow_box_mut<K: copy + drop>(table: &mut Table, key: K): &mut Box;
    native fun contains_box<K: copy + drop>(table: &Table, key: K): bool;
}
module 0x42::main {
    use 0x1::table;

    public fun present(): bool {
        let t = table::make();
        table::add(&mut t, 7, b"hi");
        table::has(&t, 7)
    }

    public fun absent(): bool {
        let t = table::make();
        table::has(&t, 9)
    }

    public fun get_after_add(): vector<u8> {
        let t = table::make();
        table::add(&mut t, 7, b"hello");
        table::get(&t, 7)
    }

    public fun set_then_get(): vector<u8> {
        let t = table::make();
        table::add(&mut t, 8, b"ab");
        table::set(&mut t, 8, b"cd");
        table::get(&t, 8)
    }

    public fun add_duplicate_aborts(): bool {
        let t = table::make();
        table::add(&mut t, 7, b"a");
        table::add(&mut t, 7, b"b");
        true
    }

    public fun get_missing_aborts(): vector<u8> {
        let t = table::make();
        table::get(&t, 5)
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
