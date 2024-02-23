module 0x8675309::M {
    struct S { x: u64, y: u64 }

    fun f1(s: S) {
        s(&mut s.x, &mut s.x)
    }

    fun f2(s: S) {
        let r = &mut s.x;
        let x = r;
        s(r, x)
    }

    fun s(_x: &mut u64, _y: &mut u64){}
}
