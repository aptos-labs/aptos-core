// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// The size of a map after an update of a key it holds and of one it does
/// not: the two facts the library states beside their parent
/// `SpecMap.size_update_sequence`, which the proof file applies by name.
/// The owner binds the executable role their fact is of, `add`.
module 0x1::map_size_update {
    struct Map<phantom K: copy + drop, phantom V> has store {
        handle: address,
    }
    spec Map {
        pragma intrinsic = map,
            map_add_override_if_exists = add,
            map_spec_len = spec_len,
            map_spec_has_key = spec_contains,
            map_spec_get = spec_get,
            map_spec_set = spec_set,
            map_spec_del = spec_remove;
    }

    native fun add<K: copy + drop, V>(self: &mut Map<K, V>, k: K, v: V);
    spec add {
        pragma intrinsic;
    }

    spec native fun spec_len<K, V>(t: Map<K, V>): num;
    spec native fun spec_contains<K, V>(t: Map<K, V>, k: K): bool;
    spec native fun spec_get<K, V>(t: Map<K, V>, k: K): V;
    spec native fun spec_set<K, V>(t: Map<K, V>, k: K, v: V): Map<K, V>;
    spec native fun spec_remove<K, V>(t: Map<K, V>, k: K): Map<K, V>;

    /// An entry of a key the map does not hold makes it one longer.
    fun size_after_insert(m: &mut Map<u8, u64>, k: u8, v: u64) {
        m.add(k, v);
        spec { assert spec_len(m) == spec_len(old(m)) + 1; };
    }
    spec size_after_insert {
        requires !spec_contains(m, k);
    }

    /// An entry of a key it holds leaves the size alone.
    fun size_after_override(m: &mut Map<u8, u64>, k: u8, v: u64) {
        m.add(k, v);
        spec { assert spec_len(m) == spec_len(old(m)); };
    }
    spec size_after_override {
        requires spec_contains(m, k);
    }

    /// Negative: of a key that may be held, neither size is stated.
    fun size_after_unknown(m: &mut Map<u8, u64>, k: u8, v: u64) {
        m.add(k, v);
        spec { assert spec_len(m) == spec_len(old(m)) + 1; }; // error: the key may be held
    }
}
