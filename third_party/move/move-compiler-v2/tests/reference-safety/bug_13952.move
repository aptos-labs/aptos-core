// TODO(#13952): after fix, rename file to reflect issue (`bug_nnn.move` to `bug_nnn_<issue>.move`)
module 0xCAFE::Module0 {
    struct Struct0 has drop, copy {
        x: bool,
    }

    public fun function5(var21: bool, var23: bool) {
        let var67 =  (&(var21) != &((var21 || var23)));
        Struct0 {
            x: var21
        };
    }
}
