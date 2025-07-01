//# publish
module 0xc0ffee::m {
    public struct NoCopy has drop;
}

//# publish
module 0xc0ffee::m_mut_ref_downgrade {

    public fun assn<T: drop>(ref: &mut T, x: T){
        *ref = x;
    }

    public fun foo() {
        use 0xc0ffee::m;

        let x = m::NoCopy;

        let a: ||u64 has drop = ||{
            let m::NoCopy = x;
            1
        };

        let b: ||u64 has drop + copy = ||1;

        assn<||u64 has drop>(&mut b, a);

        b();
        b();
    }
}

//# run 0xc0ffee::m_mut_ref_downgrade::foo
