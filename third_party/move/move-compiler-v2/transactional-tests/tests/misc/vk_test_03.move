//# publish --print-bytecode
module 0xc0ffee::m {
    public fun test(x: u64, z: u64) {
        let y = &mut x;
        *y = z;
    }

}
