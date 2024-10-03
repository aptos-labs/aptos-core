module 0xCAFE::Module0 {
    struct S {}

    public fun f() {
        let _x = S {};
        //abort 1;
        //let S {} = _x;
    }
}
