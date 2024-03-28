//# publish
module 0xc0ffee::m {
    fun dead(p: u64): u64 {
        p = p;
        p
    }
}

//# run 0xc0ffee::m::dead --args 53
