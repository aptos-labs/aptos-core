module 0x99::bug_18335_args {
    struct S has drop {
        x: u64
    }

    fun receiver(self: &S, f: ||(u64, u64)): (u64, u64) {
        f()
    }

    fun test(x: u64) {
        let s = S { x: 42 };
        let func = |x, y| { (x, y) };
        s.receiver(func); // type error happens here, as `func` (of type `|u64|(u64, u64)`) does not match function type `|u64|`
    }
}

module 0x99::bug_18335_returns {
    struct S has drop {
        x: u64
    }

    fun receiver(self: &S, f: |u64|) {
        f(self.x);
    }

    fun test(x: u64) {
        let s = S { x: 42 };
        let func = |x| { (x, 1u64) };
        s.receiver(func); // type error happens here, as `func` (of type `|u64|(u64, u64)`) does not match function type `|u64|`
    }
}
