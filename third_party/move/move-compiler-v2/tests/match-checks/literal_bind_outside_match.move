module 0xc0ffee::literal_bind_outside_match {

    struct S has drop {
        x: u64,
    }

    struct P(u64) has drop;

    // --- let bindings with literal patterns ---

    fun let_named_field_literal(): u64 {
        let S { x: 1 } = S { x: 1 };
        0
    }

    fun let_positional_field_literal(): u64 {
        let P(1) = P(1);
        0
    }

    fun let_bare_literal(): u64 {
        let 1u64 = 1;
        0
    }

    // --- lambda parameters with literal patterns ---

    fun lambda_named_field_literal(): u64 {
        let f = |S { x: 1 }| 0;
        f(S { x: 1 })
    }

    fun lambda_positional_field_literal(): u64 {
        let f = |P(1)| 0;
        f(P(1))
    }

    fun lambda_bare_literal(): u64 {
        let f = |1u64| 0;
        f(1)
    }

}
