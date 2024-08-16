// TODO(#13912): after fix, rename file to reflect issue (`bug_nnn.move` to `bug_nnn_<issue>.move`)
module 0xCAFE::Module1 {
    struct Struct3 has drop, copy {
        var32: u16,
        var33: u32,
        var34: u8,
        var35: u32,
        var36: u32,
    }

    public fun function6(): Struct3 {
        let var44: u16 =  21859u16;
        let var45: u32 =  1399722001u32;
        Struct3 {
            var32: var44,
            var33: var45,
            var34: 154u8,
            var35: var45,
            var36: var45,
        }
    }
}
