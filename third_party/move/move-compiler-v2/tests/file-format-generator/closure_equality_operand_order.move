module 0xc0ffee::m {
    // Wide operand first; the reversed expression `g == f` is accepted.
    public fun wide_first(f: |u64|u64 has copy + drop, g: |u64|u64 has drop): bool {
        f == g
    }

    // Covers the separate `!=` inference path.
    public fun wide_first_neq(f: |u64|u64 has copy + drop, g: |u64|u64 has drop): bool {
        f != g
    }

    // Immutable-reference case. The `(&T, &T)` overload fails before `(T, T)` binds `T` to a
    // reference, so the diagnostic reports an invalid type argument instead of an ability
    // mismatch.
    public fun wide_first_ref(f: &|u64|u64 has copy + drop, g: &|u64|u64 has drop): bool {
        f == g
    }

    // Neither ability set is a subset of the other, so both operand orders fail.
    public fun incomparable(f: |u64|u64 has copy + drop, g: |u64|u64 has drop + store): bool {
        f == g
    }

    public fun incomparable_swapped(
        f: |u64|u64 has copy + drop,
        g: |u64|u64 has drop + store
    ): bool {
        g == f
    }
}
