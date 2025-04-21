module 0xc0ffee::m {
    #[lint::skip(while_true)]
    use std::vector;

    public fun test() {
        let _x: vector<u8> = vector::empty();
    }
}
