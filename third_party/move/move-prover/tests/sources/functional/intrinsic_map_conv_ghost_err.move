// A `map_to_ordered_map` binding whose destination map carries a hidden
// validity slot: the conversion template returns the destination as a raw
// table, but a slot-carrying destination is represented by a carrier
// datatype, so emitted calls would be ill-typed Boogie. The binding is
// rejected with a diagnostic instead.
module 0x42::intrinsic_map_conv_ghost_err {
    struct Dest<phantom K: copy + drop, phantom V> has store, drop {}
    spec Dest {
        pragma intrinsic = map,
            map_new = dest_new,
            map_spec_iter_preserved = dest_preserved;
    }
    native fun dest_new<K: copy + drop, V: store>(): Dest<K, V>;
    spec native fun dest_preserved<K: copy + drop, V>(m1: Dest<K, V>, m2: Dest<K, V>): bool;

    struct Src<phantom K: copy + drop, phantom V> has store, drop {}
    spec Src {
        pragma intrinsic = map,
            map_new = src_new,
            map_to_ordered_map = to_dest;
    }
    native fun src_new<K: copy + drop, V: store>(): Src<K, V>;
    native fun to_dest<K: copy + drop, V>(m: Src<K, V>): Dest<K, V>;

    fun convert(m: Src<u64, u64>): Dest<u64, u64> {
        to_dest(m)
    }
}
