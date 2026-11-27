// flag: --dependency=tests/sources/functional/ghost_field_map_keys_provider.move
// A map or table key type that can hold a ghost field is refused: the
// verification model keys a map by the keys' ghost fields as well, which the
// runtime ignores. In every module but the last, `G { v }` is one key at
// runtime whatever its ghost `g`; the refusal is at the key type, at an
// instantiation of a key type parameter, and through an imported function.

// Membership, overwrite, duplicate insertion, removal, and specification
// membership, each with two keys of equal runtime fields and different ghosts.
module 0x42::ghost_field_map_keys {
    use 0x42::ghost_field_map_keys_provider::{Self as maps, Map, spec_contains};

    struct G has copy, drop, store { v: u64 }
    spec G {
        ghost g: u64;
    }

    fun pair(): (G, G) {
        let a = G { v: 1 };
        let b = G { v: 1 };
        spec { update a.g = 1; };
        spec { update b.g = 2; };
        (a, b)
    }

    fun contains_neg(): bool {
        let (a, b) = pair();
        let m = maps::new<G, u64>(); // error: ghost-carrying key refused
        maps::add(&mut m, a, 0);
        maps::contains(&m, b)
    }
    spec contains_neg { ensures result == false; }

    fun set_len_neg(): u64 {
        let (a, b) = pair();
        let m = maps::new<G, u64>(); // error: ghost-carrying key refused
        maps::add(&mut m, a, 0);
        maps::set(&mut m, b, 1);
        maps::length(&m)
    }
    spec set_len_neg { ensures result == 2; }

    fun add_twice_neg(): u64 {
        let (a, b) = pair();
        let m = maps::new<G, u64>(); // error: ghost-carrying key refused
        maps::add(&mut m, a, 0);
        maps::add(&mut m, b, 1);
        maps::length(&m)
    }
    spec add_twice_neg { aborts_if false; ensures result == 2; }

    fun remove_equivalent_neg(): u64 {
        let (a, b) = pair();
        let m = maps::new<G, u64>(); // error: ghost-carrying key refused
        maps::add(&mut m, a, 0);
        maps::remove(&mut m, b);
        maps::length(&m)
    }
    spec remove_equivalent_neg { ensures result == 1; }

    fun spec_contains_neg(a: G, b: G): Map<G, u64> { // error: ghost-carrying key refused
        let m = maps::new<G, u64>();
        maps::add(&mut m, a, 0);
        m
    }
    spec spec_contains_neg {
        requires a.v == b.v && a.g != b.g;
        ensures !spec_contains(result, b);
    }

    // A key type parameter instantiated where no map type of `G` appears.
    fun make<T: copy + drop>(): u64 {
        let m = maps::new<T, u64>();
        maps::length(&m)
    }

    fun make_neg(): u64 {
        make<G>() // error: ghost-carrying key refused at the instantiation
    }

    // A function of an imported module keying a map by its type parameter in
    // its body alone.
    fun imported_neg(a: G): u64 {
        maps::count(a) // error: ghost-carrying key refused at the instantiation
    }
}

// A specification function reading a map keyed by `G`.
module 0x42::ghost_field_map_keys_spec {
    use 0x42::ghost_field_map_keys_provider::{Map, spec_contains};

    struct G has copy, drop, store { v: u64 }
    spec G {
        ghost g: u64;
    }

    spec fun holds(m: Map<G, u64>, b: G): bool { // error: ghost-carrying key refused
        spec_contains(m, b)
    }
}

// Declarations of the imported module with a ghost field, or with a key demand
// in a data invariant, used here as keys.
module 0x42::ghost_field_map_keys_imported {
    use 0x42::ghost_field_map_keys_provider::{Self as maps, PG, PE, Holder};

    struct H has copy, drop, store { v: u64 }
    spec H {
        ghost g: u64;
    }

    fun imported_struct(): u64 {
        let m = maps::new<PG, u64>(); // error: ghost-carrying key refused
        maps::length(&m)
    }

    fun imported_enum(): u64 {
        let m = maps::new<PE, u64>(); // error: ghost-carrying key refused
        maps::length(&m)
    }

    fun holder(h: Holder<H>): Holder<H> { // error: ghost-carrying key refused
        h
    }
}

// A map whose values hold a ghost field, itself a key: directly, in a struct,
// and through a type parameter.
module 0x42::ghost_field_map_keys_nested {
    use 0x42::ghost_field_map_keys_provider::{Self as maps, Map};

    struct G has copy, drop, store { v: u64 }
    spec G {
        ghost g: u64;
    }

    struct Inner has copy, drop, store { inner: Map<u64, G> }

    fun nested(): u64 {
        let m = maps::new<Map<u64, G>, u64>(); // error: ghost-carrying key refused
        maps::length(&m)
    }

    fun inner(): u64 {
        let m = maps::new<Inner, u64>(); // error: ghost-carrying key refused
        maps::length(&m)
    }

    fun generic<V: store>(): u64 {
        let m = maps::new<Map<u64, V>, u64>();
        maps::length(&m)
    }

    fun generic_ghost(): u64 {
        generic<G>() // error: ghost-carrying key refused at the instantiation
    }
}

// `extensions::table` keyed by a struct with a ghost field.
module 0x42::ghost_field_map_keys_table {
    use extensions::table::{Self, Table};
    use extensions::table::{spec_contains, spec_len};

    struct G has copy, drop, store { v: u64 }
    spec G {
        ghost g: u64;
    }

    fun pair(): (G, G) {
        let a = G { v: 1 };
        let b = G { v: 1 };
        spec { update a.g = 1; };
        spec { update b.g = 2; };
        (a, b)
    }

    fun contains_neg(): (bool, Table<G, u64>) { // error: ghost-carrying key refused
        let (a, b) = pair();
        let t = table::new<G, u64>();
        table::add(&mut t, a, 0);
        let r = table::contains(&t, b);
        (r, t)
    }
    spec contains_neg { ensures result_1 == false; }

    fun spec_contains_neg(a: G, b: G): Table<G, u64> { // error: ghost-carrying key refused
        let t = table::new<G, u64>();
        table::add(&mut t, a, 0);
        t
    }
    spec spec_contains_neg {
        requires a.v == b.v && a.g != b.g;
        ensures !spec_contains(result, b);
    }

    fun upsert_len_neg(a: G, b: G): Table<G, u64> { // error: ghost-carrying key refused
        let t = table::new<G, u64>();
        table::add(&mut t, a, 0);
        *table::borrow_mut_with_default(&mut t, b, 1) = 2;
        t
    }
    spec upsert_len_neg {
        requires a.v == b.v && a.g != b.g;
        ensures spec_len(result) == 2;
    }
}

// Keys that hold no ghost field keep verifying: a struct, a scalar, a vector
// of scalars, a map whose values (not keys) hold a ghost field, and the
// generic functions above at a struct without a ghost field.
module 0x42::ghost_field_map_keys_ok {
    use 0x42::ghost_field_map_keys_provider::{Self as maps, Map, spec_contains};

    struct H has copy, drop, store { v: u64 }

    struct G has copy, drop, store { v: u64 }
    spec G {
        ghost g: u64;
    }

    fun struct_key(): bool {
        let m = maps::new<H, u64>();
        maps::add(&mut m, H { v: 1 }, 0);
        maps::contains(&m, H { v: 1 })
    }
    spec struct_key { ensures result == true; }

    fun struct_keys_distinct(): (u64, bool, bool) {
        let m = maps::new<H, u64>();
        maps::add(&mut m, H { v: 1 }, 0);
        maps::add(&mut m, H { v: 2 }, 0);
        (maps::length(&m), maps::contains(&m, H { v: 1 }), maps::contains(&m, H { v: 2 }))
    }
    spec struct_keys_distinct { ensures result_1 == 2 && result_2 && result_3; }

    fun scalar_key(): bool {
        let m = maps::new<u64, u64>();
        maps::add(&mut m, 1, 0);
        maps::contains(&m, 1)
    }
    spec scalar_key { ensures result == true; }

    fun vector_key(): bool {
        let m = maps::new<vector<u8>, u64>();
        maps::add(&mut m, b"ab", 0);
        maps::contains(&m, b"ab")
    }
    spec vector_key { ensures result == true; }

    fun ghost_value(x: G): u64 {
        let m = maps::new<u64, G>();
        maps::add(&mut m, 1, x);
        maps::length(&m)
    }
    spec ghost_value { ensures result == 1; }

    fun spec_member(a: H): Map<H, u64> {
        let m = maps::new<H, u64>();
        maps::add(&mut m, a, 0);
        m
    }
    spec spec_member { ensures spec_contains(result, a); }

    fun make<T: copy + drop>(): u64 {
        let m = maps::new<T, u64>();
        maps::length(&m)
    }
    spec make { ensures result == 0; }

    fun make_struct(): u64 {
        make<H>()
    }
    spec make_struct { ensures result == 0; }

    fun imported_struct(): u64 {
        maps::count(H { v: 1 })
    }
    spec imported_struct { ensures result == 1; }

    struct Tag<phantom T> has copy, drop, store { v: u64 }

    fun ghost_map_value(): u64 {
        let m = maps::new<u64, Map<u64, G>>();
        maps::length(&m)
    }
    spec ghost_map_value { ensures result == 0; }

    fun phantom_payload(): u64 {
        let m = maps::new<Tag<G>, u64>();
        maps::add(&mut m, Tag { v: 1 }, 0);
        maps::length(&m)
    }
    spec phantom_payload { ensures result == 1; }
}
