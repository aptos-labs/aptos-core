// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A read of a map after a write, asserted under a block that takes one clause
// (`on assert 1 { }`): `decide` does not close it, and `write_read` is a parked
// gap. `write_read_rest` is the control: the same body under `on _ { }` verifies.
module 0x1::map_write_block {
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

    fun write_read(m: &mut Map<u64, u64>) {
        m.add(1, 30);
        spec {
            assert spec_get(m, 1) == 30;
            assert spec_contains(m, 1);
        };
    }

    fun write_read_rest(m: &mut Map<u64, u64>) {
        m.add(1, 30);
        spec {
            assert spec_get(m, 1) == 30;
            assert spec_contains(m, 1);
        };
    }
}
