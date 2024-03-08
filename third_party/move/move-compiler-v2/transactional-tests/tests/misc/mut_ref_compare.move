//# publish
module 0x42::m {
    fun test(): bool {
        let x = 1;
        let y = 1;
        let r1 = &mut x;
        let r2 = &mut y;
        r1 == r2
    }
}

//# run 0x42::m::test
