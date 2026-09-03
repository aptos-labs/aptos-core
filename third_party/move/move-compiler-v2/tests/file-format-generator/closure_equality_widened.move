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

    // The operands have different declared ability sets.
    public fun differently_declared(
        f: |u64|u64 has copy + drop + store,
        g: |u64|u64 has drop
    ): bool {
        g == f
    }

    public fun differently_declared_neq(
        f: |u64|u64 has copy + drop + store,
        g: |u64|u64 has drop
    ): bool {
        g != f
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

    // Exercises equality joining through immutable references.
    public fun ref_compare(
        f: &|u64|u64 has copy + drop + store,
        g: &|u64|u64 has drop
    ): bool {
        g == f
    }

    // Reuses one scratch-local pair across two comparisons with the same join type.
    public fun reuses_scratch(
        f: |u64|u64 has copy + drop + store,
        g: |u64|u64 has drop,
        h: |u64|u64 has drop
    ): bool {
        (g == f) && (h == f)
    }

    // A non-function comparison bypasses normalization and emits plain `Eq`.
    public fun plain(x: u64, y: u64): bool {
        x == y
    }
}
