// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// A unit declaring ghost state, proved over every family because it routes
/// generic callees: a stored table's validity holds where it is read and written,
/// an `[abstract]` clause is what a caller sees of an opaque callee, and the memory
/// after an allocation reads as before it, with its stored invariants.
module 0x1::table_stored_resource {
    use std::signer;

    struct Table<phantom K: copy + drop, phantom V> has store {
        handle: address,
    }

    spec Table {
        pragma intrinsic = map,
            map_new = new,
            map_has_key = contains,
            map_add_no_override = add,
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

    spec native fun spec_contains<K, V>(t: Table<K, V>, k: K): bool;
    spec native fun spec_set<K, V>(t: Table<K, V>, k: K, v: V): Table<K, V>;
    spec native fun spec_get<K, V>(t: Table<K, V>, k: K): V;
    spec native fun spec_remove<K, V>(t: Table<K, V>, k: K): Table<K, V>;

    struct Holder has key {
        entries: Table<address, u64>,
    }

    spec module {
        global draws: u64;
    }

    fun flag(): bool { true }
    spec flag {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_flag();
    }

    spec fun spec_flag(): bool;

    fun read_flag(): bool { flag() }
    spec read_flag {
        aborts_if false;
        ensures result == spec_flag();
    }

    fun read_flag_false(): bool { flag() }
    spec read_flag_false {
        aborts_if false;
        ensures result; // error: only the opaque specification function is known
    }

    fun read(key: address): bool {
        let entries = &Holder[@0x1].entries;
        contains(entries, key)
    }
    spec read {
        aborts_if !exists<Holder>(@0x1);
        ensures result == spec_contains(global<Holder>(@0x1).entries, key);
    }

    fun read_false(key: address): bool {
        let entries = &Holder[@0x1].entries;
        contains(entries, key)
    }
    spec read_false {
        aborts_if !exists<Holder>(@0x1);
        ensures result; // error: the key may be absent
    }

    fun write(key: address, value: u64) {
        let entries = &mut Holder[@0x1].entries;
        add(entries, key, value)
    }
    spec write {
        modifies global<Holder>(@0x1);
        aborts_if !exists<Holder>(@0x1);
        aborts_if spec_contains(global<Holder>(@0x1).entries, key);
        ensures spec_contains(global<Holder>(@0x1).entries, key);
    }

    fun publish(account: &signer) {
        move_to(account, Holder { entries: new() })
    }
    spec publish {
        modifies global<Holder>(signer::address_of(account));
        aborts_if exists<Holder>(signer::address_of(account));
        ensures exists<Holder>(signer::address_of(account));
    }

    fun publish_false(account: &signer) {
        move_to(account, Holder { entries: new() })
    }
    spec publish_false {
        modifies global<Holder>(signer::address_of(account));
        aborts_if false; // error: the address may hold one already
    }
}
