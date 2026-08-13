// Equality of intrinsic maps over ghost-bearing value types: Move equality
// is the quotient over runtime state, so two maps with identical runtime
// content are equal regardless of the ghosts carried by the stored values.
// The table template compares stored values with `$IsEqual` (ghost-excluding)
// whenever the value type transitively carries a ghost field; ghost-less
// value types keep the raw comparison byte-for-byte.
module 0x42::ghost_field_map_equality {
    struct V has copy, drop, store { x: u64 }
    spec V {
        ghost g: u64;
    }

    struct Map<phantom K: copy + drop, phantom VV> has store, drop {}
    spec Map {
        pragma intrinsic = map,
            map_new = new,
            map_add_no_override = add,
            map_spec_len = spec_len,
            map_spec_has_key = spec_contains;
    }
    native fun new<K: copy + drop, VV: store>(): Map<K, VV>;
    native fun add<K: copy + drop, VV>(m: &mut Map<K, VV>, key: K, val: VV);
    spec native fun spec_len<K, VV>(m: Map<K, VV>): num;
    spec native fun spec_contains<K, VV>(m: Map<K, VV>, k: K): bool;

    // Identical runtime content, deliberately diverged ghosts on the stored
    // values: the maps are Move-equal.
    fun mk_diverged(): (Map<u64, V>, Map<u64, V>) {
        let v1 = V { x: 7 };
        let v2 = V { x: 7 };
        spec {
            update v1.g = 1;
            update v2.g = 2;
        };
        let m1 = new<u64, V>();
        let m2 = new<u64, V>();
        add(&mut m1, 1, v1);
        add(&mut m2, 1, v2);
        (m1, m2)
    }
    spec mk_diverged {
        ensures result_1 == result_2;
    }

    // The ghost-divergent maps must not be provably UNEQUAL either. FAILS.
    fun mk_diverged_neq(): (Map<u64, V>, Map<u64, V>) {
        let v1 = V { x: 7 };
        let v2 = V { x: 7 };
        spec {
            update v1.g = 1;
            update v2.g = 2;
        };
        let m1 = new<u64, V>();
        let m2 = new<u64, V>();
        add(&mut m1, 1, v1);
        add(&mut m2, 1, v2);
        (m1, m2)
    }
    spec mk_diverged_neq {
        ensures result_1 != result_2; // FAILS: they are Move-equal
    }

    // Runtime-divergent maps are unequal regardless of ghosts.
    fun mk_runtime_diverged_neq(): (Map<u64, V>, Map<u64, V>) {
        let m1 = new<u64, V>();
        let m2 = new<u64, V>();
        add(&mut m1, 1, V { x: 7 });
        add(&mut m2, 1, V { x: 8 });
        (m1, m2)
    }
    spec mk_runtime_diverged_neq {
        ensures result_1 != result_2;
    }
}
