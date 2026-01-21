module 0x8675309::M {
    enum X has copy, drop {
        A,
        B
    }

    fun foo(x: X): u64 {
        match ((x, x)) {
            (X::A, X::A) => 1,
            (_, _) => 2,
        }
    }

    fun bar(x: X): u64 {
        match ((x, x)) {
            (X::A, X::A) => 1,
            _ => 2,
        }
    }

    // Test 1: (X::A, _) and (_, X::A)
    fun test_wildcard_positions(x: X, y: X): u64 {
        match ((x, y)) {
            (X::A, _) => 1,
            (_, X::A) => 2,
            _ => 3,
        }
    }

    // Test 2: Exhaustive set with wildcard should error on unreachable arm
    fun test_exhaustive_then_wildcard(x: X): u64 {
        match ((x, x)) {
            (X::A, X::A) => 1,
            (X::A, X::B) => 2,
            (X::B, X::A) => 3,
            (X::B, X::B) => 4,
            _ => 5,
        }
    }

    // Test 3: Wildcard first, then specific pattern should error on unreachable arm
    fun test_wildcard_first(x: X): u64 {
        match ((x, x)) {
            _ => 1,
            (X::A, X::A) => 2,
        }
    }

    // Test 4: Wildcard with guard condition
    fun test_wildcard_with_guard(x: X, cond: bool): u64 {
        match ((x, x)) {
            (X::A, X::A) => 1,
            _ if cond => 2,
            _ => 3,
        }
    }
}
