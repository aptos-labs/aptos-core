module 0xCAFE::Module0 {
    struct S has copy, drop {f:u64}

    public fun function0() {
        let y: &S;
        *y;
    }
}
