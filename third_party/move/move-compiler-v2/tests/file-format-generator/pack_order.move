module 0x42::pack_unpack {

    struct S {
        f1: u8,
        f2: u8,
        f3: u8,
    }

    fun pack1(x: u8, y: u8, z: u8): S {
        S{f1: x, f2: y, f3: z}
    }

    fun pack2(x: u8, y: u8, z: u8): S {
        S{f1: x, f3: y, f2: z}
    }

    fun pack3(x: u8, y: u8, z: u8): S {
        S{f2: x, f1: y, f3: z}
    }

    fun pack4(x: u8, y: u8, z: u8): S {
        S{f2: x, f3: y, f1: z}
    }

    fun pack5(x: u8, y: u8, z: u8): S {
        S{f3: x, f1: y, f2: z}
    }

    fun pack6(x: u8, y: u8, z: u8): S {
        S{f3: x, f2: y, f1: z}
    }
}
