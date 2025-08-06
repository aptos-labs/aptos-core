module 0x42::test {

    struct S(u64) has key;

    fun exec(f: |u64|u64, x: u64): u64 {
        f(x)
    }
    spec exec {
        // We cannot prove this because `f` can abort
        aborts_if false;

        // We can also not prove this because `S` can be modified by `f`
        ensures old(S[@0x2]) == S[@0x2];
    }
}
