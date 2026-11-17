// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// A value a map lemma states is the value an assertion owes: the lemma's
/// instance reads outside the map by the owner's typed view of the VALUES.
/// The keys are bytes and the values words, which both read as integers,
/// so a default taken from the keys is another term. The two executable
/// roles are what a lemma of an updated or a removed map needs: its fact
/// is of the map after the operation, which the owner's own `add` and
/// `erase` give it.
module 0x1::map_value_default {
    struct Map<phantom K: copy + drop, phantom V> has store {
        handle: address,
    }
    spec Map {
        pragma intrinsic = map,
            map_add_override_if_exists = add,
            map_del_must_exist = erase,
            map_spec_has_key = spec_contains,
            map_spec_get = spec_get,
            map_spec_set = spec_set,
            map_spec_del = spec_remove;
    }

    native fun add<K: copy + drop, V>(self: &mut Map<K, V>, k: K, v: V);
    spec add {
        pragma intrinsic;
    }

    native fun erase<K: copy + drop, V>(self: &mut Map<K, V>, k: K): V;
    spec erase {
        pragma intrinsic;
    }

    spec native fun spec_contains<K, V>(t: Map<K, V>, k: K): bool;
    spec native fun spec_get<K, V>(t: Map<K, V>, k: K): V;
    spec native fun spec_set<K, V>(t: Map<K, V>, k: K, v: V): Map<K, V>;
    spec native fun spec_remove<K, V>(t: Map<K, V>, k: K): Map<K, V>;

    /// The value at an updated key is the value.
    fun value_at_set(m: &mut Map<u8, u64>, k: u8, v: u64) {
        m.add(k, v);
        spec { assert spec_get(m, k) == v; };
    }

    /// Another key keeps its value when a key is removed.
    fun value_at_erase(m: &mut Map<u8, u64>, k: u8, p: u8) {
        let _dropped = m.erase(k);
        spec { assert spec_get(m, p) == spec_get(old(m), p); };
    }
    spec value_at_erase {
        requires p != k;
    }

    /// Negative: the value of a key without an entry has none stated.
    fun value_at_missing(m: &Map<u8, u64>, k: u8) {
        spec { assert spec_get(m, k) == 0; }; // error: unspecified without an entry
    }
    spec value_at_missing {
        requires !spec_contains(m, k);
    }
}
