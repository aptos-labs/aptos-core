// The enumeration roles have fixed signatures: the template emits
// `(map<K, V>, num): K` for `map_spec_key_at` and `(map<K, V>, K): num` for
// `map_spec_rank`, while user spec text calls the binding as declared. A
// deviating declaration would make the two disagree in emitted Boogie — and
// because several source types erase to the same Boogie type (an `address`
// key argument erases to `int`, like the ordinal index), the mismatch would
// otherwise be silent type confusion rather than an error. Both roles must
// also bind a `spec native fun`, since a bodied one would get a second
// Boogie function emitted under the same name.
module 0x42::intrinsic_map_rank_sig_err {
    struct Map<phantom K: copy + drop, phantom V> has store, drop {}

    spec Map {
        pragma intrinsic = map,
            map_new = new,
            map_spec_len = spec_len,
            map_spec_has_key = spec_contains,
            map_spec_key_at = spec_key_at,
            map_spec_rank = spec_rank;
    }

    public native fun new<K: copy + drop, V: store>(): Map<K, V>;

    spec native fun spec_len<K, V>(m: Map<K, V>): num;
    spec native fun spec_contains<K, V>(m: Map<K, V>, k: K): bool;
    // Wrong: the index must be `num`, and the result the key type.
    spec native fun spec_key_at<K, V>(m: Map<K, V>, k: K): num;
    // Wrong: bodied rather than `spec native fun`.
    spec fun spec_rank<K, V>(m: Map<K, V>, k: K): num {
        spec_len(m)
    }

    fun mk(): Map<u64, u64> {
        new()
    }
}
