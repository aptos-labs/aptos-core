//# publish
module 0x42::test {
    struct S has drop, copy {
        f: u64
    }

    public fun one_captured(x: u64): u64 {
        let f = |y| x + y;
        f(2)
    }

    public fun two_captured(x: u64, y: u8): u64 {
        let f = |z| x + (y as u64) + z;
        f(3)
    }

    public fun struct_captured(f: u64): u64 {
        struct_captured_helper(S { f })
    }

    fun struct_captured_helper(s: S): u64 {
        let f = |x| s.f + x;
        f(4)
    }
}

//# run 0x42::test::one_captured --args 3

//# run 0x42::test::two_captured --args 3 2u8

//# run 0x42::test::struct_captured --args 3
