// These tests are used elsewhere as tests for feature gating

module 0x815::function_types_only {
    public fun fn_id(f: |u64|u64 with copy): |u64|u64 with copy {
        f
    }
}

module 0x815::function_values_apply_only {
    public fun map(f: |u64|u64 with copy, x: u64): u64 {
        (f)(x)
    }
}

module 0x815::function_values_create_only {
    public fun add_func(x: u64, y: u64): u64 {
        x + y
    }
    public fun build_function(x: u64): |u64|u64 with copy+store {
        let f = move |y| add_func(x, y);
        f
    }
}

module 0x815::function_values_early_bind_only {
    fun map(f: |u64|u64 with copy, x: u64): u64 {
        (f)(x)
    }
    public fun add_func(x: u64, y: u64): u64 {
        x + y
    }
    fun build_function(x: u64): |u64|u64 with copy+store {
        let f = move |y| add_func(x, y);
        f
    }
    public fun main(x: u64): u64 {
        let g = build_function(x);
        map(g, 3)
    }
}
