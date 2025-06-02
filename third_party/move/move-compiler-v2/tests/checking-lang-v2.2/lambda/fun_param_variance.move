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

module 0x66::yy {
    fun apply(f: |&mut u64|&u64) {
        f(&mut 0);
    }

    fun test() {
        let func: |&mut u64|&mut u64 = |x| { x };
        apply(func);
    }
}
