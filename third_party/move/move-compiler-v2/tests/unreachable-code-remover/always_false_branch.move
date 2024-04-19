module 0xc0ffee::m {
    fun test(): u64 {
        if (false) {
            let i = 0;
            i = i + 1;
            return i
        };
        0
    }

}
