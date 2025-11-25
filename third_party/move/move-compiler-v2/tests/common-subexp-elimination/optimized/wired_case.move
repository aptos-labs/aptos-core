module 0xc0ffee::m {
    use std::option;
    fun test(x: u64): (u64, option::Option<u64>, option::Option<u64>) {
        let y = x * x;
        (y, option::none(), option::none())
    }
}
