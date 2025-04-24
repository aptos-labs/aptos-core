// Fixes ##16403
module 0x66::xx {
    fun apply(f: |&mut u64|) {
        let x = 0;
        f(&mut x);
    }

    fun test() {
        let func = |x: &u64| { *x + 1; };
        apply(func);
    }
}
