// Fixes ##16403
module 0x66::xx {
    inline fun apply(f: |&mut u64|) {
        let x = 0;
        f(&mut x);
    }

    fun test() {
        apply(|x: &u64| { *x + 1; });
    }

    struct S();

    inline fun apply_s(self: S, f: |&mut u64|) {
        let x = 0;
        f(&mut x);
    }

    fun test_s() {
        S().apply_s(|x: &u64| { *x + 1; });
    }


}
