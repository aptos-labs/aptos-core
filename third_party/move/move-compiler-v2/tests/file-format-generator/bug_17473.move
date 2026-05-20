module 0xCAFE::Module0 {
    public fun f1(_x: &bool, _y: bool) { }

    public fun f2(z: bool) {
        f1(
            &(z),
            if (z) true else true
        );
    }
}
