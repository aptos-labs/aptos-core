module 0x42::M {
    spec fun foo(x: u64): u64 { x + 1 }

    fun use_foo(): u64 {
        foo(1) // not visible
    }

    fun bar(x: u64): u64 {
        x + 1
    }

    fun use_bar(): u64 {
        bar(1)
    }
}
