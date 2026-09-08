// flag: --generate-only
module 0x42::map_has_key_non_aborting {
    use aptos_framework::big_ordered_map::{Self, BigOrderedMap};

    fun contains<K: copy + drop + store, V: store>(
        map: &BigOrderedMap<K, V>, key: &K
    ): bool {
        big_ordered_map::contains(map, key)
    }
    spec contains<K: copy + drop + store, V: store>(map: &0x1::big_ordered_map::BigOrderedMap<K, V>, key: &K): bool {
        use 0x1::big_ordered_map;
        pragma opaque = true;
        ensures [inferred] result == big_ordered_map::spec_contains_key<K, V>(map, key);
        aborts_if [inferred] false;
    }

}
/*
Verification: Succeeded.
*/
