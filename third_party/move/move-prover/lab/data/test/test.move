module 0x42::test {
    fun f1(x: u64): u64 {
        x + 1
    }
    spec f1 {
        ensures result == x + 1;
    }

    fun f2(x: u64, y: u64): u64 {
        x + y
    }
    spec f2 {
        ensures result == x + y;
    }
}
