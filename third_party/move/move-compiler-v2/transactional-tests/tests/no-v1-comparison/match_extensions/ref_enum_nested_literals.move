//# publish
module 0xc0ffee::m {
    enum E has drop, copy { V1(u64), V2 }
    enum Pair has drop, copy { P(u64, u64), Q }
    enum Inner has drop, copy { A(u64), B }
    enum Outer has drop, copy { W(Inner), X }
    enum Wrapper has drop, copy { S(vector<u8>), T }
    enum Generic<T> has drop, copy { V1(T), V2 }
    enum GenericPair<T, U> has drop, copy { P(T, U), Q }

    // --- Integer literal inside &Enum ---
    public fun test_ref_enum_hit(): u64 {
        let e = E::V1(1);
        match (&e) { E::V1(1) => 1, E::V1(_) => 2, E::V2 => 3 }
    }
    public fun test_ref_enum_miss(): u64 {
        let e = E::V1(42);
        match (&e) { E::V1(1) => 1, E::V1(_) => 2, E::V2 => 3 }
    }
    public fun test_ref_enum_other_variant(): u64 {
        let e = E::V2;
        match (&e) { E::V1(1) => 1, E::V1(_) => 2, E::V2 => 3 }
    }

    // --- Multiple literals in &Pair ---
    public fun test_ref_multi_hit(): u64 {
        let p = Pair::P(1, 2);
        match (&p) { Pair::P(1, 2) => 10, Pair::P(_, _) => 20, Pair::Q => 30 }
    }
    public fun test_ref_multi_partial(): u64 {
        let p = Pair::P(1, 99);
        match (&p) { Pair::P(1, 2) => 10, Pair::P(_, _) => 20, Pair::Q => 30 }
    }

    // --- Variable + literal mix with deref in body ---
    public fun test_ref_mix_var(): u64 {
        let p = Pair::P(5, 2);
        match (&p) { Pair::P(x, 2) => *x + 100, Pair::P(_, _) => 20, Pair::Q => 30 }
    }
    public fun test_ref_mix_var_miss(): u64 {
        let p = Pair::P(5, 99);
        match (&p) { Pair::P(x, 2) => *x + 100, Pair::P(_, _) => 20, Pair::Q => 30 }
    }

    // --- Literal + guard ---
    public fun test_ref_guard_pass(): u64 {
        let e = E::V1(1);
        match (&e) { E::V1(1) if true => 50, E::V1(_) => 60, E::V2 => 70 }
    }
    public fun test_ref_guard_fail(): u64 {
        let e = E::V1(1);
        match (&e) { E::V1(1) if false => 50, E::V1(_) => 60, E::V2 => 70 }
    }

    // --- Nested enum through reference ---
    public fun test_ref_nested_hit(): u64 {
        let o = Outer::W(Inner::A(1));
        match (&o) {
            Outer::W(Inner::A(1)) => 100,
            Outer::W(Inner::A(_)) => 200,
            Outer::W(Inner::B) => 300,
            Outer::X => 400,
        }
    }
    public fun test_ref_nested_miss(): u64 {
        let o = Outer::W(Inner::A(42));
        match (&o) {
            Outer::W(Inner::A(1)) => 100,
            Outer::W(Inner::A(_)) => 200,
            Outer::W(Inner::B) => 300,
            Outer::X => 400,
        }
    }

    // --- &mut reference ---
    public fun test_ref_mut_hit(): u64 {
        let e = E::V1(1);
        match (&mut e) { E::V1(1) => 1, E::V1(_) => 2, E::V2 => 3 }
    }
    public fun test_ref_mut_miss(): u64 {
        let e = E::V1(42);
        match (&mut e) { E::V1(1) => 1, E::V1(_) => 2, E::V2 => 3 }
    }

    // --- Byte string inside &Enum ---
    public fun test_ref_bytestring_hit(): u64 {
        let w = Wrapper::S(b"hello");
        match (&w) { Wrapper::S(b"hello") => 1, Wrapper::S(_) => 2, Wrapper::T => 3 }
    }
    public fun test_ref_bytestring_miss(): u64 {
        let w = Wrapper::S(b"world");
        match (&w) { Wrapper::S(b"hello") => 1, Wrapper::S(_) => 2, Wrapper::T => 3 }
    }

    // --- Top-level match (&x) { 1 => .. } ---
    public fun test_ref_top_level_hit(): u64 {
        let x: u64 = 1;
        match (&x) { 1 => 10, 2 => 20, _ => 0 }
    }
    public fun test_ref_top_level_miss(): u64 {
        let x: u64 = 99;
        match (&x) { 1 => 10, 2 => 20, _ => 0 }
    }

    // --- Mixed tuple (&E, u64) ---
    public fun test_mixed_ref_enum_hit(): u64 {
        let e = E::V1(1);
        match ((&e, 2u64)) {
            (E::V1(1), 2) => 10,
            (E::V1(_), _) => 20,
            (E::V2, _) => 30,
        }
    }
    public fun test_mixed_ref_enum_miss(): u64 {
        let e = E::V1(42);
        match ((&e, 2u64)) {
            (E::V1(1), 2) => 10,
            (E::V1(_), _) => 20,
            (E::V2, _) => 30,
        }
    }

    // --- Mixed tuple (&u64, E) ---
    public fun test_mixed_ref_prim_hit(): u64 {
        let x: u64 = 1;
        match ((&x, E::V1(2))) {
            (1, E::V1(2)) => 10,
            (_, E::V1(y)) => y,
            (_, E::V2) => 99,
        }
    }
    public fun test_mixed_ref_prim_miss(): u64 {
        let x: u64 = 99;
        match ((&x, E::V1(2))) {
            (1, E::V1(2)) => 10,
            (_, E::V1(y)) => y,
            (_, E::V2) => 99,
        }
    }

    // --- All-reference tuple (fully-transformable path) ---
    // Both tuple elements are &u64, so the entire match is fully transformable.
    public fun test_all_ref_tuple_hit(): u64 {
        let a: u64 = 1;
        let b: u64 = 2;
        match ((&a, &b)) { (1, 2) => 10, (1, _) => 20, (_, _) => 30 }
    }
    public fun test_all_ref_tuple_partial(): u64 {
        let a: u64 = 1;
        let b: u64 = 99;
        match ((&a, &b)) { (1, 2) => 10, (1, _) => 20, (_, _) => 30 }
    }
    public fun test_all_ref_tuple_miss(): u64 {
        let a: u64 = 42;
        let b: u64 = 2;
        match ((&a, &b)) { (1, 2) => 10, (1, _) => 20, (_, _) => 30 }
    }

    // --- Variable binding + literal in all-reference tuple ---
    // x binds to &u64 element, body uses *x.
    public fun test_all_ref_tuple_var_hit(): u64 {
        let a: u64 = 5;
        let b: u64 = 2;
        match ((&a, &b)) { (x, 2) => *x + 100, (_, _) => 0 }
    }
    public fun test_all_ref_tuple_var_miss(): u64 {
        let a: u64 = 5;
        let b: u64 = 99;
        match ((&a, &b)) { (x, 2) => *x + 100, (_, _) => 0 }
    }

    // --- Top-level match (&bool) { true => .. } ---
    public fun test_ref_bool_hit(): u64 {
        let b = true;
        match (&b) { true => 1, false => 0 }
    }
    public fun test_ref_bool_miss(): u64 {
        let b = false;
        match (&b) { true => 1, false => 0 }
    }

    // --- Top-level match (&vector<u8>) { b"hi" => .. } ---
    public fun test_ref_bytes_hit(): u64 {
        let bs = b"hi";
        match (&bs) { b"hi" => 1, _ => 0 }
    }
    public fun test_ref_bytes_miss(): u64 {
        let bs = b"bye";
        match (&bs) { b"hi" => 1, _ => 0 }
    }

    // --- Generic enum with literal through reference ---
    public fun test_ref_generic_hit(): u64 {
        let g = Generic::V1(1u64);
        match (&g) { Generic::V1(1) => 10, Generic::V1(_) => 20, Generic::V2 => 30 }
    }
    public fun test_ref_generic_miss(): u64 {
        let g = Generic::V1(42u64);
        match (&g) { Generic::V1(1) => 10, Generic::V1(_) => 20, Generic::V2 => 30 }
    }
    public fun test_ref_generic_other_variant(): u64 {
        let g: Generic<u64> = Generic::V2;
        match (&g) { Generic::V1(1) => 10, Generic::V1(_) => 20, Generic::V2 => 30 }
    }

    // --- Generic enum with multiple type params, literals in both fields ---
    public fun test_ref_generic_pair_hit(): u64 {
        let p = GenericPair::P(1u64, true);
        match (&p) { GenericPair::P(1, true) => 10, GenericPair::P(_, _) => 20, GenericPair::Q => 30 }
    }
    public fun test_ref_generic_pair_partial(): u64 {
        let p = GenericPair::P(1u64, false);
        match (&p) { GenericPair::P(1, true) => 10, GenericPair::P(_, _) => 20, GenericPair::Q => 30 }
    }

    // --- Generic enum with variable + literal mix through reference ---
    public fun test_ref_generic_var_lit(): u64 {
        let g = Generic::V1(5u64);
        match (&g) { Generic::V1(5) => 100, Generic::V1(x) => *x, Generic::V2 => 0 }
    }
    public fun test_ref_generic_var_lit_fallthrough(): u64 {
        let g = Generic::V1(99u64);
        match (&g) { Generic::V1(5) => 100, Generic::V1(x) => *x, Generic::V2 => 0 }
    }

    // --- Reference enum without literals (regression guard) ---
    // No literals at all — verifies existing ref enum matching is unchanged.
    public fun test_ref_enum_no_literal(): u64 {
        let e = E::V1(42);
        match (&e) { E::V1(x) => *x, E::V2 => 0 }
    }
    public fun test_ref_enum_no_literal_v2(): u64 {
        let e = E::V2;
        match (&e) { E::V1(x) => *x, E::V2 => 0 }
    }
}

//# run 0xc0ffee::m::test_ref_enum_hit

//# run 0xc0ffee::m::test_ref_enum_miss

//# run 0xc0ffee::m::test_ref_enum_other_variant

//# run 0xc0ffee::m::test_ref_multi_hit

//# run 0xc0ffee::m::test_ref_multi_partial

//# run 0xc0ffee::m::test_ref_mix_var

//# run 0xc0ffee::m::test_ref_mix_var_miss

//# run 0xc0ffee::m::test_ref_guard_pass

//# run 0xc0ffee::m::test_ref_guard_fail

//# run 0xc0ffee::m::test_ref_nested_hit

//# run 0xc0ffee::m::test_ref_nested_miss

//# run 0xc0ffee::m::test_ref_mut_hit

//# run 0xc0ffee::m::test_ref_mut_miss

//# run 0xc0ffee::m::test_ref_bytestring_hit

//# run 0xc0ffee::m::test_ref_bytestring_miss

//# run 0xc0ffee::m::test_ref_top_level_hit

//# run 0xc0ffee::m::test_ref_top_level_miss

//# run 0xc0ffee::m::test_mixed_ref_enum_hit

//# run 0xc0ffee::m::test_mixed_ref_enum_miss

//# run 0xc0ffee::m::test_mixed_ref_prim_hit

//# run 0xc0ffee::m::test_mixed_ref_prim_miss

//# run 0xc0ffee::m::test_ref_bool_hit

//# run 0xc0ffee::m::test_ref_bool_miss

//# run 0xc0ffee::m::test_ref_bytes_hit

//# run 0xc0ffee::m::test_ref_bytes_miss

//# run 0xc0ffee::m::test_all_ref_tuple_hit

//# run 0xc0ffee::m::test_all_ref_tuple_partial

//# run 0xc0ffee::m::test_all_ref_tuple_miss

//# run 0xc0ffee::m::test_all_ref_tuple_var_hit

//# run 0xc0ffee::m::test_all_ref_tuple_var_miss

//# run 0xc0ffee::m::test_ref_generic_hit

//# run 0xc0ffee::m::test_ref_generic_miss

//# run 0xc0ffee::m::test_ref_generic_other_variant

//# run 0xc0ffee::m::test_ref_generic_pair_hit

//# run 0xc0ffee::m::test_ref_generic_pair_partial

//# run 0xc0ffee::m::test_ref_generic_var_lit

//# run 0xc0ffee::m::test_ref_generic_var_lit_fallthrough

//# run 0xc0ffee::m::test_ref_enum_no_literal

//# run 0xc0ffee::m::test_ref_enum_no_literal_v2
