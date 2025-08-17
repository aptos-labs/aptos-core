module 0x815::m {

    enum Positional has drop {
        A(u8),
        B(u8),
    }
    fun test_common_access(): u8 {
        let x = Positional::A(42);
        x.0 = 19;
        20
    }

}
