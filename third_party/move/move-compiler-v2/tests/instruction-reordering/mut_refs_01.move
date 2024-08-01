module 0xc0ffee::m {
    fun inc_immut(x: u64): u64 {
        x + 1
    }

    public fun test1(): u64 {
        let x = 1;
        inc_immut({x = inc_mut(&mut x) + 1; x = x + 1; x}) + {x = inc_mut(&mut x) + 1; x}
    }

    fun inc_mut(x: &mut u64): u64 {
        *x = *x + 1;
        *x
    }
}
