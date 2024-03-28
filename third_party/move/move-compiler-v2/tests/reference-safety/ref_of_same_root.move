module 0x42::m {
    struct S has drop { x: u64, y: u64 }

    fun t0(s: S) {
        let x = &mut s.x;
        let y = &mut s.y;
        c(x, y)
    }

    fun t1(s: S) {
        c(&mut s.x, &mut s.y)
    }

    fun c(_x: &mut u64, _y: &mut u64) {}
}
