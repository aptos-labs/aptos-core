module 0x42::M {
    public inline fun f(): u64 {
        42
    }
    spec f {
        aborts_if false;
        ensures result == 42;
    }
}
