//# publish
module 0x42::M {
    fun t2(u1: &mut u64, u2: &mut u64): (&mut u64, &mut u64) {
        (u1, u2)
    }
    public fun bar() {
        let x: &u64;
        let y: &u64;
        (x, y) = t2(&mut 3, &mut 4);
        assert!(*x == 3, 1);
        assert!(*y == 4, 1);
    }
}

//# run 0x42::M::bar
