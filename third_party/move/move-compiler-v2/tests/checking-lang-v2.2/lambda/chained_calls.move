module 0x66::test {

    fun add_some(x: u64): |u64|u64 {
        |y| x + y
    }

    fun chain(v: vector<|u64|(|u64|u64)>): u64 {
        v[0](1)(2) + add_some(1)(2)
    }
}
