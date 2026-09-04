// Equality binds its type parameter to the join of the operand types, so either operand order is
// accepted whenever a common type exists.
module 0xc0ffee::m {
    // Wide operand first. The join is `|u64|u64 has drop`.
    public fun wide_first(f: |u64|u64 has copy + drop, g: |u64|u64 has drop): bool {
        f == g
    }

    // `!=` is a separately declared builtin.
    public fun wide_first_neq(f: |u64|u64 has copy + drop, g: |u64|u64 has drop): bool {
        f != g
    }

    // Immutable references join to a reference to the join.
    public fun wide_first_ref(f: &|u64|u64 has copy + drop, g: &|u64|u64 has drop): bool {
        f == g
    }

    // Mutable references are compared frozen and join like immutable ones.
    public fun wide_first_mut_ref(
        f: &mut |u64|u64 has copy + drop,
        g: &mut |u64|u64 has drop
    ): bool {
        f == g
    }

    // Neither ability set is a subset of the other. Both operand orders compile to a comparison
    // at the join `|u64|u64 has drop`.
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
