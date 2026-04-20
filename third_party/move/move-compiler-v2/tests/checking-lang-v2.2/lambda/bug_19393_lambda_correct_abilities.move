// Tests that lambdas with correct ability sets are accepted without errors.
// Companion to bug_19393_lambda_key_ability.move.
module 0xa::test {
    // No ability constraints: any lambda is accepted
    fun require_none<T>(_x: T) { }

    // Only drop: lambda always has at least drop
    fun require_drop<T: drop>(_x: T) { }

    // copy + drop: lambda over copy+drop captures satisfies this
    fun require_copy_drop<T: copy + drop>(_x: T) { }

    fun ok() {
        require_none(|| 1u64);
        require_drop(|| 1u64);
        require_copy_drop(|| 1u64);
    }
}
