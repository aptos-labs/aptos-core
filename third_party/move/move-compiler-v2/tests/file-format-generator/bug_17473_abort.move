module 0xCAFE::Module0 {
    public fun f(x: u64, cond: bool): u64 {
        let r = &x;
        if (cond) {
            abort x
        };
        *r
    }
}
