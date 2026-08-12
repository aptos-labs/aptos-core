// Data invariants on an intrinsic map type are rejected: the map's validity
// predicate is template-defined and its pack/unpack sites are erased by the
// intrinsic representation, so the invariant would be silently inert —
// neither checked at creation/mutation nor assumed at use.
module 0x42::intrinsic_map_invariant_err {
    struct MapG<phantom K: copy + drop, phantom V> has store, drop {}
    spec MapG {
        pragma intrinsic = map,
            map_new = new;
        invariant 0 == 0; // error: even a trivial invariant is rejected
    }
    native fun new<K: copy + drop, V: store>(): MapG<K, V>;

    fun mk(): MapG<u64, u64> {
        new()
    }
}
