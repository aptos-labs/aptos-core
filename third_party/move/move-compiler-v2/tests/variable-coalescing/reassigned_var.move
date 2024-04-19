module 0xc0ffee::m {
    fun test(): u64 {
        let a = 1;
        let b = 2;
        a = 9; // reassigned
        a + b
    }

}
