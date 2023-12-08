module 0x42::m {

    fun f(x: u64): u64 {
        spec {
            assert 1;
        };
        x + 1
    }
    spec f {
        ensures result == x + true;
    }
}
