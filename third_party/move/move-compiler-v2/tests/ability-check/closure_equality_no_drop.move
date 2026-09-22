// Equality operands need `drop`; the ability checker enforces this for function values too.
module 0x42::m {
    fun no_drop(f: |u64|u64 has copy + store, g: |u64|u64 has copy + store): bool {
        f == g
    }
}
