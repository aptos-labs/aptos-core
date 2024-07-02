module 0x42::m {

    struct S has key, drop {

    }

    fun g(s: &S): &S {
        s
    }

    fun foo(x: &vector<u64>): (vector<u64>, u64) {
        (*x, 0)
    }

    fun test_call() {
        let _y = vector[1];
        let z = &_y;
        (_y, _) = foo(&_y);
        *z;
    }

    fun test_assign() {
        let _y = vector[1];
        let z = &_y;
        _y = vector[2];
        *z;
    }

}
