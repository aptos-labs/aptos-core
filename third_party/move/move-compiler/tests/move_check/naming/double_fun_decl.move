module 0x42::M {

    fun bar(x: u64): u64 {
        x + 1
    }

    fun bar(): u64 {
        bar(1)
    }

    spec fun foo(x: u64): u64 {
        x + 1
    }

    fun foo() {
        foo(1)
    }
}
