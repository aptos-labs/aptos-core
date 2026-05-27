module 0xCAFE::Module0 {
    public fun f1(_a: &bool, _b: bool) {}
    public fun f2(x: bool) {
        f1(&x, x);
    }
}
