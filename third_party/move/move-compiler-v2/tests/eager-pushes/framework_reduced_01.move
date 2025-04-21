module 0xc0ffee::m {
    fun one(): u64 {
        1
    }

    fun two(): u64 {
        2
    }

    fun foo() {}

    public fun test(p: u64) {
        let e = two();
        if (p - one() > e) {
            foo();
        }
    }

}
