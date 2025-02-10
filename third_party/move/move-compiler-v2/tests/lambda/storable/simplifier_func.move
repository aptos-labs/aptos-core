module 0x42::mod1 {
    struct S {  // no drop
        x: u64
    }

    public fun triple(x: u64) : u64 {
        let _f = (|x: u64| { let _t = S { x: 3 }; x } : |u64|u64 has drop);
        x * 3
    }
}
