// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// A key a map lemma states is the key an assertion owes: the lemma's
/// instance reads outside the map by the owner's typed view of the KEYS.
/// `key_at_rank_narrow` holds values of another width than its keys and
/// `key_at_rank_generic` keys that are no integers, so a default taken
/// from the values or synthesized for an integer is another term.
module 0x1::map_key_default {
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
    // The intrinsic requires the value roles even where no cell reads one.
    spec native fun spec_get<K, V>(t: Map<K, V>, k: K): V;
    spec native fun spec_set<K, V>(t: Map<K, V>, k: K, v: V): Map<K, V>;
    spec native fun spec_remove<K, V>(t: Map<K, V>, k: K): Map<K, V>;
    spec native fun spec_key_at<K, V>(t: Map<K, V>, i: num): K;
    spec native fun spec_rank<K, V>(t: Map<K, V>, k: K): num;

    /// The key at a present key's position is the key.
    fun key_at_rank(m: &Map<u64, u64>, k: u64) {
        spec { assert spec_key_at(m, spec_rank(m, k)) == k; };
    }
    spec key_at_rank {
        requires spec_contains(m, k);
    }

    /// The same at a ground position.
    fun key_at_ground(m: &Map<u64, u64>) {
        spec { assert spec_key_at(m, 1) == 7; };
    }
    spec key_at_ground {
        requires spec_contains(m, 7);
        requires spec_rank(m, 7) == 1;
    }

    /// The same where the values are of another width than the keys.
    fun key_at_rank_narrow(m: &Map<u64, u8>, k: u64) {
        spec { assert spec_key_at(m, spec_rank(m, k)) == k; };
    }
    spec key_at_rank_narrow {
        requires spec_contains(m, k);
    }

    /// The same at an owner whose keys are a type parameter.
    fun key_at_rank_generic<K: copy + drop>(m: &Map<K, u64>, k: K) {
        spec { assert spec_key_at(m, spec_rank(m, k)) == k; };
    }
    spec key_at_rank_generic {
        requires spec_contains(m, k);
    }

    /// Negative: the lemma's fact is of another key.
    fun key_at_other(m: &Map<u64, u64>) {
        spec { assert spec_key_at(m, 1) == 8; }; // error: the key at position 1 is 7
    }
    spec key_at_other {
        requires spec_contains(m, 7);
        requires spec_rank(m, 7) == 1;
    }

    /// Negative: a read outside the map has no stated value.
    fun key_at_outside(m: &Map<u64, u64>) {
        spec { assert spec_key_at(m, spec_len(m)) == 0; }; // error: unspecified outside
    }

    /// Negative: an integer-keyed law at keys that are no integers is
    /// refused outright, and nothing is synthesized for it.
    fun key_at_lt_generic<K: copy + drop>(m: &Map<K, u64>) {
        spec { assert spec_rank(m, spec_key_at(m, 0)) == 0; }; // error: the lemma does not fit
    }
    spec key_at_lt_generic {
        requires spec_len(m) > 0;
    }
}
