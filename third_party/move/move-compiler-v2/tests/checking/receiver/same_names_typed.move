module 0x42::a {

    struct MyList { len: u64 }

    public fun len(self: &MyList): u64 {
        self.len
    }
}

module 0x42::b {

    struct MyOtherList { len: u64 }

    public fun len(self: &MyOtherList): u64 {
        self.len
    }
}

module 0x42::c {
    use 0x42::a;
    use 0x42::b;

    inline fun foo(f: |a::MyList, b::MyOtherList|, x: a::MyList, y: b::MyOtherList) {
        f(x, y)
    }

    fun test(x: a::MyList, y: b::MyOtherList) {
        // In the lambda below, the type of x and y is not known when the
        // expression is checked.
        foo(|x: a::MyList, y: b::MyOtherList| { assert!(x.len() + y.len() == 1, 1) }, x, y)
    }
}
