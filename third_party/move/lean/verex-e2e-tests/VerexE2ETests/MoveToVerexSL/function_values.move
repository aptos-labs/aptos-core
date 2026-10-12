module 0x42::function_values {
    /// Invoke a unary callback from a retained inline helper.
    public inline fun apply(value: u64, f: |u64|u64): u64 {
        f(value)
    }

    /// Invoke a callback with more than one argument.
    public inline fun apply_two(left: u64, right: u64, f: |u64,u64|u64): u64 {
        f(left, right)
    }

    /// Construct a function value in an inline helper: the helper is expanded
    /// where it is called, so its omission from the export loses nothing.
    inline fun increment(value: u64): u64 {
        let f = |x: u64| x + 1;
        f(value)
    }

    /// Construct a function value and invoke it.
    public fun add_one(value: u64): u64 {
        let f = |x: u64| x + 1;
        f(value)
    }

    /// Invoke function values that expanded inline helpers construct.
    public fun add_two(value: u64): u64 {
        increment(increment(value))
    }
}
