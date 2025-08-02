module 0x42::test {
    struct Coin(u256) has drop, copy;

    fun test0(x: &mut vector<Coin>, index: u64) {
        let _p = &mut x[index].0;
    }

    fun test1(x: vector<Coin>, index: u64) {
        let _p = &mut x[index].0;
    }

    fun test3(x: &vector<Coin>, index: u64) {
        let _p = x[index];
    }

    fun inc_vec_new(x: &mut vector<u256>, index: u64) {
        x[index] = x[index] + 1;
    }
}
