module 0xc0ffee::m {
    enum E has drop {
        V1(u64),
        V2,
    }

    enum Pair has drop {
        P(u64, u64),
        Q,
    }

    enum Inner has drop {
        A(u64),
        B,
    }

    enum Outer has drop {
        W(Inner),
        X,
    }

    // Single literal in enum variant
    fun single_literal(e: E): u64 {
        match (e) {
            E::V1(1) => 1,
            E::V1(_) => 2,
            E::V2 => 3,
        }
    }

    // Multiple literals in one variant
    fun multi_literal(p: Pair): u64 {
        match (p) {
            Pair::P(1, 2) => 10,
            Pair::P(_, _) => 20,
            Pair::Q => 30,
        }
    }

    // Mix of variable + literal
    fun mix_var_literal(p: Pair): u64 {
        match (p) {
            Pair::P(x, 2) => x + 100,
            Pair::P(_, _) => 20,
            Pair::Q => 30,
        }
    }

    // Literal + existing guard
    fun literal_with_guard(e: E): u64 {
        match (e) {
            E::V1(1) if true => 50,
            E::V1(_) => 60,
            E::V2 => 70,
        }
    }

    // Nested enum with literal
    fun nested_literal(o: Outer): u64 {
        match (o) {
            Outer::W(Inner::A(1)) => 100,
            Outer::W(Inner::A(_)) => 200,
            Outer::W(Inner::B) => 300,
            Outer::X => 400,
        }
    }
}
