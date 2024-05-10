//# publish
module 0xc0ffee::m {
    fun inc(x: &mut u64): u64 {
        *x = *x + 1;
        *x
    }

    public fun test1(): u64 {
        let x = 1;
        inc(&mut x) + { inc(&mut x); inc(&mut x) + inc(&mut x) } + { inc(&mut x); inc(&mut x); inc(&mut x) }
    }

    public fun test2(): u64 {
        let x = 1;
        inc(&mut {x = x + 1; x}) + { inc(&mut x); inc(&mut x) + inc(&mut x) } + { inc(&mut x); inc(&mut x); inc(&mut x) }
    }
}

//# run 0xc0ffee::m::test1

//# run 0xc0ffee::m::test2
