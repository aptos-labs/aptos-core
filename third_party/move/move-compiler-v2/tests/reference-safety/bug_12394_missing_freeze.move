// TODO(#12394): after fix, rename file to reflect issue (`bug_nnn.move` to `bug_nnn_<issue>.move`)
module 0x815::m {
    fun t2(u1: &mut u64, u2: &mut u64): (&mut u64, &mut u64) {
        (u1, u2)
    }
    fun test_4() {
        let x: &u64;
        let y: &u64;
        (x, y) = t2(&mut 3, &mut 4);
    }
}
