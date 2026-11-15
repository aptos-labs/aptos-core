// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// One spelling of a defined value: a clause's `exists c: u64: c == v && p(c)` reads as the
/// range of `v` and `p` at `v`, under the clause's binders too, and `let x : T where φ` whose
/// `φ` defines `x` names that value.
module 0x1::proof_defined_values {
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

    spec fun contained<K, V>(t: Table<K, V>, k: K): bool {
        spec_contains(t, k)
    }

    struct Slots has store {
        entries: Table<u64, u64>,
        size: u64,
    }
    spec Slots {
        invariant size != 0;
        invariant forall k in 0..spec_len(entries): contained(entries, k);
    }

    // The instance at `i / size` of the invariant's quantifier, under its binder.
    fun instance(self: &Slots, i: u64): u64 {
        i
    }
    spec instance {
        requires i / self.size < spec_len(self.entries);
        ensures contained(self.entries, i / self.size);
    }

    // The value named and read by later steps.
    fun named(self: &Slots, i: u64): u64 {
        i
    }
    spec named {
        requires i / self.size < spec_len(self.entries);
        requires spec_get(self.entries, i / self.size) == 7;
        ensures contained(self.entries, i / self.size);
    }
}
