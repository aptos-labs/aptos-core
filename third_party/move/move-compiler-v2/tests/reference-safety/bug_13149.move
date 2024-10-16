// TODO(#13149): after fix, rename file to reflect issue (`bug_nnn.move` to `bug_nnn_<issue>.move`)
module 0x815::m {
    fun t1() {
        let a = 0;
        let r1 = &mut a;
        let r2 = &mut a;
        *r2 = 2;
        *r1 = 1;
    }
}
