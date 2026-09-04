// Function values are compared at identical types: `==` and `!=` do not widen abilities, so
// operands with different declared ability sets are rejected in either order. Operands whose
// type is still open (lambdas, function names used as values) take the other operand's type.
module 0xc0ffee::m {
    fun wide_first(f: |u64|u64 has copy + drop, g: |u64|u64 has drop): bool {
        f == g
    }

    fun narrow_first(f: |u64|u64 has copy + drop, g: |u64|u64 has drop): bool {
        g == f
    }

    fun narrow_first_neq(f: |u64|u64 has copy + drop, g: |u64|u64 has drop): bool {
        g != f
    }

    // Neither ability set is a subset of the other.
    fun incomparable(f: |u64|u64 has copy + drop, g: |u64|u64 has drop + store): bool {
        f == g
    }

    // References are compared frozen, but the referents must still have identical types.
    fun ref_mismatch(f: &|u64|u64 has copy + drop, g: &|u64|u64 has drop): bool {
        f == g
    }

    fun mut_ref_mismatch(f: &mut |u64|u64 has copy + drop, g: &mut |u64|u64 has drop): bool {
        f == g
    }
}
