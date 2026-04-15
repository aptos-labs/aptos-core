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

    // Range in single enum field: E::V1(1..10) extracted to guard
    fun range_in_enum(e: E): u64 {
        match (e) {
            E::V1(1..10) => 1,
            E::V1(_) => 2,
            E::V2 => 3,
        }
    }

    // Multiple ranges in one variant
    fun multi_range_in_enum(p: Pair): u64 {
        match (p) {
            Pair::P(0..10, 0..10) => 1,
            Pair::P(_, _) => 2,
            Pair::Q => 3,
        }
    }

    // Mix of range + literal in variant
    fun range_and_literal_in_enum(p: Pair): u64 {
        match (p) {
            Pair::P(1..10, 42) => 1,
            Pair::P(_, _) => 2,
            Pair::Q => 3,
        }
    }

    // Mix of range + variable in variant
    fun range_and_var_in_enum(p: Pair): u64 {
        match (p) {
            Pair::P(1..10, y) => y,
            Pair::P(_, _) => 0,
            Pair::Q => 0,
        }
    }

    // Range + existing guard
    fun range_with_guard(e: E): u64 {
        match (e) {
            E::V1(1..10) if true => 1,
            E::V1(_) => 2,
            E::V2 => 3,
        }
    }

    // Nested enum with range
    fun nested_range(o: Outer): u64 {
        match (o) {
            Outer::W(Inner::A(1..100)) => 1,
            Outer::W(Inner::A(_)) => 2,
            Outer::W(Inner::B) => 3,
            Outer::X => 4,
        }
    }

    // Reference discriminator with range in enum
    fun ref_range_in_enum(e: &E): u64 {
        match (e) {
            E::V1(0..5) => 1,
            E::V1(_) => 2,
            E::V2 => 3,
        }
    }
}
