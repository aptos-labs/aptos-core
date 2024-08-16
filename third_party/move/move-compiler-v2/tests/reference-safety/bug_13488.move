// TODO(#13488): after fix, rename file to reflect issue (`bug_nnn.move` to `bug_nnn_<issue>.move`)
module 0x8675309::Tester {
    fun t() {
        let x = 0;
        let y = 0;
        let r1 = foo(&x, &y);
        let r2 = foo(&x, &y);
        x + copy x;
        y + copy y;
        r1;
        r2;
    }

    fun foo(_r: &u64, r2: &u64): &u64 {
        r2
    }
}
