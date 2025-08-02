module 0x42::pack_unpack {

    struct S {
        f: u64,
        g: T
    }

    struct T {
        h: u64
    }


    fun pack(x: u64, y: u64): S {
        S{f: x, g: T{h: y}}
    }

    fun unpack(s: S): (u64, u64) {
        let S{f, g: T{h}} = s;
        (f, h)
    }
}
