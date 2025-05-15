module 0xc0ffee::m {
    // Test case from bug 16491.
    struct Func1(|&mut &mut u64|);

    struct Func2(|& &u64|);

    struct Func3(||(&mut &mut u64));

    struct Func4(|(|&mut &mut u64|)|);

    fun test1(f: |&mut &mut u64|) {}

    fun test2(): |&(&u64)|u64 {
        |x| **x + 1
    }

    fun test3() {
        let f: |& &u64|u64 = |x| {**x + 1};
    }

    fun test4(f: & &|(&u64)|) {}
}
