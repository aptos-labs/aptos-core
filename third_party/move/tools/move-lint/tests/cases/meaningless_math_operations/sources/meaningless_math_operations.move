module NamedAddr::Detector {
      // Correct usage
    public fun correct_usage(x: u64): u64 {
        x + 2
    }

    // Meaningless operations
    public fun multiply_by_zero(x: u64): u64 {
        x * 0 // Should trigger a warning
    }

    public fun shift_left_by_zero(x: u64): u64 {
        x << 0 // Should trigger a warning
    }

    public fun shift_right_by_zero(x: u64): u64 {
        x >> 0 // Should trigger a warning
    }

    public fun multiply_by_one(x: u64): u64 {
        x * 1 // Should trigger a warning
    }

    public fun add_zero(x: u64): u64 {
        x + 0 // Should trigger a warning
    }

    public fun subtract_zero(x: u64): u64 {
        x - 0 // Should trigger a warning
    }

    public fun divide_by_zero(x: u64): u64 {
        x / 0 // Should trigger a warning, but also note that this is a runtime error
    }
}
