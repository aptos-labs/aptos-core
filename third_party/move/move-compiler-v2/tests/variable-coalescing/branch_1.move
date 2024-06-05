module 0xc0ffee::m {

    fun foo(b: bool, p: u64): u64 {
        let a = p;
        if (b) {
            a = 0; // kills copy `a := p`
        };
        a + 1 // should not have any copies available
    }
}
