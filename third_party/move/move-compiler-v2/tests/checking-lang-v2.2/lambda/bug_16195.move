module 0xc0ffee::m {
    struct Bug16195(|| ||u64) has copy, drop;

    struct Bug16195_variant1(||||u64) has copy, drop;

    struct Bug16195_variant2(||||||||||u64) has copy, drop;

    struct Bug16195_variant3(|(||)| ||u64) has copy, drop;

    struct Bug16195_variant4(|(||)|(||u64)) has copy, drop;

    struct Bug16195_variant5(|| ||bool) has copy, drop;

    struct Bug16195_variant6(||||||||||bool) has copy, drop;

    struct Bug16195_variant7(|| |bool|bool) has copy, drop;

    struct Bug16195_variant8(|bool| ||bool) has copy, drop;

    /// function that returns a function value
    public fun test_bug16195(): u64 {
        let f = Bug16195(|| ||42);
        f()()
    }

    /// function that returns a function value
    public fun test_bug16195_variant1(): u64 {
        let f = Bug16195_variant1(||||42);
        f()()
    }

    /// five-layers of nested function values
    public fun test_bug16195_variant2(): u64 {
        let f = Bug16195_variant2(||||||||||42);
        f()()()()()
    }

    /// function that returns a function value
    public fun test_bug16195_variant3(): u64 {
        let _arg = || {};
        let f = Bug16195_variant3(|_arg| ||42);
        f(_arg)()
    }

    /// function that returns a function value
    public fun test_bug16195_variant4(): u64 {
        let _arg = || {};
        let f = Bug16195_variant4(|_arg|(||42));
        f(_arg)()
    }

    /// regular OR between two operands
    public fun test_regular_OR_case1(a: u64, b: u64): bool {
        (a > 10) || (b > 20)
    }

    /// regular OR among three operands
    public fun test_regular_OR_case2(a: u64, b: u64, c: u64): bool {
        (a > 10) || (b > 20) || (c > 20)
    }

    /// mix of OR and a function that returns a function value.
    public fun test_bug16195_OR_mix1(a: bool): bool {
        let f = Bug16195_variant5(|| ||true);
        f ()() || a
    }

    /// mix of OR and a function that returns a function value.
    public fun test_bug16195_OR_mix2(a: bool, b: bool): bool {
        let f = Bug16195_variant5(|| ||true);
        f ()() || a || b
    }

    /// mix of OR and a five-layter nested function value
    public fun test_bug16195_OR_mix3(a: bool): bool {
        let f = Bug16195_variant6(||||||||||true);
        f ()()()()() || a
    }

     /// mix of OR and a five-layter nested function value
    public fun test_bug16195_OR_mix4(a: bool, b: bool): bool {
        let f = Bug16195_variant6(||||||||||true);
        f ()()()()() || a || b
    }

    /// mix of OR and a function that returns a function value. The second-layer function takes an argument
    public fun test_bug16195_OR_mix5(a: bool): bool {
        let f = Bug16195_variant7(|| |x| x || a);
        f ()(true) || a
    }

    /// mix of OR and a function that returns a function value. The first-layer function takes an argument
    public fun test_bug16195_OR_mix6(a: bool): bool {
        let f = Bug16195_variant8(|x| || x || true);
        f (true)() || a
    }
}
