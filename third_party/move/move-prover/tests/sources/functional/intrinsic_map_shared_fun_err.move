// A function bound to one intrinsic type cannot be bound to another:
// template definitions are emitted once per type under the bound function's
// name, so sharing would declare the same Boogie symbol twice.
module 0x42::intrinsic_map_shared_spec_fun {
    struct MapA<phantom K: copy + drop, phantom V> has store, drop {}
    struct MapB<phantom K: copy + drop, phantom V> has store, drop {}
    spec MapA {
        pragma intrinsic = map, map_new = new_a, map_spec_len = shared_len;
    }
    spec MapB {
        pragma intrinsic = map, map_new = new_b,
            map_spec_len = shared_len; // error: already bound to MapA
    }
    native fun new_a<K: copy + drop, V: store>(): MapA<K, V>;
    native fun new_b<K: copy + drop, V: store>(): MapB<K, V>;
    spec native fun shared_len<K, V>(t: MapA<K, V>): num;
    fun touch(): (MapA<u64, u64>, MapB<u64, u64>) {
        (new_a<u64, u64>(), new_b<u64, u64>())
    }
}

module 0x42::intrinsic_map_shared_move_fun {
    struct MapA<phantom K: copy + drop, phantom V> has store, drop {}
    struct MapB<phantom K: copy + drop, phantom V> has store, drop {}
    spec MapA {
        pragma intrinsic = map, map_len = shared_length;
    }
    spec MapB {
        pragma intrinsic = map,
            map_len = shared_length; // error: already bound to MapA
    }
    native fun shared_length<K: copy + drop, V>(m: &MapA<K, V>): u64;
    fun touch(m: &MapA<u64, u64>): u64 {
        shared_length(m)
    }
}
