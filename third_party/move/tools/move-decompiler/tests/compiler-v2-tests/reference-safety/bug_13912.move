// TODO(#13912): after fix, rename file to reflect issue (`bug_nnn.move` to `bug_nnn_<issue>.move`)
module 0xCAFE::Module0 {
    // Expected to succeed without borrow errors
    public fun function2(var4: u8): bool {
        (&mut (var4) != &mut (copy var4))
    }
}
