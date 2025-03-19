module 0xc0ffee::m {
    use std::option;

    fun test1() {
        let r = option::fold(option::some(1), 1, |a, b| a + b);
        assert!(r == 2, 0);
    }

    fun one(): u64 {
        1
    }

    fun test2(): u64 {
        let i = 2;
        let (a, b) = (i, one());
        a + b
    }
}
