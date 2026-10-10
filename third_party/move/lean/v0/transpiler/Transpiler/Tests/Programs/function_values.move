module 0x42::function_values {
    /// Invoke a unary callback from a retained inline helper.
    public inline fun apply(value: u64, f: |u64|u64): u64 {
        f(value)
    }

    /// Invoke a callback with more than one argument.
    public inline fun apply_two(left: u64, right: u64, f: |u64,u64|u64): u64 {
        f(left, right)
    }
}
