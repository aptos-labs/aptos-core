module 0xc0ffee::m {
    struct Func(|(||)|||u64) has copy, drop;
    let _arg = || {};
    let f = Func(|_arg|||42);
    f()()
}
