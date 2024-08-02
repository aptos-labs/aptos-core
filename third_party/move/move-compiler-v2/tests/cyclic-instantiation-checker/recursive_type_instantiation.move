module 0x42::m0 {
    struct S<T> {
        f: T
    }

    public fun simple_recursion<T>() {
        simple_recursion<S<T>>()
    }

    fun two_level_recursion_0<T>() {
        two_level_recursion_1<T>()
    }

    fun two_level_recursion_1<T>() {
        two_level_recursion_0<S<T>>()
    }

    fun three_level_recursion_0<T>() {
        three_level_recursion_1<T>()
    }

    fun three_level_recursion_1<T>() {
        three_level_recursion_2<T>()
    }

    fun three_level_recursion_2<T>() {
        three_level_recursion_0<S<T>>()
    }

    fun recurse_at_different_position<T1, T2>() {
        recurse_at_different_position<T2, S<T1>>()
    }

    // ok
    fun simple_loop<T>() {
        simple_loop<T>()
    }

    // ok
    fun simple_recursion_no_ty_param() {
        simple_recursion_no_ty_param()
    }

    fun test_vec<T>() {
        test_vec<vector<T>>()
    }

    fun call_native() {
        std::vector::empty<bool>();
    }
}

module 0x42::m1 {
    use 0x42::m0::simple_recursion;

    fun call_simple_recursion() {
        // no err message for this
        simple_recursion<bool>()
    }
}
