// flag: --vector-theory=SmtArrayExt
// Wrapper equality over intrinsic maps with ghost-bearing VALUE types, under
// an extensional theory (native equality enabled). The transitive ghost
// detection must see through the intrinsic map's type instantiation — an
// intrinsic map carries its content in the instantiation, not in declared
// fields — otherwise the wrapper is compared with raw datatype equality,
// which includes the stored values' ghost constructor arguments.
//
// SmtArrayExt is chosen deliberately: it is the only extensional theory that
// treats these traces honestly. Under BoogieArrayIntern and SmtSeq the
// function bodies below are vacuous (`ensures false` verifies), so those
// theories cannot pin this bug class.
module 0x42::ghost_field_map_equality_ext {
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

    struct Wrap has drop { m: Map<u64, V> }

    // Identical runtime content, deliberately diverged ghosts on the stored
    // values: the wrappers are Move-equal.
    fun mk_wrapped(): (Wrap, Wrap) {
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
        (Wrap { m: m1 }, Wrap { m: m2 })
    }
    spec mk_wrapped {
        ensures result_1 == result_2;
    }

    // The ghost-divergent wrappers must not be provably UNEQUAL. FAILS.
    fun mk_wrapped_neq(): (Wrap, Wrap) {
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
        (Wrap { m: m1 }, Wrap { m: m2 })
    }
    spec mk_wrapped_neq {
        ensures result_1 != result_2; // FAILS: they are Move-equal
    }

    // vector<Map<u64, V>> classifies the ELEMENT type directly, not through a
    // wrapper's declared-field walk: the ghost detection must apply its
    // intrinsic-instantiation case to the map itself, or the vector falls
    // back to raw element equality including stored-value ghosts.
    fun mk_vec(): (vector<Map<u64, V>>, vector<Map<u64, V>>) {
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
        (vector[m1], vector[m2])
    }
    spec mk_vec {
        ensures result_1 == result_2;
    }

    // The ghost-divergent element vectors must not be provably UNEQUAL —
    // pre-fix this VERIFIED (raw datatype equality distinguished the ghost
    // constructor arguments), certifying a runtime-false fact. FAILS.
    fun mk_vec_neq(): (vector<Map<u64, V>>, vector<Map<u64, V>>) {
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
        (vector[m1], vector[m2])
    }
    spec mk_vec_neq {
        ensures result_1 != result_2; // FAILS: they are Move-equal
    }
}
