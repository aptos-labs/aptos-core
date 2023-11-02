module 0xdeadbeef::M {
    friend 0xdeadbeef::N;
    use 0xdeadbeef::O as OO;
    use 0xdeadbeef::P;
    friend OO;
    friend P;
    fun foo(): u64 { 1 }

    public(friend) fun bar(): u64 { foo() }
    public(friend) fun id<T>(x: T): T { x  }
}

module 0xdeadbeef::N {
    fun foo(): u64 { 2 }

    fun calls_bar(): u64 {
        0xdeadbeef::M::bar() + 0xdeadbeef::M::id(foo())
    }
}

module 0xdeadbeef::O {
    use 0xdeadbeef::M as MM;
    use 0xdeadbeef::M::bar;
    use 0xdeadbeef::M::bar as mbar;

    fun foo(): u64 { 3 }

    fun calls_bar(): u64 {
        MM::bar() + MM::id(foo()) + mbar() + bar()
    }
}

module 0xdeadbeef::P {
    use 0xdeadbeef::M;
    fun my_foo(): u64 { 4 }

    fun calls_bar(): u64 {
        M::bar() + M::id(my_foo()) + 0xdeadbeef::M::bar()
    }
}

module 0xdeadbeef::Q {
    use 0xdeadbeef::M::{Self, bar};

    fun calls_bar(): u64 {
        M::id(5) + bar()
    }
}
