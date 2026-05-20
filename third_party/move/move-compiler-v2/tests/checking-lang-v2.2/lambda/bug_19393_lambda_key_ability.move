// Tests that lambdas cannot be passed where a type requiring `key` or `store` ability is expected.
// Regression test for https://github.com/aptos-labs/aptos-core/issues/19393
module 0xa::test {
    // Requires key + drop: lambda cannot satisfy key
    fun require_key<T: key + drop>(_x: T) { }

    // Requires only copy + drop: lambda CAN satisfy this (should be OK)
    fun require_copy_drop<T: copy + drop>(_x: T) { }

    fun bad_key() {
        require_key(|| 1u64);
    }

    fun ok_copy_drop() {
        // This should be valid: lambda || 1u64 has copy + drop
        require_copy_drop(|| 1u64);
    }
}
