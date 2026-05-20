module 0xCAFE::Module0 {
    public fun f(x: vector<u8>, cond: bool): vector<u8> {
        let r = &x;
        if (cond) {
            abort x
        };
        *r
    }
}
