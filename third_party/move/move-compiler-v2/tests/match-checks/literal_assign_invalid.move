module 0xc0ffee::literal_assign_invalid {

    struct S has drop {
        x: u64,
    }

    struct P(u64) has drop;

    // --- assignments (LHS parsed as expression, not as bind pattern) ---

    fun assign_named_field_literal(): u64 {
        let s = S { x: 1 };
        S { x: 1 } = s;
        0
    }

    fun assign_positional_field_literal(): u64 {
        let p = P(1);
        P(1) = p;
        0
    }

    fun assign_bare_literal(): u64 {
        let x: u64 = 0;
        1 = x;
        0
    }
}
