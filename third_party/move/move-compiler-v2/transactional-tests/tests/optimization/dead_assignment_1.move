//# publish
module 0xc0ffee::m {
    fun dead(p: u64): u64 {
        let a = p;
        let a = a;
        a
    }
}

//# run 0xc0ffee::m::dead --args 55
