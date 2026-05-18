module 0x42::m {
    // Private inline function called by public function
    inline fun add_one(x: u64): u64 {
        x + 1
    }

    // Inline function calling another inline
    inline fun add_two(x: u64): u64 {
        add_one(add_one(x))
    }

    // Inline function with lambda
    inline fun apply(x: u64, f: |u64| u64): u64 {
        f(x)
    }

    public fun test(): u64 {
        let a = add_two(10);
        apply(a, |v| v * 2)
    }
}
