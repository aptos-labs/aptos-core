//# publish
module 0x66::m {
    public fun run() {
        debug_assert!(1 > 2);
    }
}

//# run 0x66::m::run
