module 0xc0ffee::m {
    fun secret(): u64 {
        42
    }

    inline fun inner(): u64 {
        secret() + secret()
    }

    inline fun outer(): u64 {
        // private inline function calling private function
        // should be okay
        inner() + inner()
    }

    fun test() {
        assert!(outer() == 168);
    }
}

module 0xc0ffee::n {
    public fun non_secret(): u64 {
        42
    }

    inline fun inner(): u64 {
        non_secret() + non_secret()
    }

    public(friend) inline fun outer_friend(): u64 {
        // public(friend) inline function eventually calling public function
        // should be okay
        inner() + inner()
    }

    fun test() {
        assert!(outer_friend() == 168);
    }
}

module 0xc0ffee::o {
    public fun non_secret(): u64 {
        42
    }

    inline fun inner(): u64 {
        non_secret() + non_secret()
    }

    package inline fun outer_package(): u64 {
        // package inline function eventually calling public function
        // should be okay
        inner() + inner()
    }

    fun test() {
        assert!(outer_package() == 168);
    }
}
