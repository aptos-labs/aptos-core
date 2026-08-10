// An intrinsic map binding a value-MUTATING role without `map_spec_get` must
// produce a diagnostic: stored values are reachable through the model's
// write-back, but the deep data invariant over them cannot be stated —
// silently skipping would drop the check and admit a false proof (a mutation
// to an invariant-violating value would verify).
module 0x42::intrinsic_map_mut_no_get_err {
    struct W has copy, drop, store { v: u64 }
    spec W {
        invariant v != 0;
    }

    struct Map<phantom K: copy + drop, phantom V> has store, drop {}
    spec Map {
        pragma intrinsic = map,
            map_new = new,
            map_borrow_mut = bmut; // error: mutating role without map_spec_get
    }
    native fun new<K: copy + drop, V: store>(): Map<K, V>;
    native fun bmut<K: copy + drop, V>(m: &mut Map<K, V>, k: K): &mut V;

    fun poke(m: &mut Map<u64, W>) {
        let w = bmut(m, 1);
        w.v = 0;
    }
}
