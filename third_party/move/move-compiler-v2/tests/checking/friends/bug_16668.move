module 0xc0ffee::m {
    package fun package_inner(): u64 {
        12
    }

    public inline fun outer(): u64 {
        package_inner() + package_inner()
    }

    public inline fun outer_2(): u64 {
        outer() + outer()
    }
}

module 0xc0ffee::n {
    fun test() {
        assert!(0xc0ffee::m::outer() == 24);
        assert!(0xc0ffee::m::outer_2() == 48);
    }
}
