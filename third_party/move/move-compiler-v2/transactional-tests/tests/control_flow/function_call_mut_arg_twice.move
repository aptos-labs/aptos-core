//# publish
module 0xc0ffee::m {

fun update(r: &mut u64, x: u64, y: u64) {
    *r = x + y;
}

fun test(): u64 {
    let x = 0;
    let rx = &mut x;
    update(rx, 3, 4);
    update(rx, 5, 6);
    x
}

}

//# run 0xc0ffee::m::test
