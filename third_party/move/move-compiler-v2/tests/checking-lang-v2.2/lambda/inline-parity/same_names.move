module 0x42::a {

    struct MyList has drop { len: u64 }

    public fun len(self: &MyList): u64 {
        self.len
    }
}

module 0x42::b {

    struct MyOtherList has drop { len: u64 }

    public fun len(self: &MyOtherList): u64 {
        self.len
    }
}

module 0x42::c {
    use 0x42::a;
    use 0x42::b;

    fun foo(f: |a::MyList, b::MyOtherList|, x: a::MyList, y: b::MyOtherList) {
        f(x, y)
    }

    fun test(x: a::MyList, y: b::MyOtherList) {
        // In the lambda below, the type of x and y is not known when the
        // expression is checked.
        foo(|x, y| { assert!(x.len() + y.len() == 1, 1) }, x, y)
    }
}
