module 0xc0ffee::m {
    package fun package_inner(): u64 {
        42
    }

    package inline fun outer(): u64 {
        package_inner()
    }
}

module 0xc0ffee::n {
    use 0xc0ffee::m;

    public fun call_outer(): u64 {
        m::outer()
    }
}
