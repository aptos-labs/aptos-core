module 0x42::m {

    struct S {
        x: u64,
    }

    struct R {
        y: u16,
    }

    fun assign_direct() {
        let x = 2u64;
        x = 1u8;
    }

    fun assign_unspecified_int() {
        let x = 2;
        x = vector[];
    }

    fun assign_unpack() {
        let y = 2;
        let s = S{x: 1};
        R{y} = s;
        R{y} = 1;
    }

    fun assign_tuple() {
        let x = 2;
        let y = 2;
        let s = S { x: 1 };
        (R{y}, x) = (s, y);
    }

}
