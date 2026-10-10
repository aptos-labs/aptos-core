// The enumeration roles define each other — `map_spec_key_at`'s axioms are
// stated through `map_spec_rank` and vice versa — so the model emits both
// declarations or neither. Binding only one passes the per-role signature and
// `spec native fun` checks, and would then reach Boogie as a call to a
// function that was never declared, ending verification with a name
// resolution error instead of a diagnostic. Rejected at the declaration.
module 0x42::intrinsic_map_rank_pair_err {
    struct Map<phantom K: copy + drop, phantom V> has store, drop {}

    spec Map {
        pragma intrinsic = map,
            map_new = new,
            map_spec_len = spec_len,
            map_spec_has_key = spec_contains,
            map_spec_key_at = spec_key_at;
    }

    public native fun new<K: copy + drop, V: store>(): Map<K, V>;

    spec native fun spec_len<K, V>(m: Map<K, V>): num;
    spec native fun spec_contains<K, V>(m: Map<K, V>, k: K): bool;
    spec native fun spec_key_at<K, V>(m: Map<K, V>, i: num): K;

    fun use_it(_m: &Map<u64, u64>) {}
    spec use_it {
        requires spec_len(_m) > 0;
        ensures spec_contains(_m, spec_key_at(_m, 0));
    }

    fun mk(): Map<u64, u64> {
        new()
    }
}
