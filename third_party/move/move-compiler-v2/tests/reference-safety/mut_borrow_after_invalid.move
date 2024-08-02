module 0x42::m {
    struct S has copy, drop { f: u64, g: u64 }

    // bytecode verification fails
    fun t1(s: &mut S) {
        let s1 = s;
        let f = &mut s.f;
        *f;
        *s1;
    }

    // bytecode verification fails
    fun t3(p: u64): u64 {
        let a = &mut p;
        let b = a;
        *a = 0;
        *b
    }

    // bytecode verification fails
    fun t4(p: u64): u64 {
        let a = &mut p;
        let b = a;
        let c = b;
        *a = 0;
        *c
    }

    fun id_mut<T>(r: &mut T): &mut T {
        r
    }

    // bytecode verification fails
    fun t5() {
        let x = &mut 0;
        let y = id_mut(x);
        *y;
        *x;
    }
}
