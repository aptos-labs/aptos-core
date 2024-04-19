module 0x42::m {
    friend 0x42::o;
    public inline fun foo(): u64 {
        bar()
    }

    public(friend) fun bar(): u64 { 42 }
}

module 0x42::o {
    use 0x42::m;

    public inline fun foo(): u64 {
        m::foo();
	bar()
    }

    fun bar(): u64 { 42 }
}

module 0x42::n {
    use 0x42::o;

    public fun test() {
        assert!(o::foo() == 42, 1);
    }
}
