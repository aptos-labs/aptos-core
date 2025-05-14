module 0xc0ffee::m1 {
    struct Func {
        bar: ||u64
     } has copy, drop;

    fun test(): u64 {
        let f = Func{ bar: || 42};
        f.bar()
    }
}

module 0xc0ffee::m2 {
    enum Func {
        V1 { bar: ||u64 },
        V2 { bar: ||u64, x: u64 },
     } has copy, drop;

    fun test(): u64 {
        let f = Func::V1{ bar: || 42};
        f.bar()
    }
}

module 0xc0ffee::m3 {
    enum Func {
        V1 { bar: ||u64 },
        V2 { bar: u64 }
     } has copy, drop;

    fun test(): u64 {
        let f = Func::V1{ bar: || 42};
        f.bar()
    }
}

module 0xc0ffee::m4 {
    struct Func(|u64|u64) has copy, drop;

    fun test(): u64 {
        let f = Func(|x| x + 1);
        f.0(1)
    }
}
