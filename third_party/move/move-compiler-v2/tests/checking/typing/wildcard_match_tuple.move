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
}
