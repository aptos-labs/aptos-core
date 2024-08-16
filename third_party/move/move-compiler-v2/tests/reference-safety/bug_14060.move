// TODO(#14060): after fix, rename file to reflect issue (`bug_nnn.move` to `bug_nnn_<issue>.move`)
module 0xCAFE::Module0 {
    public fun f() {
        let x = &mut 0u8;
        let y = &mut 1u8;
        x != copy y;
    }
}
