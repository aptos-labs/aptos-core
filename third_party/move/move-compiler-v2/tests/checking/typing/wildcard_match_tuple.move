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

    fun test_wildcard_positions(x: X, y: X): u64 {
        match ((x, y)) {
            (X::A, _) => 1,
            (_, X::A) => 2,
            _ => 3,
        }
    }

    fun test_wildcard_with_guard(x: X, cond: bool): u64 {
        match ((x, x)) {
            (X::A, X::A) => 1,
            _ if cond => 2,
            _ => 3,
        }
    }
}
