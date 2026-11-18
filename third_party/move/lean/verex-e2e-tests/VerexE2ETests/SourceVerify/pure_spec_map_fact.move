// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// At a map owner with no executable operations, the length-update lemma and
/// a precondition equating a parameter with an updated map do not close the
/// authored assertions of `v_len_set` and `v_set_param`, which are parked
/// gaps; `v_rank_nonneg` is the control. The cause is not localized.
module 0x1::pure_spec_map_fact {
    struct Map<phantom K: copy + drop, phantom V> has store {
        handle: address,
    }
    spec Map {
        pragma intrinsic = map,
            map_spec_len = spec_len,
            map_spec_has_key = spec_contains,
            map_spec_get = spec_get,
            map_spec_set = spec_set,
            map_spec_del = spec_remove,
            map_spec_key_at = spec_key_at,
            map_spec_rank = spec_rank;
    }

    spec native fun spec_len<K, V>(t: Map<K, V>): num;
    spec native fun spec_contains<K, V>(t: Map<K, V>, k: K): bool;
    spec native fun spec_get<K, V>(t: Map<K, V>, k: K): V;
    spec native fun spec_set<K, V>(t: Map<K, V>, k: K, v: V): Map<K, V>;
    spec native fun spec_remove<K, V>(t: Map<K, V>, k: K): Map<K, V>;
    spec native fun spec_key_at<K, V>(t: Map<K, V>, i: num): K;
    spec native fun spec_rank<K, V>(t: Map<K, V>, k: K): num;

    /// The size of an updated map, by a lemma with no default hole.
    fun v_len_set(m: &Map<u64, u64>, k: u64, v: u64) {
        spec {
            assert spec_len(spec_set(m, k, v)) >= spec_len(m);
        };
    }

    /// The updated map as a parameter, equated by a precondition.
    fun v_set_param(m: &Map<u8, u64>, n: &Map<u8, u64>, k: u8, v: u64) {
        spec {
            assert spec_get(n, k) == v;
        };
    }
    spec v_set_param {
        requires n == spec_set(m, k, v);
    }

    /// Control: a lemma of the parameter map itself.
    fun v_rank_nonneg(m: &Map<u64, u64>, k: u64) {
        spec {
            assert spec_rank(m, k) >= 0;
        };
    }
}
