//# publish
module 0xc0ffee::m {
    fun cyclic(p: u64): u64 {
        let a = p;
        let b = a;
        p = b;
        p
    }
}

//# run 0xc0ffee::m::cyclic --args 42
