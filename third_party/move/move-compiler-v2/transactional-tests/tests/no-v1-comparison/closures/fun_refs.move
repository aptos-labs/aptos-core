//# publish
module 0x42::test {

    public fun ref(): u64 {
        // TODO(#15664): ability inference not working as expected. The closure gets a type assigned
        //   via the parameter to `ref_helper` which makes it non-droppable, but it IS droppable.
        //   Need to revise inference.
        let f : |u64|u64 has drop+copy = |x| x + x;
        ref_helper(&f, 5)
    }

    fun ref_helper(f: &|u64|u64 has copy, x: u64): u64 {
        (*f)(x)
    }

    public fun ref_mut(): u64 {
        let f : |u64|u64 has drop+copy = |x| x * x;
        ref_mut_helper(&mut f, 5);
        f(2)
    }

    fun ref_mut_helper(f: &mut |u64|u64 has copy+drop, x: u64) {
        let cur_f = *f; // can't capture references
        *f = |y|cur_f(x + y)
    }

}

//# run 0x42::test::ref

//# run 0x42::test::ref_mut
