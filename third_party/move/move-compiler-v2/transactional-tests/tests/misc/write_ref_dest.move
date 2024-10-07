//# publish
module 0x42::m {

    fun foo(x: &vector<u64>): (vector<u64>, u64) {
        (*x, 0)
    }


    fun test_call() {
        let y = vector[];
        (y, _) = foo(&y);
        assert!(y == vector[], 0);
    }

    fun test_assign() {
        let y = vector[1];
        (y, _) = (*&y, 1);
        assert!(y == vector[1], 0);
    }

}

//# run 0x42::m::test_call

//# run 0x42::m::test_assign
