module 0xc0ffee::m {
    enum E has drop, copy {
        V1(u64),
        V2,
    }

    enum Pair has drop, copy {
        P(u64, u64),
        Q,
    }

    enum Inner has drop, copy {
        A(u64),
        B,
    }

    enum Outer has drop, copy {
        W(Inner),
        X,
    }

    // Single literal in enum variant through reference
    fun ref_single_literal(e: &E): u64 {
        match (e) {
            E::V1(1) => 1,
            E::V1(_) => 2,
            E::V2 => 3,
        }
    }

    // Multiple literals in one variant through reference
    fun ref_multi_literal(p: &Pair): u64 {
        match (p) {
            Pair::P(1, 2) => 10,
            Pair::P(_, _) => 20,
            Pair::Q => 30,
        }
    }

    // Mix of variable + literal through reference
    fun ref_mix_var_literal(p: &Pair): u64 {
        match (p) {
            Pair::P(x, 2) => *x + 100,
            Pair::P(_, _) => 20,
            Pair::Q => 30,
        }
    }

    // Literal + existing guard through reference
    fun ref_literal_with_guard(e: &E): u64 {
        match (e) {
            E::V1(1) if true => 50,
            E::V1(_) => 60,
            E::V2 => 70,
        }
    }

    // Mutable reference with literal
    fun ref_mut_literal(e: &mut E): u64 {
        match (e) {
            E::V1(1) => 1,
            E::V1(_) => 2,
            E::V2 => 3,
        }
    }

    // Nested enum with literal through reference
    fun ref_nested_literal(o: &Outer): u64 {
        match (o) {
            Outer::W(Inner::A(1)) => 100,
            Outer::W(Inner::A(_)) => 200,
            Outer::W(Inner::B) => 300,
            Outer::X => 400,
        }
    }

    // Top-level literal against reference to primitive (fully transformable)
    fun ref_top_level_literal(x: &u64): u64 {
        match (x) {
            1 => 10,
            2 => 20,
            _ => 0,
        }
    }
}
