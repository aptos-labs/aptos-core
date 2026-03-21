//# publish
module 0xc0ffee::m {
    // --- Enums with single field ---
    enum E1 has drop { V1(u64), V2 }
    enum E2 has drop { V1{x: u64}, V2 }

    // --- Enums with multiple fields ---
    enum Pair has drop { P(u64, u64), Q }
    enum PairNamed has drop { P{a: u64, b: u64}, Q }

    // --- Nested enum ---
    enum Inner has drop { A(u64), B }
    enum Outer has drop { W(Inner), X }

    // === Basic: single positional literal ===
    public fun test_positional_hit(): u64 {
        match (E1::V1(1)) { E1::V1(1) => 1, E1::V1(_) => 2, E1::V2 => 3 }
    }
    public fun test_positional_fallthrough(): u64 {
        match (E1::V1(42)) { E1::V1(1) => 1, E1::V1(_) => 2, E1::V2 => 3 }
    }
    public fun test_positional_other_variant(): u64 {
        match (E1::V2) { E1::V1(1) => 1, E1::V1(_) => 2, E1::V2 => 3 }
    }

    // === Basic: single named literal ===
    public fun test_named_hit(): u64 {
        match (E2::V1{x:1}) { E2::V1{x:1} => 1, E2::V1{x:_} => 2, E2::V2 => 3 }
    }
    public fun test_named_fallthrough(): u64 {
        match (E2::V1{x:42}) { E2::V1{x:1} => 1, E2::V1{x:_} => 2, E2::V2 => 3 }
    }
    public fun test_named_other_variant(): u64 {
        match (E2::V2) { E2::V1{x:1} => 1, E2::V1{x:_} => 2, E2::V2 => 3 }
    }

    // === Multiple literals in one variant (positional) ===
    public fun test_multi_pos_hit(): u64 {
        match (Pair::P(1, 2)) { Pair::P(1, 2) => 10, Pair::P(_, _) => 20, Pair::Q => 30 }
    }
    public fun test_multi_pos_partial(): u64 {
        match (Pair::P(1, 99)) { Pair::P(1, 2) => 10, Pair::P(_, _) => 20, Pair::Q => 30 }
    }

    // === Multiple literals in one variant (named) ===
    public fun test_multi_named_hit(): u64 {
        match (PairNamed::P{a:1,b:2}) { PairNamed::P{a:1,b:2} => 10, PairNamed::P{a:_,b:_} => 20, PairNamed::Q => 30 }
    }
    public fun test_multi_named_partial(): u64 {
        match (PairNamed::P{a:1,b:99}) { PairNamed::P{a:1,b:2} => 10, PairNamed::P{a:_,b:_} => 20, PairNamed::Q => 30 }
    }

    // === Mix of variable + literal (positional) ===
    public fun test_mix_pos(): u64 {
        match (Pair::P(5, 2)) { Pair::P(x, 2) => x + 100, Pair::P(_, _) => 20, Pair::Q => 30 }
    }
    public fun test_mix_pos_miss(): u64 {
        match (Pair::P(5, 99)) { Pair::P(x, 2) => x + 100, Pair::P(_, _) => 20, Pair::Q => 30 }
    }

    // === Mix of variable + literal (named) ===
    public fun test_mix_named(): u64 {
        match (PairNamed::P{a:5,b:2}) { PairNamed::P{a:x,b:2} => x + 100, PairNamed::P{a:_,b:_} => 20, PairNamed::Q => 30 }
    }
    public fun test_mix_named_miss(): u64 {
        match (PairNamed::P{a:5,b:99}) { PairNamed::P{a:x,b:2} => x + 100, PairNamed::P{a:_,b:_} => 20, PairNamed::Q => 30 }
    }

    // === Literal + existing guard (positional) ===
    public fun test_guard_pos_pass(): u64 {
        match (E1::V1(1)) { E1::V1(1) if true => 50, E1::V1(_) => 60, E1::V2 => 70 }
    }
    public fun test_guard_pos_fail(): u64 {
        match (E1::V1(1)) { E1::V1(1) if false => 50, E1::V1(_) => 60, E1::V2 => 70 }
    }

    // === Literal + existing guard (named) ===
    public fun test_guard_named_pass(): u64 {
        match (E2::V1{x:1}) { E2::V1{x:1} if true => 50, E2::V1{x:_} => 60, E2::V2 => 70 }
    }
    public fun test_guard_named_fail(): u64 {
        match (E2::V1{x:1}) { E2::V1{x:1} if false => 50, E2::V1{x:_} => 60, E2::V2 => 70 }
    }

    // === Nested enum with literal ===
    public fun test_nested_hit(): u64 {
        match (Outer::W(Inner::A(1))) {
            Outer::W(Inner::A(1)) => 100,
            Outer::W(Inner::A(_)) => 200,
            Outer::W(Inner::B) => 300,
            Outer::X => 400,
        }
    }
    public fun test_nested_miss(): u64 {
        match (Outer::W(Inner::A(42))) {
            Outer::W(Inner::A(1)) => 100,
            Outer::W(Inner::A(_)) => 200,
            Outer::W(Inner::B) => 300,
            Outer::X => 400,
        }
    }

    // --- Deeply nested: literals at multiple levels ---
    struct Wrap has drop { inner: Inner, tag: u64 }

    // All literals match
    public fun test_deep_nested_hit(): u64 {
        match (Outer::W(Inner::A(1))) {
            Outer::W(Inner::A(1)) => 10,
            Outer::W(_) => 20,
            Outer::X => 30,
        }
    }

    // Inner literal misses
    public fun test_deep_nested_inner_miss(): u64 {
        match (Outer::W(Inner::A(99))) {
            Outer::W(Inner::A(1)) => 10,
            Outer::W(_) => 20,
            Outer::X => 30,
        }
    }

    // Struct wrapping enum with literals at both levels
    public fun test_deep_struct_hit(): u64 {
        let w = Wrap { inner: Inner::A(1), tag: 42 };
        match (w) {
            Wrap { inner: Inner::A(1), tag: 42 } => 100,
            Wrap { inner: Inner::A(_), tag: _ } => 200,
            Wrap { inner: Inner::B, tag: _ } => 300,
        }
    }

    public fun test_deep_struct_miss_inner(): u64 {
        let w = Wrap { inner: Inner::A(99), tag: 42 };
        match (w) {
            Wrap { inner: Inner::A(1), tag: 42 } => 100,
            Wrap { inner: Inner::A(_), tag: _ } => 200,
            Wrap { inner: Inner::B, tag: _ } => 300,
        }
    }

    public fun test_deep_struct_miss_tag(): u64 {
        let w = Wrap { inner: Inner::A(1), tag: 99 };
        match (w) {
            Wrap { inner: Inner::A(1), tag: 42 } => 100,
            Wrap { inner: Inner::A(_), tag: _ } => 200,
            Wrap { inner: Inner::B, tag: _ } => 300,
        }
    }

    // --- Plain struct (non-enum) with literal fields ---
    struct S has drop { x: u64, y: u64 }

    public fun test_struct_hit(): u64 {
        match (S { x: 1, y: 42 }) { S { x: 1, y } => y + 100, S { x: _, y: _ } => 0 }
    }
    public fun test_struct_miss(): u64 {
        match (S { x: 99, y: 42 }) { S { x: 1, y } => y + 100, S { x: _, y: _ } => 0 }
    }

    // === Mixed tuple: struct literal extraction + mixed-tuple lowering ===
    public fun test_mixed_tuple_enum_hit(): u64 {
        match ((E1::V1(1), 2)) { (E1::V1(1), 2) => 10, (E1::V1(_), _) => 20, (E1::V2, _) => 30 }
    }
    public fun test_mixed_tuple_enum_miss_prim(): u64 {
        match ((E1::V1(1), 99)) { (E1::V1(1), 2) => 10, (E1::V1(_), _) => 20, (E1::V2, _) => 30 }
    }
    public fun test_mixed_tuple_enum_miss_slit(): u64 {
        match ((E1::V1(42), 2)) { (E1::V1(1), 2) => 10, (E1::V1(_), _) => 20, (E1::V2, _) => 30 }
    }
    public fun test_mixed_tuple_struct_hit(): u64 {
        match ((S { x: 1, y: 5 }, 2)) { (S { x: 1, y }, 2) => y + 100, (S { x: _, y: _ }, _) => 0 }
    }
    public fun test_mixed_tuple_struct_miss(): u64 {
        match ((S { x: 99, y: 5 }, 2)) { (S { x: 1, y }, 2) => y + 100, (S { x: _, y: _ }, _) => 0 }
    }
}

//# run 0xc0ffee::m::test_positional_hit

//# run 0xc0ffee::m::test_positional_fallthrough

//# run 0xc0ffee::m::test_positional_other_variant

//# run 0xc0ffee::m::test_named_hit

//# run 0xc0ffee::m::test_named_fallthrough

//# run 0xc0ffee::m::test_named_other_variant

//# run 0xc0ffee::m::test_multi_pos_hit

//# run 0xc0ffee::m::test_multi_pos_partial

//# run 0xc0ffee::m::test_multi_named_hit

//# run 0xc0ffee::m::test_multi_named_partial

//# run 0xc0ffee::m::test_mix_pos

//# run 0xc0ffee::m::test_mix_pos_miss

//# run 0xc0ffee::m::test_mix_named

//# run 0xc0ffee::m::test_mix_named_miss

//# run 0xc0ffee::m::test_guard_pos_pass

//# run 0xc0ffee::m::test_guard_pos_fail

//# run 0xc0ffee::m::test_guard_named_pass

//# run 0xc0ffee::m::test_guard_named_fail

//# run 0xc0ffee::m::test_nested_hit

//# run 0xc0ffee::m::test_nested_miss

//# run 0xc0ffee::m::test_deep_nested_hit

//# run 0xc0ffee::m::test_deep_nested_inner_miss

//# run 0xc0ffee::m::test_deep_struct_hit

//# run 0xc0ffee::m::test_deep_struct_miss_inner

//# run 0xc0ffee::m::test_deep_struct_miss_tag

//# run 0xc0ffee::m::test_struct_hit

//# run 0xc0ffee::m::test_struct_miss

//# run 0xc0ffee::m::test_mixed_tuple_enum_hit

//# run 0xc0ffee::m::test_mixed_tuple_enum_miss_prim

//# run 0xc0ffee::m::test_mixed_tuple_enum_miss_slit

//# run 0xc0ffee::m::test_mixed_tuple_struct_hit

//# run 0xc0ffee::m::test_mixed_tuple_struct_miss
