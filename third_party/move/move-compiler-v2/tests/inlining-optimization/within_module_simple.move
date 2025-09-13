module 0xc0ffee::m {
    fun one(): u64 {
        1
    }

    public fun two(): u64 {
        2
    }

    public fun compute(): u64 {
        one() + two()
    }
}
