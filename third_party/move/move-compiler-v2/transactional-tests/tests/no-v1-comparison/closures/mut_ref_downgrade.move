//# publish
module 0xc0ffee::m {
    struct NoCopy has drop;

    public fun assn<T: drop>(ref: &mut T, x: T){
        *ref = x;
    }

    public fun foo() {
        let x = NoCopy;

        let a: ||u64 has drop = ||{
            let NoCopy = x;
            1
        };

        let b: ||u64 has drop + copy = ||1;

        assn<||u64 has drop>(&mut b, a);

        b();
        b();
    }
}

//# run 0xc0ffee::m::foo
