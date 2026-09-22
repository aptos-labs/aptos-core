// flag: --generate-only
module 0x42::map_has_key_non_aborting {
    use aptos_framework::big_ordered_map::{Self, BigOrderedMap};

    fun contains<K: copy + drop + store, V: store>(
        map: &BigOrderedMap<K, V>, key: &K
    ): bool {
        big_ordered_map::contains(map, key)
    }
}
