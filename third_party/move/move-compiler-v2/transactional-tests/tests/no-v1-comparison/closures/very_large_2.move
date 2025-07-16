//# publish
module 0xCAFE::Module0 {
    public fun f1(
        var1: &(
            |
                (|
                    &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8,
                | has drop)
            | has drop),
    ) { }
    public fun f2() {
        f1(&(
            |
                var2: |
                    &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8, &u8,
                | has drop
            |
            { }
        ));
    }
}
