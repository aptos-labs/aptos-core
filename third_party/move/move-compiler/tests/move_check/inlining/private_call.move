module 0x42::m {

    public inline fun foo(): u64 {
        bar()
    }

    fun bar(): u64 { 42 }
}

module 0x42::n {
    use 0x42::m;

    public fun test() {
        assert!(m::foo() == 42, 1);
    }
}
