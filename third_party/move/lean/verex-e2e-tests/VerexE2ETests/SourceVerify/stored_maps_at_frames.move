// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// A stored map's validity holds wherever it is read or written: in a generic
/// function over a resource with no type parameters, and over a generic resource,
/// where the map's key and value types are or are not its parameters.
module 0x1::stored_maps_at_frames {
    struct Table<phantom K: copy + drop, phantom V> has store {
        handle: address,
    }

    spec Table {
        pragma intrinsic = map,
            map_new = new,
            map_has_key = contains,
            map_add_no_override = add,
            map_borrow = borrow,
            map_spec_has_key = spec_contains,
            map_spec_get = spec_get,
            map_spec_set = spec_set,
            map_spec_del = spec_remove;
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

    native fun entry<K: copy + drop, V>(self: &Table<K, V>, key: K): &V;
    public fun borrow<K: copy + drop, V>(self: &Table<K, V>, key: K): &V {
        entry(self, key)
    }
    spec borrow { pragma intrinsic; }

    spec native fun spec_contains<K, V>(t: Table<K, V>, k: K): bool;
    spec native fun spec_set<K, V>(t: Table<K, V>, k: K, v: V): Table<K, V>;
    spec native fun spec_get<K, V>(t: Table<K, V>, k: K): V;
    spec native fun spec_remove<K, V>(t: Table<K, V>, k: K): Table<K, V>;

    struct Key has copy, drop, store { id: vector<u8> }

    struct Entry<T: store> has store { payload: T, count: u64 }

    /// No type parameters, read by generic functions.
    struct Registry has key {
        entries: Table<Key, u64>,
    }

    /// Generic, with a table not mentioning the parameter and one mentioning it.
    struct Forum<T: store> has key {
        counts: Table<Key, u64>,
        proposals: Table<u64, Entry<T>>,
    }

    fun registered<T>(key: Key): bool {
        let table = &Registry[@0x1].entries;
        contains(table, key)
    }
    spec registered {
        aborts_if !exists<Registry>(@0x1);
        ensures result == spec_contains(global<Registry>(@0x1).entries, key);
    }

    fun registered_false<T>(key: Key): bool {
        let table = &Registry[@0x1].entries;
        contains(table, key)
    }
    spec registered_false {
        aborts_if !exists<Registry>(@0x1);
        ensures result; // error: the key may be absent
    }

    fun registered_count<T>(key: Key): u64 {
        let table = &Registry[@0x1].entries;
        *borrow(table, key)
    }
    spec registered_count {
        aborts_if !exists<Registry>(@0x1);
        aborts_if !spec_contains(global<Registry>(@0x1).entries, key);
        ensures result == spec_get(global<Registry>(@0x1).entries, key);
    }

    fun has_count<T: store>(addr: address, key: Key): bool {
        let table = &Forum<T>[addr].counts;
        contains(table, key)
    }
    spec has_count {
        aborts_if !exists<Forum<T>>(addr);
        ensures result == spec_contains(global<Forum<T>>(addr).counts, key);
    }

    fun count_of<T: store>(addr: address, key: Key): u64 {
        let table = &Forum<T>[addr].counts;
        *borrow(table, key)
    }
    spec count_of {
        aborts_if !exists<Forum<T>>(addr);
        aborts_if !spec_contains(global<Forum<T>>(addr).counts, key);
        ensures result == spec_get(global<Forum<T>>(addr).counts, key);
    }

    fun has_proposal<T: store>(addr: address, id: u64): bool {
        let table = &Forum<T>[addr].proposals;
        contains(table, id)
    }
    spec has_proposal {
        aborts_if !exists<Forum<T>>(addr);
        ensures result == spec_contains(global<Forum<T>>(addr).proposals, id);
    }

    fun proposal_count<T: store>(addr: address, id: u64): u64 {
        let table = &Forum<T>[addr].proposals;
        borrow(table, id).count
    }
    spec proposal_count {
        aborts_if !exists<Forum<T>>(addr);
        aborts_if !spec_contains(global<Forum<T>>(addr).proposals, id);
        ensures result == spec_get(global<Forum<T>>(addr).proposals, id).count;
    }

    fun proposal_count_false<T: store>(addr: address, id: u64): u64 {
        let table = &Forum<T>[addr].proposals;
        borrow(table, id).count
    }
    spec proposal_count_false {
        aborts_if !exists<Forum<T>>(addr);
        aborts_if !spec_contains(global<Forum<T>>(addr).proposals, id);
        ensures result == 0; // error: the count may be any
    }

    /// A map value written by a generic function.
    fun register<T>(key: Key, value: u64) {
        let table = &mut Registry[@0x1].entries;
        add(table, key, value)
    }
    spec register {
        modifies global<Registry>(@0x1);
        aborts_if !exists<Registry>(@0x1);
        aborts_if spec_contains(global<Registry>(@0x1).entries, key);
        ensures spec_contains(global<Registry>(@0x1).entries, key);
    }

    fun proposal_count_u64(addr: address, id: u64): u64 {
        let table = &Forum<u64>[addr].proposals;
        borrow(table, id).count
    }
    spec proposal_count_u64 {
        aborts_if !exists<Forum<u64>>(addr);
        aborts_if !spec_contains(global<Forum<u64>>(addr).proposals, id);
        ensures result == spec_get(global<Forum<u64>>(addr).proposals, id).count;
    }
}
