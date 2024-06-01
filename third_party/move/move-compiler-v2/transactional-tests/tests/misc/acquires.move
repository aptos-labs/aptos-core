//# publish
module 0x42::m {

    struct R has key { v: u64 }

    fun read(): u64 acquires R {
        let x = borrow_global<R>(@0x42).v;
        x
    }

    public fun test(): u64 acquires R {
        read2()
    }

    inline fun read2(): u64 {
        read()
    }
}
