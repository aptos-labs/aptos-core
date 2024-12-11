//# publish
module 0x42::Test {
    use std::vector;

    public inline fun foreach<X>(v: &vector<X>, action: |&X|) {
        let i = 0;
        while (i < vector::length(v)) {
            action(vector::borrow(v, i));
            i = i + 1;
        }
    }

    public fun test(): u64 {
        let v = vector[1u64, 2, 3];
        let sum = 0;
        foreach<u64>(&v, |e: &u64| sum = sum + *e);
        sum
    }

}

//# run 0x42::Test::test
