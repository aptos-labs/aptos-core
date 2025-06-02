module 0xc0ffee::m {
    fun test1(): & &u64 {
        let x: & &u64;
        return x
    }

    fun test2(x: &mut &mut u64): (&mut &mut u64, &mut &mut u64) {(x, x)}

    fun test3(x: &mut &mut u64): &mut &mut u64 {x}

    fun test4() {
        let x: &mut &mut u64;
    }
}
