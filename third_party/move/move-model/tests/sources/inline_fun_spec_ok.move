module 0x42::M {
    public inline fun f(): u64 {
        spec {
            assert 1 == 1;
        };
        42
    }
}
