module 0xc0ffee::m {
    fun consume(a: u64) {
        a;
    }

    fun test(a: u64, p: bool) {
        let b = move a;
        if (p) {
            consume(b);
        } else {
            a = 99;
            let c = b;
            c = c + 1;
        }
    }

}
