//# publish
module 0x42::m {
    fun f(r: &mut u64): &mut u64 {
        r
    }
    fun g() {
        let v = 22;
        let _r = &mut v;
        _r = f(_r);
        let _s = &v;
    }
}

// Note we only check for bytecode verification here executing this example
// does not make sense
