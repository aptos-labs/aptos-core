module 0xc0ffee::m {
    struct DropStore has drop, store { x: u64 }

    struct CopyDropStore has copy, drop, store { x: u64 }

    public fun with_capture(d: DropStore, y: u64): u64 {
        let DropStore { x } = d;
        x + y
    }

    public fun with_copy_capture(c: CopyDropStore, y: u64): u64 {
        c.x + y
    }

    // The verifier derives a wider `PackClosure` type than the shared source type.
    public fun widened_producer(d: DropStore, g: |u64|u64 has drop): bool {
        (|y| with_capture(d, y)) == g
    }

    // The lambda takes the other operand's type in either operand order.
    public fun widened_second(d: DropStore, g: |u64|u64 has drop): bool {
        g == (|y| with_capture(d, y))
    }

    public fun widened_neq(d: DropStore, g: |u64|u64 has drop): bool {
        (|y| with_capture(d, y)) != g
    }

    // Both operands share the checked type `|u64|u64 has drop` but derive different abilities,
    // so each must be normalized to the join.
    public fun both_widened(d: DropStore, c: CopyDropStore): bool {
        let lhs: |u64|u64 has drop = |y| with_capture(d, y);
        let rhs: |u64|u64 has drop = |y| with_copy_capture(c, y);
        lhs == rhs
    }

    // Exercises repeated-operand copy/move handling in the two-scratch sequence.
    public fun same_operand(f: |u64|u64 has copy + drop): bool {
        f == f
    }

    // Exercises equality normalization through immutable references.
    public fun ref_compare(d: DropStore, g: &|u64|u64 has drop): bool {
        let lhs: |u64|u64 has drop = |y| with_capture(d, y);
        &lhs == g
    }

    // Reuses one scratch-local pair across two comparisons with the same join type.
    public fun reuses_scratch(
        d: DropStore,
        c: CopyDropStore,
        g: |u64|u64 has drop,
        h: |u64|u64 has drop
    ): bool {
        ((|y| with_capture(d, y)) == g) && ((|y| with_copy_capture(c, y)) == h)
    }

    public fun inc(y: u64): u64 {
        y + 1
    }

    // A function name used as a value takes the other operand's type, like a lambda.
    public fun named_function(g: |u64|u64 has drop): bool {
        inc == g
    }

    // A non-function comparison bypasses normalization and emits plain `Eq`.
    public fun plain(x: u64, y: u64): bool {
        x == y
    }
}
