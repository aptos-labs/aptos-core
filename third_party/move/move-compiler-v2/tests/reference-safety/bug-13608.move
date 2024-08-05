module 0x8675309::M {

    fun t3(u1: &mut u64, u2: &mut u64): (&mut u64, &u64) {
        (u1, u2)
    }
    public fun bar2() {
        let x: &mut u64;
        let y: &u64;
        (x, y) = t3(&mut 3, &mut 4);
        x;
        y;
    }

}
