// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// A map equated entry by entry to a valid map, at a value type that compares
/// by entries, is valid: what a map role called on it owes. A spec variable
/// makes the call's result a value compared part by part.
module 0x1::proof_equal_tables {
    struct Table<phantom K: copy + drop, phantom V> has store {
        handle: address,
    }
    spec Table {
        pragma intrinsic = map,
            map_new = new,
            map_has_key = contains,
            map_len = length,
            map_spec_len = spec_len,
            map_add_no_override = add,
            map_spec_has_key = spec_contains,
            map_spec_get = spec_get,
            map_spec_set = spec_set,
            map_spec_del = spec_remove;
    }

    spec module {
        global calls: u64;
    }

    native fun new_table_handle<K, V>(): address;

    public fun new<K: copy + drop, V>(): Table<K, V> {
        Table { handle: new_table_handle<K, V>() }
    }
    spec new { pragma intrinsic; }

    native fun add_entry<K: copy + drop, V>(self: &mut Table<K, V>, key: K, val: V);
    public fun add<K: copy + drop, V>(self: &mut Table<K, V>, key: K, val: V) {
        add_entry(self, key, val)
    }
    spec add { pragma intrinsic; }

    native fun has_entry<K: copy + drop, V>(self: &Table<K, V>, key: K): bool;
    public fun contains<K: copy + drop, V>(self: &Table<K, V>, key: K): bool {
        has_entry(self, key)
    }
    spec contains { pragma intrinsic; }

    native fun entry_count<K: copy + drop, V>(self: &Table<K, V>): u64;
    public fun length<K: copy + drop, V>(self: &Table<K, V>): u64 {
        entry_count(self)
    }
    spec length { pragma intrinsic; }

    spec native fun spec_len<K, V>(t: Table<K, V>): num;
    spec native fun spec_contains<K, V>(t: Table<K, V>, k: K): bool;
    spec native fun spec_set<K, V>(t: Table<K, V>, k: K, v: V): Table<K, V>;
    spec native fun spec_get<K, V>(t: Table<K, V>, k: K): V;
    spec native fun spec_remove<K, V>(t: Table<K, V>, k: K): Table<K, V>;

    struct Bag<V> has store {
        entries: Table<u64, vector<V>>,
        count: u64,
    }
    spec Bag {
        invariant count == 0 || spec_contains(entries, 0);
    }

    enum Slot<E> has copy, drop, store {
        Empty,
        Full { payload: E },
    }

    spec fun spec_payload<E>(s: Slot<E>): E {
        if (s is Full) {
            s.payload
        } else {
            abort 0u64
        }
    }

    // At a type parameter the result carries no validity of its own; the
    // comparison, of a parameter's payload, relates it to the payload's.
    native fun payload<E>(s: &Slot<E>): &E;
    spec payload {
        pragma opaque;
        requires s is Full;
        ensures result == spec_payload(s);
    }

    public fun count_of<V: copy>(b: &Bag<V>): u64 {
        length(&b.entries)
    }

    struct Outer<V> has store {
        inner: vector<V>,
        big: Slot<Bag<V>>,
    }

    fun caller<V: store + copy>(o: &Outer<V>): u64 {
        if (o.big is Slot::Full) {
            let bag = payload(&o.big);
            count_of(bag)
        } else {
            0
        }
    }
    spec caller {
        pragma verify = true;
    }
}
