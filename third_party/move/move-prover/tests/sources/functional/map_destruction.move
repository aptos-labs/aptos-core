// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// The role declaration determines destruction semantics, for any map owner
// and independently of the names of the bound functions.
module 0x42::map_destruction {
    struct Map<phantom K: copy + drop, phantom V> has store {}
    spec Map {
        pragma intrinsic = map,
            map_new = fresh,
            map_add_no_override = insert,
            map_destroy_empty = checked,
            map_destroy_unchecked = discard,
            map_spec_get = spec_get,
            map_spec_set = spec_set,
            map_spec_del = spec_del,
            map_spec_has_key = spec_contains,
            map_spec_len = spec_len;
    }

    native fun fresh<K: copy + drop, V>(): Map<K, V>;
    native fun insert<K: copy + drop, V>(m: &mut Map<K, V>, k: K, v: V);
    native fun checked<K: copy + drop, V>(m: Map<K, V>);
    native fun discard<K: copy + drop, V>(m: Map<K, V>);

    spec native fun spec_get<K, V>(m: Map<K, V>, k: K): V;
    spec native fun spec_set<K, V>(m: Map<K, V>, k: K, v: V): Map<K, V>;
    spec native fun spec_del<K, V>(m: Map<K, V>, k: K): Map<K, V>;
    spec native fun spec_contains<K, V>(m: Map<K, V>, k: K): bool;
    spec native fun spec_len<K, V>(m: Map<K, V>): num;

    fun nonempty(): Map<u64, u64> {
        let m = fresh();
        insert(&mut m, 1, 7);
        m
    }
    spec nonempty {
        aborts_if false;
        ensures spec_len(result) == 1;
    }

    fun unchecked_returns(): u64 {
        discard(nonempty());
        7
    }
    spec unchecked_returns {
        aborts_if false;
        ensures result == 7;
    }

    fun unchecked_incorrect(): u64 {
        discard(nonempty());
        7
    }
    spec unchecked_incorrect {
        aborts_if false;
        ensures result == 8; // error: unchecked destruction returns normally, so this postcondition is false
    }

    fun checked_rejects() {
        checked(nonempty());
    }
    spec checked_rejects {
        aborts_if true;
    }

    fun checked_empty() {
        checked(fresh<u64, u64>());
    }
    spec checked_empty {
        aborts_if false;
    }

    fun unchecked_any<K: copy + drop, V>(m: Map<K, V>) {
        discard(m);
    }
    spec unchecked_any {
        aborts_if false;
    }
}
