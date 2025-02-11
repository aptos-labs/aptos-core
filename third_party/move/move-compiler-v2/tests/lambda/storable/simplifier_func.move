module 0x42::mod1 {
    struct S has drop {
        x: u64
    }

    public fun triple(x: u64) : u64 {
        let _f : |u64|u64 has drop = |x: u64| { let _t = S { x: 3 }; x };
        x * 3
    }
}
