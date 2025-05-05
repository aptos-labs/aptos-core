module 0xc0ffee::m {
    struct Func1<phantom T>(|T|T) has copy, drop;
    struct Func2<phantom T>(|&T|&T) has copy, drop;
    struct Func3<phantom T>(|u64|(T, T)) has copy, drop;
    struct Func4(|u64|((u64, u64), u64)) has copy, drop;
}
