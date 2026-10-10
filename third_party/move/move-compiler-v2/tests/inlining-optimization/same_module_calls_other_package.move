// Inlining a same-module helper preserves its calls to public functions in other
// packages. The caller has the same access and is recompiled with the helper.
module 0xc0ffee::m {
    fun helper(v: &vector<u64>): bool {
        std::vector::is_empty(v)
    }

    public fun compute(v: &vector<u64>): bool {
        helper(v)
    }
}
