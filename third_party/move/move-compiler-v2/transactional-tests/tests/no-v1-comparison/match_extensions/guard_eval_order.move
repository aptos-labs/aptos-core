//# publish
module 0xc0ffee::m {
    fun next(counter: &mut u64): u64 {
        let v = *counter;
        *counter = v + 1;
        v
    }

    enum Color has drop {
        Red,
        Blue,
    }

    fun make_color(): Color {
        Color::Red
    }

    // Guards evaluate top-to-bottom: both guards run, neither succeeds, falls through.
    public fun test_scalar_guard_top_to_bottom() {
        let c: u64 = 0;
        let x: u64 = 0;
        let result = match (x) {
            0 if (next(&mut c) == 5) => 1,  // guard runs (c: 0->1, returns 0), false
            0 if (next(&mut c) == 5) => 2,  // guard runs (c: 1->2, returns 1), false
            0 => 3,                           // fallthrough
            _ => 4,
        };
        assert!(c == 2);
        assert!(result == 3);
    }

    // Guards on non-matching patterns are skipped.
    public fun test_scalar_guard_short_circuit() {
        let c: u64 = 0;
        let x: u64 = 1;
        let result = match (x) {
            0 if (next(&mut c) > 0) => 1,    // pattern doesn't match, guard skipped
            1 if (next(&mut c) == 0) => 2,   // pattern matches, guard runs (c: 0->1, returns 0), true
            _ => 3,
        };
        assert!(c == 1);
        assert!(result == 2);
    }

    // Tuple match with guarded arms: guards evaluate top-to-bottom.
    public fun test_tuple_guard_order() {
        let c: u64 = 0;
        let x: u64 = 1;
        let y: u64 = 2;
        let result = match ((x, y)) {
            (1, 2) if (next(&mut c) == 5) => 1,  // matches, guard runs (c: 0->1, returns 0), false
            (1, 2) if (next(&mut c) == 1) => 2,  // matches, guard runs (c: 1->2, returns 1), true
            _ => 3,
        };
        assert!(c == 2);
        assert!(result == 2);
    }

    // Mixed tuple (enum + primitive) with side-effecting guards.
    public fun test_mixed_guard_top_to_bottom() {
        let c: u64 = 0;
        let x: u64 = 5;
        let result = match ((make_color(), x)) {
            (Color::Red, 5) if (next(&mut c) == 5) => 1,  // pattern+prim match, guard runs (c: 0->1), false
            (Color::Red, 5) if (next(&mut c) == 1) => 2,  // pattern+prim match, guard runs (c: 1->2, returns 1), true
            _ => 3,
        };
        assert!(c == 2);
        assert!(result == 2);
    }

    // When the primitive position doesn't match, the guard should not run.
    public fun test_mixed_guard_skipped_on_prim_mismatch() {
        let c: u64 = 0;
        let x: u64 = 99;
        let result = match ((make_color(), x)) {
            (Color::Red, 5) if (next(&mut c) == 0) => 1,    // prim doesn't match, guard skipped
            (Color::Red, _) if (next(&mut c) == 0) => 2,    // prim wildcard matches, guard runs (c: 0->1, returns 0), true
            _ => 3,
        };
        assert!(c == 1);
        assert!(result == 2);
    }

    // Discriminator side effects happen before any guard side effects.
    public fun test_disc_before_guards() {
        let c: u64 = 0;
        let result = match (next(&mut c)) {   // c: 0->1, returns 0
            0 if ({ c = c + 10; true }) => 1,  // guard runs, c: 1->11
            _ => 2,
        };
        assert!(c == 11);
        assert!(result == 1);
    }
}

//# run 0xc0ffee::m::test_scalar_guard_top_to_bottom

//# run 0xc0ffee::m::test_scalar_guard_short_circuit

//# run 0xc0ffee::m::test_tuple_guard_order

//# run 0xc0ffee::m::test_mixed_guard_top_to_bottom

//# run 0xc0ffee::m::test_mixed_guard_skipped_on_prim_mismatch

//# run 0xc0ffee::m::test_disc_before_guards
