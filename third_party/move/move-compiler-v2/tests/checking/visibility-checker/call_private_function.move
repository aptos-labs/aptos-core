module 0xdeadbeef::M {
    fun foo(): u64 { 1 }
}

module 0xdeadbeef::N {
    fun my_foo(): u64 { 2 }

    fun calls_foo(): u64 {
        0xdeadbeef::M::foo() + my_foo()
    }
}
