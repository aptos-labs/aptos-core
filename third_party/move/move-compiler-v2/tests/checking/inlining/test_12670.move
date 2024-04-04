module 0x1::Test {
    use std::vector::for_each_ref;
    struct  S has drop {
        x :u8,
    }
    fun foo(xs : vector<S>) {
        let sum :u8 = 0;
        for_each_ref(&xs , | e | {   sum = sum + e.x;});
    }
}
