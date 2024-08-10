// TODO(#14096): after fix, rename file to reflect issue (`bug_nnn.move` to `bug_nnn_<issue>.move`)
module 0xCAFE::Module0 {
    // Expected to compile without errors
    public fun function2() {
        let x = &mut 0u8;
        (copy x == copy x);
    }
}
