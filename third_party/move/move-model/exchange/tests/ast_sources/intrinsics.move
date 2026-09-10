module 0x1::intrinsics {
    struct Map<phantom K, phantom V> {}

    native fun new<K, V>(): Map<K, V>;
    native fun destroy_empty<K, V>(map: Map<K, V>);
    native fun borrow_mut<K, V>(map: &mut Map<K, V>, key: K): &mut V;
    native fun length<K, V>(map: &Map<K, V>): u64;

    spec native fun spec_len<K, V>(map: Map<K, V>): num;
    spec native fun spec_set<K, V>(map: Map<K, V>, key: K, value: V): Map<K, V>;
    spec native fun spec_get<K, V>(map: Map<K, V>, key: K): V;

    spec Map {
        pragma intrinsic = map,
            map_new = new,
            map_destroy_empty = destroy_empty,
            map_borrow_mut = borrow_mut,
            map_len = length,
            map_spec_len = spec_len,
            map_spec_get = spec_get,
            map_spec_set = spec_set;
    }
}
