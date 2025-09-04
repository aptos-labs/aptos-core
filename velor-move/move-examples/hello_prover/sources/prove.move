module 0x42::prove {
    fun plus1(x: u64): u64 {
        x+1
    }
    spec plus1 {
        ensures result == x+1;
    }

    fun abortsIf0(x: u64) {
        if (x == 0) {
            abort(0)
        };
    }
    spec abortsIf0 {
        aborts_if x == 0;
    }
}
