module 0xc0ffee::m {
    use std::vector;

    fun one(): u64 {
        1
    }

    fun consume(_a: u64, _b: &u64) {}

    public fun test(v: vector<u64>) {
        let x = one();
        vector::for_each_ref(&v, |e| {
            let e1 = e;
            consume(x, e1);
        });
    }
}
