//# publish
module 0xc0ffee::m {
    public fun double(x: u64): u64 {
        match (x) {
            y => x + y
        }
    }
}

//# run 0xc0ffee::m::double --args 10
