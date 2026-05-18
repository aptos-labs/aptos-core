module 0x42::m {
    // Package function with no callers - should warn (unused_function)
    package fun unused_pkg_func(): u64 { 1 }

    // Self-recursive package function - should warn (unused_function, self-call doesn't count)
    package fun self_recursive_pkg(n: u64): u64 {
        if (n <= 1) { 1 }
        else { n * self_recursive_pkg(n - 1) }
    }

    // Package function called only from the same module - should warn (needless_visibility)
    package fun same_module_only_pkg(): u64 { 2 }

    public fun caller(): u64 {
        same_module_only_pkg()
    }
}
