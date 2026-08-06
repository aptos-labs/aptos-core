// Ghost field initializers (`ghost f: T = init;`) and data invariants over
// ghosts: initializers are evaluated over the packed runtime fields at every
// pack; data invariants and the declared type's value domain are asserted at
// every pack and update. See `ghost_field.move` for the basic mechanics.
module 0x42::ghost_field_invariants {
    // Ghost field with an initializer expression referencing a runtime field.
    // The initializer is evaluated at each Pack so the data invariant holds
    // immediately, without requiring an inline `update`.
    struct WithInit has copy, drop { x: u64 }
    spec WithInit {
        ghost g: u64 = x;
        invariant g >= x;
    }

    // Positive: pack alone establishes the invariant.
    fun with_init_pack(n: u64): WithInit {
        WithInit { x: n }
    }
    spec with_init_pack {
        ensures result.g == n;
    }

    // Positive: initializer holds, then update to a value that still
    // satisfies the invariant.
    fun with_init_update(): WithInit {
        let s = WithInit { x: 3 };
        spec { update s.g = 10; };
        s
    }
    spec with_init_update {
        ensures result.g == 10;
    }

    // Ghost field with a literal initializer.
    struct WithConstInit has copy, drop { x: u64 }
    spec WithConstInit {
        ghost total: u64 = 0;
        invariant total <= 100;
    }
    fun with_const_init_pack(): WithConstInit {
        WithConstInit { x: 7 }
    }
    spec with_const_init_pack {
        ensures result.total == 0;
    }

    // Initializer that does NOT satisfy the data invariant at Pack: this must FAIL.
    struct BadInit has copy, drop { x: u64 }
    spec BadInit {
        ghost g: u64 = 0;
        invariant g >= x;
    }
    fun bad_init_pack(): BadInit { BadInit { x: 5 } }

    // ---- Multi-field additive initializer: `total = x + y` implies
    // `total >= x` and `total >= y` at every pack ----
    struct SumS has copy, drop { x: u64, y: u64 }
    spec SumS {
        ghost total: u64 = x + y;
        invariant total >= x;
        invariant total >= y;
    }

    fun sum_pack_small(): SumS { SumS { x: 2, y: 3 } }
    spec sum_pack_small { ensures result.total == 5; }

    // Update total to a value that still satisfies both invariants. The
    // requires discharges the update's range check (`total + 1` must stay
    // in u64).
    fun sum_update(s: &mut SumS) {
        spec { update s.total = s.total + 1; };
    }
    spec sum_update { requires s.total < 18446744073709551615; }

    // Update total below x — invariant `total >= x` is violated. FAILS.
    fun sum_break(s: &mut SumS) {
        spec { update s.total = 0; };
    }

    // ---- Ghost tracking the last-seen value of a runtime field ----
    struct Mono has copy, drop { x: u64 }
    spec Mono {
        ghost max_seen: u64 = x;
        invariant max_seen >= x;
    }

    fun mono_pack(n: u64): Mono { Mono { x: n } }

    // Grow both in tandem: update ghost first, then runtime — invariant
    // holds after each mutation.
    fun mono_grow(s: &mut Mono) {
        spec { update s.max_seen = s.max_seen + 1; };
        s.x = s.x + 1;
    }
    spec mono_grow {
        // Discharges the update's range check on `max_seen + 1`.
        requires s.max_seen < 18446744073709551615;
        aborts_if s.x + 1 > MAX_U64;
    }

    // Grow only the runtime field: at the write, `max_seen >= x` fails
    // because the ghost hasn't advanced. FAILS.
    fun mono_break_write(s: &mut Mono) {
        s.x = s.x + 100;
    }
    spec mono_break_write { aborts_if s.x + 100 > MAX_U64; }

    // ---- Two ghost fields, both with runtime initializers ----
    struct RangeG has copy, drop { lo: u64, hi: u64 }
    spec RangeG {
        ghost g_lo: u64 = lo;
        ghost g_hi: u64 = hi;
        invariant g_lo <= g_hi;
    }

    fun range_pack_eq(): RangeG { RangeG { lo: 3, hi: 3 } }
    fun range_pack_lt(): RangeG { RangeG { lo: 1, hi: 5 } }

    // Pack with lo > hi: invariant `g_lo <= g_hi` violated. FAILS.
    fun range_pack_reversed(): RangeG { RangeG { lo: 5, hi: 3 } }

    // ---- Enum with variant-agnostic ghost + literal initializer +
    // invariant referencing only the ghost ----
    enum EInit has copy, drop { A { y: u64 }, B }
    spec EInit {
        ghost h: u64 = 0;
        invariant h < 100;
    }

    fun einit_pack_a(): EInit { EInit::A { y: 1 } }
    fun einit_pack_b(): EInit { EInit::B }
    fun einit_update_ok(e: &mut EInit) { spec { update e.h = 42; }; }
    // Update the ghost past the invariant bound. FAILS.
    fun einit_update_bad(e: &mut EInit) { spec { update e.h = 200; }; }

    // ---- Nested struct where the inner has a runtime-driven ghost init
    // and a matching invariant ----
    struct InnerG has copy, drop { v: u64 }
    spec InnerG {
        ghost gi: u64 = v;
        invariant gi == v;
    }
    struct OuterG has copy, drop { i: InnerG }

    fun outer_nested_pack(): OuterG { OuterG { i: InnerG { v: 7 } } }
    spec outer_nested_pack {
        ensures result.i.gi == 7;
        ensures result.i.v == 7;
    }

    // ---- Typed ghosts are range-checked at every write ----
    // A ghost's declared type is enforced: initializers at pack and update
    // RHS values are asserted to stay in the type's value domain, which
    // justifies assuming the range at boundaries ($IsValid). Use `num` for
    // an unbounded ghost integer.
    struct OvfS has copy, drop { x: u64 }
    spec OvfS { ghost g: u64 = x + 1; }

    // Initializer can overflow at this pack: the range check FAILS.
    fun ovf_init_out_of_range(): OvfS {
        OvfS { x: 18446744073709551615 }
    }

    // With the overflow excluded, the range guarantee holds at boundaries.
    fun ovf_range_guarantee(n: u64): OvfS {
        OvfS { x: n }
    }
    spec ovf_range_guarantee {
        requires n < 18446744073709551615;
        ensures result.g == n + 1;
        ensures result.g <= 18446744073709551615;
    }

    // Update RHS can overflow: the range check FAILS without a requires.
    fun ovf_update_out_of_range(s: &mut OvfS) {
        spec { update s.g = s.g + 1; };
    }

    // `num` ghosts are unbounded: no range check applies.
    struct NumG has copy, drop { x: u64 }
    spec NumG { ghost big: num; }
    fun num_unbounded(s: &mut NumG) {
        spec { update s.big = 18446744073709551615 + 1; };
    }
    spec num_unbounded { ensures s.big == 18446744073709551616; }

    // Data invariants over ghosts compose with the range checks and are
    // likewise asserted at every pack and update.
    struct RangedG has copy, drop { x: u64 }
    spec RangedG {
        ghost g: u64 = 0;
        invariant g <= 100;
    }
    fun ranged_pack(n: u64): RangedG { RangedG { x: n } }
    spec ranged_pack { ensures result.g <= 100; }

    // ---- Ghost initializer with a quantifier that shadows a runtime field ----
    // The bound `x` in the quantifier must NOT be captured by the pack-arg
    // substitution for the field `x`. The initializer's true meaning is
    // `forall integer x. x != 0`, i.e. false, so `ensures !result.g` holds;
    // under a capture bug it would fail.
    struct Cap has copy, drop { x: u64 }
    spec Cap {
        ghost g: bool = forall x: u64: x != 0;
    }
    fun cap_pack(n: u64): Cap {
        Cap { x: n }
    }
    spec cap_pack {
        ensures !result.g;
    }
}
