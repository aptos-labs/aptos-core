// `table::add` of a struct with two vector fields, on a heap where the native's allocation
// for the new entry collects while the value is still only the native's argument.
// `add_fill` runs the same preamble without the native and does not collect; `add_gc` then
// reads each field back from the table. `many` adds and removes eight entries with churn
// between the adds. MonoMove only: the V1 side of the test harness has no table support.

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

    public fun remove<K: copy + drop, V>(table: &mut Table<K, V>, key: K): V {
        let Box { val } = remove_box<K, V, Box<V>>(table, key);
        val
    }

    public fun destroy<K: copy + drop, V>(table: Table<K, V>) {
        destroy_empty_box<K, V, Box<V>>(&table);
        drop_unchecked_box<K, V, Box<V>>(table)
    }

    public fun drop_unchecked<K: copy + drop, V>(table: Table<K, V>) {
        drop_unchecked_box<K, V, Box<V>>(table)
    }

    native fun new_table_handle<K, V>(): address;
    native fun add_box<K: copy + drop, V, B>(table: &mut Table<K, V>, key: K, val: Box<V>);
    native fun borrow_box<K: copy + drop, V, B>(table: &Table<K, V>, key: K): &Box<V>;
    native fun remove_box<K: copy + drop, V, B>(table: &mut Table<K, V>, key: K): Box<V>;
    native fun destroy_empty_box<K: copy + drop, V, B>(table: &Table<K, V>);
    native fun drop_unchecked_box<K: copy + drop, V, B>(table: Table<K, V>);
}
module 0x42::main {
    use 0x1::table;

    struct Rec has drop, store {
        a: vector<u8>,
        n: u64,
        b: vector<u64>,
    }

    fun churn(rounds: u64) {
        let counter = 0;
        while (counter < rounds) {
            let junk = vector[counter, counter, counter, counter];
            counter = counter + junk.length();
        };
    }

    public fun add_gc(rounds: u64): (u64, u8, u64, u64) {
        let map = table::make<u64, Rec>();
        let rec = Rec { a: vector[1u8, 2, 3], n: 7, b: vector[40u64, 50] };
        churn(rounds);
        table::add(&mut map, 7, rec);
        let got = table::get(&map, 7);
        let out_a = got.a.length();
        let out_a2 = got.a[2];
        let out_n = got.n;
        let out_b1 = got.b[1];
        let Rec { a: _, n: _, b: _ } = table::remove(&mut map, 7);
        table::destroy(map);
        (out_a, out_a2, out_n, out_b1)
    }

    public fun add_only(rounds: u64): u64 {
        let map = table::make<u64, Rec>();
        let rec = Rec { a: vector[1u8, 2, 3], n: 7, b: vector[40u64, 50] };
        churn(rounds);
        table::add(&mut map, 7, rec);
        table::drop_unchecked(map);
        rounds
    }

    public fun add_fill(rounds: u64): u64 {
        let map = table::make<u64, Rec>();
        let rec = Rec { a: vector[1u8, 2, 3], n: 7, b: vector[40u64, 50] };
        churn(rounds);
        table::destroy(map);
        rec.n
    }

    // Many entries, each added after churn, then all removed in order.
    public fun many(count: u64, rounds: u64): vector<u8> {
        let map = table::make<u64, vector<u8>>();
        let index = 0;
        while (index < count) {
            churn(rounds);
            table::add(&mut map, index, vector[(index as u8), (index as u8)]);
            index += 1;
        };
        churn(rounds);
        let out = vector[];
        index = 0;
        while (index < count) {
            out.append(table::remove(&mut map, index));
            index += 1;
        };
        table::destroy(map);
        out
    }
}

// RUN: execute 0x42::main::add_fill --args 80 --heap-size 1024
// CHECK-V2: results: 7
// CHECK-GC-COUNT: 0

// RUN: execute 0x42::main::add_only --args 80 --heap-size 1024
// CHECK-V2: results: 80
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::main::add_gc --args 80 --heap-size 1024
// CHECK-V2: results: 3, 3, 7, 50
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::main::many --args 8, 300 --heap-size 1024
// CHECK-V2: results: 0x00000101020203030404050506060707
// CHECK-GC-COUNT: 41
