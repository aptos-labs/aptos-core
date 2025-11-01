module 0x8675::M {
    struct S has copy, drop {
        x: u64
    }

    fun test(data: S, a: u64, b: u64): u64 {
        a = a + 1;
        b = b * 2;
        a = a + b;
        if (data.x != 0) {
            a / data.x
             +
            b * data.x
        } else {
            b * data.x
        }
    }
}
