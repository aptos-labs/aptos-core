module 0xCAFE::Module0 {
    struct Struct0 has drop, copy {
        x: bool,
    }

    public fun function5(var21: bool, var23: bool) {
        let _var67 =  (&(var21) != &((var21 || var23)));
        Struct0 {
            x: var21
        };
    }
}
