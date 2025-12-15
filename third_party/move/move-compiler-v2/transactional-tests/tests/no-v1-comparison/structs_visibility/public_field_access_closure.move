//# publish
module 0xc0ffee::m {
    public struct Func1 {
        bar: ||u64
     } has copy, drop;


    public enum Func2 {
        V1 { bar: |u64|u64 },
        V2 { bar: |u64|u64, x: u64 },
     } has copy, drop;

    public struct Func3(||u64) has copy, drop;

}

//# publish
module 0xc0ffee::m2 {
    use 0xc0ffee::m::Func1;
    use 0xc0ffee::m::Func2;
    use 0xc0ffee::m::Func3;

    fun test1(): u64 {
        let f = Func1{ bar: || 42};
        (f.bar)()
    }


    fun test2(): u64 {
        let f1 = Func2::V1{ bar: |x| x};
        let f2 = Func2::V2{ bar: |x| x + 1, x: 44};
        (f1.bar)(42) + (f2.bar)(f2.x)
    }

    fun test3(): u64 {
        let f = Func3(|| 42);
        (f.0)()
    }
}

//# run 0xc0ffee::m2::test1

//# run 0xc0ffee::m2::test2

//# run 0xc0ffee::m2::test3
