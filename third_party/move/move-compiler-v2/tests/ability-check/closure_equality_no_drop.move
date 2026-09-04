// Equality operands are typed at the join of their types, but `drop` is still required of each
// operand by the ability checker.
module 0x42::m {
    // The join `|u64|u64 has store` lacks `drop`, as does `f` itself.
    fun no_drop(f: |u64|u64 has copy + store, g: |u64|u64 has drop + store): bool {
        f == g
    }
}
