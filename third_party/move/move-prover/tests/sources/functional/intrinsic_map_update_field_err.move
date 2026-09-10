// Functional `update_field` on an intrinsic map's field is the
// expression-form twin of the rejected statement `update`: declared fields
// are erased by the intrinsic representation, so no update function exists
// in the emitted Boogie. Rejected with a diagnostic at translation instead
// of emitting an undefined symbol.
module 0x42::intrinsic_map_update_field_err {
    struct MapD<phantom K: copy + drop, phantom V> has store, drop { dummy: bool }
    spec MapD {
        pragma intrinsic = map,
            map_new = new;
    }
    native fun new<K: copy + drop, V: store>(): MapD<K, V>;

    fun mk(): MapD<u64, u64> {
        new()
    }
    spec mk {
        // error: fields of an intrinsic map cannot be updated in specs
        ensures result == update_field(result, dummy, false);
    }
}
