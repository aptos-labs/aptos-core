//# publish
module 0xc0ffee::m {
    use std::vector;

    fun run(x: &mut vector<u64>) {
        if (vector::length(x) > 0) {
            vector::push_back(x, 1);
        }
    }

    public fun test() {
        let v = vector::empty<u64>();
        run(&mut v);
        assert!(vector::length(&v) == 0, 1);
        vector::push_back(&mut v, 42);
        run(&mut v);
        assert!(vector::length(&v) == 2, 2);
    }
}

//# run 0xc0ffee::m::test
