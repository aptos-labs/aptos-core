//# publish
module 0xc0ffee::m {
    fun consume(a: u64) {
        a;
    }

    fun test(_a: u64, p: bool) {
        let b = move _a;
        if (p) {
            consume(b);
        } else {
            _a = 99;
            let _c = b;
            _c = _c + 1;
        }
    }

    public fun main() {
        test(42, true);
        test(42, false);
    }
}

//# run 0xc0ffee::m::main
