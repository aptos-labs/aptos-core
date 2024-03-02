module 0x42::m {

    struct S {
        x: u64,
        y: u8
    }

    struct R {
        z: u16,
        s: S
    }

    fun bind_1(r: R, s: S) {
        let S{x, y} = r;
        let (S{x, y}, R{z, s}) = (r, s);
    }

    fun bind_2() {
        let (x, y) = (1, 2, 3);
    }
}
