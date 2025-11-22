module 0x8675::M {
    struct S has copy, drop {
        x: u64
    }

    fun test(data: S, a: u64, b: u64): u64 {
        let v1 = 1;
        let v2 = 1;

        let x = (a + b + v1) + (a + b + v2);

        if (data.x != 0) {
            x + a / data.x
        } else {
            x + data.x + 1
        }
    }
}
