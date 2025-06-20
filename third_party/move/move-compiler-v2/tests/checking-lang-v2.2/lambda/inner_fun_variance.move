// Fixes #16402
module 0x66::zz {

    public fun eval(_x: u64) {
        let a: |u64|u64 has drop = |x| x + 1;
        let b = vector[a];

        zzz(&b);
    }

    fun zzz(_a: &vector<|u64|u64>) {}
}

module 0x66::yy {
    fun apply(fv: vector<|&mut u64| has copy + drop>) {
        let x = 0;
        fv[0](&mut x);
    }

    fun test() {
        let func: |&u64| has copy + drop = |x| { *x + 1; };
        apply(vector[func]);
    }
}
